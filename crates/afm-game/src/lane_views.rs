//! Lane commands: views (docs/final-build/plan.md). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! The film room (later extras topics 2 and 3, screens topic 12): one of
//! your kept games played again from its seed with frames on, kept only
//! while you watch it, and handed to the 2D and 3D views a snap at a time.

mod film;
pub mod numbers;
mod venue;

use std::sync::Mutex;

use afm_engine::world::seat::FilmMark;
use serde_json::{Value, json};

use crate::{Game, Reply};

pub use film::Film;

/// The film being watched (one game at a time; freed when you leave).
static FILM: Mutex<Option<Film>> = Mutex::new(None);

fn game_index(args: &Value) -> Result<usize, String> {
    args.get("index").and_then(Value::as_u64).map(|v| v as usize).ok_or_else(|| "Missing index.".to_string())
}

/// The film of one of your games, played again if it isn't the one open.
fn with_film<T>(game: &Game, k: usize, f: impl FnOnce(&Film) -> Result<T, String>) -> Result<T, String> {
    let w = game.world.as_ref().ok_or("No world is loaded.")?;
    let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
    let key = (g.date.0 as u32, g.teams);
    let mut slot = FILM.lock().map_err(|_| "The film room is busy.".to_string())?;
    if slot.as_ref().is_none_or(|x| x.key != key) {
        *slot = None;
        *slot = Some(Film::open(w, g)?);
    }
    f(slot.as_ref().unwrap())
}

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        // The film's index: teams, venue, players, every play and its tags.
        "film" => (|| {
            let k = game_index(args)?;
            let w = game.world.as_ref().ok_or("No world is loaded.")?;
            let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
            with_film(game, k, |f| Ok(f.index(w, g, k)))
        })(),
        // One snap's frames, in the stadium's frame.
        "film_play" => (|| {
            let k = game_index(args)?;
            let n = args.get("n").and_then(Value::as_u64).ok_or("Missing play.")? as u32;
            with_film(game, k, |f| f.play(n))
        })(),
        // Several snaps at once (a cut-up loads ahead).
        "film_plays" => (|| {
            let k = game_index(args)?;
            let ns: Vec<u32> =
                args.get("n").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).map(|v| v as u32).collect()).unwrap_or_default();
            with_film(game, k, |f| Ok(Value::Array(ns.iter().filter_map(|&n| f.play(n).ok()).collect())))
        })(),
        // Your drawings on the film, saved with the game.
        "film_marks" => (|| {
            let k = game_index(args)?;
            let marks: Vec<FilmMark> = args
                .get("marks")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|m| {
                            Some(FilmMark {
                                play: m.get("play")?.as_u64()? as u32,
                                t: m.get("t")?.as_f64()? as f32,
                                kind: m.get("kind")?.as_u64()?.min(3) as u8,
                                color: m.get("color")?.as_u64()?.min(3) as u8,
                                pts: m
                                    .get("pts")?
                                    .as_array()?
                                    .iter()
                                    .filter_map(|p| Some((p.get(0)?.as_f64()? as f32, p.get(1)?.as_f64()? as f32)))
                                    .take(400)
                                    .collect(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let w = game.world.as_mut().ok_or("No world is loaded.")?;
            if !w.set_film_marks(k, marks) {
                return Err("That game is not kept.".into());
            }
            Ok(json!({ "saved": w.seat_games()[k].marks.len() }))
        })(),
        // Leaving the film room frees the film.
        "film_close" => {
            if let Ok(mut s) = FILM.lock() {
                *s = None;
            }
            Ok(json!({ "closed": true }))
        }
        _ => return None,
    })
}
