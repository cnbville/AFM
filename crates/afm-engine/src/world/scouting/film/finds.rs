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

use crate::calendar::Date;
use crate::positions::overall_for;
use crate::rng::clamp;
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// A kid found on another's tape.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Find {
    pub id: u32,
    /// Whose tape he was on.
    pub on: u32,
    pub date: Date,
}

/// Maybe a kid pops off the tape of `on` (the snaps' other players).
pub fn pop(w: &mut World, on: u32, others: &[u32], watched: usize, eye: f64) -> Option<String> {
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
    Some(format!(
        "{name} ({}) popped off the tape: nobody is recruiting him, and he played like the man you came to see. He's on My players.",
        pos.name()
    ))
}
