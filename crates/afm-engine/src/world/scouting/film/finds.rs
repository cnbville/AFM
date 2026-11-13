//! Found on another kid's tape (SK34): watching one player's film now and
//! then turns up a teammate or an opponent nobody is recruiting. In 2002
//! Cal's Jeff Tedford was watching a Butte College tight end when the
//! quarterback "popped off the tape": Aaron Rodgers.
//!
//! A kid pops when he plays as well on the snaps you watched as the man
//! you came to see and the public has him low (two stars or fewer, or off
//! every board); your eye and the snaps you watched size the chance, and
//! more film is simply more chances (SK63). He goes on My players with
//! the tape as the reason. Rates are Claude's, proposed, not confirmed.
//!
//! You watched him play, so you have a read of him (minor bug 379, Cain
//! 2026-11-12: he came with none, "0 idea of the attributes"; "keep it
//! 75"): three quarters of what the watch gave you of the man you came to
//! see (`read`).

use super::Catch;
use crate::attributes::{ATTRIBUTE_COUNT, Attr};
use crate::calendar::Date;
use crate::positions::{overall_for, weight};
use crate::rng::clamp;
use crate::world::scouting::{Kind, file, readers, yours};
use crate::world::season::{key_normal, key_uniform};
use crate::world::{Stage, World};

/// A kid found on another's tape.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Find {
    pub id: u32,
    /// Whose tape he was on.
    pub on: u32,
    pub date: Date,
}

/// What a watch gave you of the man you came to see: it sizes your read of
/// a kid who pops off it (`read`).
pub struct Watch<'a> {
    /// The catches from its snaps (yours and your analyst's).
    pub caught: &'a [Catch],
    /// His games in the snaps you watched: on tape, each is a film look of
    /// him (`yours::cut_ups`).
    pub games: usize,
    /// From the stands: his look is the trip's, in person, not tape.
    pub live: bool,
}

/// Maybe a kid pops off the tape of `on` (the snaps' other players).
/// `gave`: what the watch gave you of `on`, which sizes your read of the
/// kid (`read`).
pub fn pop(w: &mut World, on: u32, others: &[u32], watched: usize, eye: f64, gave: &Watch) -> Option<String> {
    if others.is_empty() || watched == 0 {
        return None;
    }
    let i = super::super::place(w, on)?;
    let base = {
        let p = &w.people[i];
        overall_for(p.player.position, &p.player.attributes)
    };
    let stage = w.people[i].stage;
    let followed: Vec<u32> = w.scouting.dept.my_players.iter().map(|f| f.id).collect();
    let mut cands: Vec<(f64, u32, usize)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &id in others {
        if id == on || !seen.insert(id) || followed.contains(&id) || w.scouting.film.finds.iter().any(|f| f.id == id) {
            continue;
        }
        let Some(j) = super::super::place(w, id) else { continue };
        let p = &w.people[j];
        let low = match p.stage {
            Stage::HighSchool | Stage::PrepSchool => p.stars <= 2,
            Stage::College | Stage::Declared => !w.scouting.board.order.contains(&id) && p.stars <= 2,
            _ => false,
        };
        if !low || p.stage != stage && !matches!((p.stage, stage), (Stage::PrepSchool, Stage::HighSchool)) {
            continue;
        }
        let ovr = overall_for(p.player.position, &p.player.attributes);
        if ovr + 3.0 >= base {
            cands.push((ovr, id, j));
        }
    }
    cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let &(_, id, j) = cands.first()?;
    let d = w.today;
    let p_pop = clamp(0.10 * watched as f64 / 6.0 * (1.0 + 0.05 * (eye - 10.0)), 0.01, 0.3);
    if key_uniform(w.options.seed as u64 ^ 0x9095 ^ (d.0 as u64) << 20, id as u64 ^ (on as u64) << 32) >= p_pop {
        return None;
    }
    let (name, pos, stars) = (w.people[j].name.clone(), w.people[j].player.position, w.people[j].stars);
    let on_name = w.people[i].name.clone();
    let public = if stars > 0 { format!("{stars}-star") } else { "Unranked".to_string() };
    w.scouting.dept.my_players.retain(|f| f.id != id);
    w.scouting.dept.my_players.push(crate::world::scouting::dept::Followed {
        id,
        since: d,
        why: format!("Popped off {on_name}'s tape"),
        first_read: "On tape".into(),
        public_then: public,
    });
    if let Some(car) = w.career.as_mut() {
        car.desk.unlisted.retain(|&u| u != id);
        if !car.desk.is_listed(id) {
            car.desk.listed.push(crate::world::desk::Listed { id, since: d, read: (0, 0), stars, how: 3 });
        }
    }
    w.scouting.film.finds.push(Find { id, on, date: d });
    if w.scouting.film.finds.len() > 300 {
        w.scouting.film.finds.remove(0);
    }
    read(w, id, on, gave);
    Some(format!(
        "{name} ({}) popped off the tape: nobody is recruiting him, and he played like the man you came to see. He's on My players, with a first read of him from these snaps.",
        pos.name()
    ))
}

