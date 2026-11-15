//! A decision you told a player in a talk (5.1: "the game runs the real
//! action once you confirm"), made real by the room's own rules: starting
//! puts him first at his position, benched puts him behind the men who
//! start there, and suspended sits him for your next game. Your order goes
//! through `propose_depth`, so in an assistant's seat the head coach can
//! still overrule it. The rest (cut, traded, an offer pulled, a coach let
//! go, a move or a redshirt) belong to their own lanes and screens.

use super::super::Them;
use super::super::terms::Decision;
use crate::game::roster::starters;
use crate::world::World;
use crate::world::folk::{self as folk, Team, rules};

/// Runs a decision on your player, or None when it isn't this lane's.
pub fn run(w: &mut World, him: Them, d: Decision) -> Option<Vec<String>> {
    let Them::Player(id) = him else { return None };
    match d {
        Decision::Starting | Decision::Benched => Some(chart(w, id, d == Decision::Starting)),
        Decision::Suspended => Some(suspend(w, id)),
        _ => None,
    }
}

fn chart(w: &mut World, id: u32, start: bool) -> Vec<String> {
    let Some(i) = folk::person(w, id) else { return vec!["He is no longer on the team.".into()] };
    let pos = w.people[i].player.position;
    let name = w.people[i].name.clone();
    let mut order: Vec<u32> =
        w.seat_game_depth().and_then(|d| d.into_iter().find(|(p, _)| *p == pos).map(|(_, v)| v)).unwrap_or_default();
    if !order.contains(&id) {
        order.push(id);
    }
    let n = starters(pos).max(1);
    let at = order.iter().position(|&x| x == id).unwrap_or(0);
    if start && at == 0 {
        return vec![format!("{name} is already first at {}.", pos.name())];
    }
    if !start && at >= n {
        return vec![format!("{name} is already behind the starters at {}.", pos.name())];
    }
    order.retain(|&x| x != id);
    let to = if start { 0 } else { n.min(order.len()) };
    order.insert(to, id);
    match w.propose_depth(pos, order) {
        Ok(None) => vec![if start {
            format!("{name} is first at {} on your depth chart.", pos.name())
        } else {
            format!("{name} is behind the starters at {} on your depth chart.", pos.name())
        }],
        Ok(Some(note)) => vec![note],
        Err(e) => vec![e],
    }
}

fn suspend(w: &mut World, id: u32) -> Vec<String> {
    let Some(i) = folk::person(w, id) else { return vec!["He is no longer on the team.".into()] };
    let Some(team) = Team::seat(w) else { return vec!["You have no team.".into()] };
    let until = rules::after_games(w, team, 1);
    let name = w.people[i].name.clone();
    if rules::sitting(w, id, until) {
        return vec![format!("{name} already sits your next game.")];
    }
    w.folk.out.push((id, until, "suspended for a game by you".into()));
    vec![format!("{name} sits your next game.")]
}
