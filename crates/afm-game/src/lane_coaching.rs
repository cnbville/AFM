//! Update 3 lane 5, coaching people: the tips your contacts send (SK35,
//! C14) and placing a player you can't keep with a friend (C16). The
//! Contacts screen itself is the people lane's (`lane_people.rs`), which
//! carries the tips and every kind of contact from this lane on.

use afm_engine::player::PlayingTime;
use afm_engine::world::Stage;
use afm_engine::world::World;
use afm_engine::world::folk::network::Other;
use afm_engine::world::folk::{self as folk, Team, backchannel, facts, tips};
use serde_json::{Value, json};

use crate::views::date_text;
use crate::{Game, Reply};

pub fn call(g: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let r = match cmd {
        "l5_place_options" => g.world.as_ref().map(place_options).ok_or_else(|| "No world is loaded.".to_string()),
        "l5_place" => (|| {
            let w = g.world.as_mut().ok_or("No world is loaded.")?;
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
            let coach = args.get("coach").and_then(Value::as_u64).ok_or("Missing coach.")? as usize;
            let text = backchannel::place_player(w, id, coach)?;
            Ok(json!({ "text": text, "options": place_options(w) }))
        })(),
        _ => return None,
    };
    Some(r)
}

/// Who sent a tip, as the Contacts screen names him.
fn sender(w: &World, coach: usize, other: Option<Other>) -> String {
    match other {
        None if coach < w.coaches.len() => folk::coach_link(w, coach),
        Some(Other::Former(id)) => w.formers.get(id).map_or_else(|| "A former player".into(), |f| f.name.clone()),
        Some(Other::Agent(a)) => w.market.agents.get(a).map_or_else(|| "An agent".into(), |x| x.name.clone()),
        _ => "Someone you know".into(),
    }
}

/// The tips you've had, newest first (SK35): what he said, his camp's
/// lines (C14) and, once the kid's road shows, whether it held up.
pub(crate) fn tips(w: &World) -> Value {
    let rows: Vec<Value> = w
        .folk
        .tips
        .list
        .iter()
        .rev()
        .take(40)
        .map(|t| {
            let player = folk::person(w, t.player)
                .map_or_else(|| "a player no longer in the game".into(), |i| folk::player_link(w, i));
            json!({
                "date": date_text(t.date),
                "from": sender(w, t.coach, t.other),
                "player": player,
                "claim": tips::CLAIMS[t.claim.min(2) as usize],
                "lines": t.lines,
                "status": match t.settled {
                    Some(true) => "Held up",
                    Some(false) => "Missed",
                    None => "Too soon to tell",
                },
            })
        })
        .collect();
    json!(rows)
}

/// Your players you could place, and the friends who could take them
/// (C16): at a college, the players who don't start, with friends at
/// smaller programs; in the NFL, near cutdown day, your roster, with
/// friends at other clubs.
fn place_options(w: &World) -> Value {
    let seat = Team::seat(w);
    let mut players: Vec<Value> = Vec::new();
    let mut friends: Vec<Value> = Vec::new();
    let note: String = match seat {
        Some(Team::College(j)) => {
            let div = w.programs[j].division as usize;
            for &i in &w.programs[j].roster {
                let p = &w.people[i];
                if p.stage == Stage::College && p.college == Some(j) && p.last_role != Some(PlayingTime::Starter) {
                    players.push(
                        json!({ "id": p.player.id, "link": folk::player_link(w, i), "pos": p.player.position.name() }),
                    );
                }
            }
            for x in w.folk.network.contacts.iter().filter(|x| x.warmth >= 0.2) {
                if let Some(c) = x.coach_id()
                    && let Some(Team::College(q)) = facts::team_of_coach(w, c)
                    && (w.programs[q].division as usize) > div
                {
                    friends.push(
                        json!({ "coach": c, "link": folk::coach_link(w, c), "at": w.program_name(q), "sent": x.sent }),
                    );
                }
            }
            "He still chooses: he enters the next window with your word, and your friend wants him.".into()
        }
        Some(Team::Nfl(t)) => {
            let cut = w.dates.cutdown.date;
            if w.today >= cut.add(-10) && w.today <= cut {
                for (i, p) in w.people.iter().enumerate() {
                    if p.pro.as_ref().is_some_and(|c| c.team == Some(t)) {
                        players.push(json!({ "id": p.player.id, "link": folk::player_link(w, i), "pos": p.player.position.name() }));
                    }
                }
                for x in w.folk.network.contacts.iter().filter(|x| x.warmth >= 0.2) {
                    if let Some(c) = x.coach_id()
                        && let Some(Team::Nfl(o)) = facts::team_of_coach(w, c)
                        && o != t
                    {
                        friends.push(
                            json!({ "coach": c, "link": folk::coach_link(w, c), "at": w.nfl[o].name, "sent": x.sent }),
                        );
                    }
                }
                "A late-round pick swap instead of a release.".into()
            } else {
                format!("In the NFL it's a cutdown-day deal: from {}.", date_text(cut.add(-10)))
            }
        }
        _ => "A college or NFL job places players with friends.".into(),
    };
    json!({ "players": players, "friends": friends, "note": note })
}