/// Your read of a kid who popped off the tape of `on` (minor bug 379): a
/// flat three quarters (`file::SIDE_SHARE`, Cain 2026-11-12) of what the
/// watch gave you of the man you came to see, whatever the kid's own part
/// in the snaps. A watch gives two things, and he gets both:
///
/// - A film look in his file under your name. His is a look at another
///   man's tape (`Kind::SideFilm`): the file reads every rating film shows
///   of him three quarters of the way in from the wide guess to a film
///   look's read (`file::read_in`), for your staff's read and your own.
///   From the stands too, where the man you came to see got the trip's
///   look in person: the kid's reads as tape either way (proposed, not
///   confirmed).
/// - What your eye caught snap by snap. Each rating it caught of the man
///   you watched gives the kid one catch on a rating of his own, the
///   ratings that weigh most at his position first and the tightest read
///   first. The catch is sized so that your own read of the kid's rating
///   lands three quarters of the way in from his wide guess to your read
///   of the other man's rating, each from this watch alone (the look and
///   the catches, joined as `sharpen_with` joins them; from the stands the
///   other man's look is the trip's, in person). It is kept as one
///   catch of your eye, so it carries your eye's lasting misjudgment of
///   him, ages as tape does and joins his own tape when you watch it. Your
///   analyst charted the other man: none of his points come across.
///
/// Which ratings take the catches is Claude's, proposed, not confirmed.
fn read(w: &mut World, id: u32, on: u32, watch: &Watch) {
    let (Some(you), Some(staff)) = (readers::you(w), readers::seat_staff(w)) else { return };
    let (Some(j), Some(i)) = (super::super::place(w, id), super::super::place(w, on)) else { return };
    let d = w.today;
    super::super::dept::add(w, id, readers::COACH + you as u32, Kind::SideFilm, d);
    // Your own read of a rating from one watch: the wide guess joined with
    // your eye's read, which is its misjudgment and what the tape and the
    // catches pinned down (`sharpen_with`).
    let whole =
        |wide: f64, lean: f64, pinned: f64| (1.0 / (1.0 / (wide * wide) + 1.0 / (lean * lean + 1.0 / pinned))).sqrt();
    let pin = |sd: Option<f64>| sd.map_or(0.0, |sd| 1.0 / (sd * sd).max(0.01));
    // The man you watched: each rating your eye caught, from this watch
    // alone, the tightest first. On tape each game is a film look of him,
    // which your catches join. From the stands his look is the trip's, in
    // person: it stands where the wide guess does, apart from your eye.
    let wide_on = file::GUESS_SD * file::visibility(w, i);
    let eye_on = super::session::lasting_eye(w, w.people[i].player.position);
    let kind_on = if watch.live { Kind::Game } else { super::session::film_kind(w, i) };
    let look_on = yours::tape_sd(w, staff, on, kind_on, watch.games);
    let mut eyed: Vec<&Catch> =
        watch.caught.iter().filter(|c| c.id == on && c.by == 0 && (c.attr as usize) < ATTRIBUTE_COUNT).collect();
    eyed.sort_by_key(|c| (c.attr, c.game));
    let mut gave: Vec<f64> = eyed
        .chunk_by(|a, b| a.attr == b.attr)
        .filter_map(|cs| {
            let k = cs[0].attr as usize;
            let seen: f64 = super::looks_of(cs, None).iter().map(|g| 1.0 / g.var).sum();
            let lean = super::lean_sd(eye_on, Attr::ALL[k]);
            (seen > 0.0).then(|| match watch.live {
                true => whole(look_on[k].unwrap_or(wide_on), lean, seen),
                false => whole(wide_on, lean, pin(look_on[k]) + seen),
            })
        })
        .collect();
    gave.sort_by(f64::total_cmp);
    // The kid's ratings his look shows, the ones his position leans on
    // first.
    let p = &w.people[j];
    let pos = p.player.position;
    let tape = yours::tape_sd(w, staff, id, Kind::SideFilm, 1);
    let mut his: Vec<Attr> =
        Attr::ALL.iter().copied().filter(|&a| weight(pos, a) > 0.0 && tape[a.index()].is_some()).collect();
    his.sort_by(|a, b| weight(pos, *b).total_cmp(&weight(pos, *a)).then(a.index().cmp(&b.index())));
    let wide = file::GUESS_SD * file::visibility(w, j);
    let steady = super::session::lasting_eye(w, pos);
    let seed = w.options.seed as u64 ^ 0x51DE ^ (d.0 as u64) << 20;
    let new: Vec<Catch> = his
        .into_iter()
        .zip(gave)
        .filter_map(|(a, theirs)| {
            // Where your read of his rating lands, and what the catch has
            // to pin down to take it there past his look and your eye's
            // misjudgment of him (at least half a point is left to it).
            let to = file::side(theirs, wide);
            if to >= wide {
                return None;
            }
            let lean = super::lean_sd(steady, a);
            let eye = 1.0 / (1.0 / (to * to) - 1.0 / (wide * wide));
            let more = 1.0 / (eye - lean * lean).max(0.25) - pin(tape[a.index()]);
            if more <= 0.0 {
                return None;
            }
            let rest = (1.0 / more).sqrt();
            let noise = key_normal(seed, id as u64 ^ (a.index() as u64) << 50);
            let v = p.player.attributes.get(a) + noise * rest + lean * super::eye_lean(w, id, a);
            Some(Catch {
                id,
                attr: a.index() as u8,
                v: v as f32,
                sd: rest as f32,
                date: d,
                by: 0,
                key: 0,
                lean: lean as f32,
                game: 0,
                form: 0.0,
            })
        })
        .collect();
    super::add_catches(&mut w.scouting.film.catches, new);
}
