//! Ground's favors (lane G11b): the commands a favor board reads and the
//! calls it makes. The layer ships switched off (GR61): in a world without
//! it the board says so and every call refuses. Everything comes back in
//! words, never numbers (GR63).

use afm_engine::world::World;
use afm_engine::world::ground::favors::{self, Worry};
use serde_json::{Value, json};

use crate::lane_ground_people::{key, who};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "ground_favors" => game.w().and_then(board),
        "ground_favor_takers" => game.w().and_then(|w| takers(w, args)),
        "ground_favor_place" => game.wm().and_then(|w| place(w, args)),
        "ground_favor_ask" => game.wm().and_then(|w| ask(w, args)),
        "ground_favor_rule" => game.wm().and_then(|w| rule(w, args)),
        _ => return None,
    })
}

fn kid(args: &Value) -> Result<u32, String> {
    Ok(args.get("kid").and_then(Value::as_u64).ok_or("Which kid?")? as u32)
}

fn board(w: &World) -> Reply {
    let b = favors::board(w);
    let mut v = serde_json::to_value(&b).map_err(|e| e.to_string())?;
    for side in ["owed", "owe"] {
        if let Some(rows) = v.get_mut(side).and_then(Value::as_array_mut) {
            let src = if side == "owed" { &b.owed } else { &b.owe };
            for (x, line) in rows.iter_mut().zip(src) {
                x["key"] = key(line.who);
            }
        }
    }
    Ok(v)
}

/// The programs with a real hole for a kid, with the head coach to call.
fn takers(w: &World, args: &Value) -> Reply {
    if !favors::on(w) {
        return Err("Favors are switched off in this world.".into());
    }
    let id = kid(args)?;
    let rows: Vec<Value> = favors::takers(w, id, 12)
        .into_iter()
        .filter_map(|j| {
            let p = &w.programs[j];
            let c = p.head_coach?;
            Some(json!({
                "program": j,
                "name": w.program_name(j),
                "division": format!("{:?}", p.division),
                "coach": c,
                "coach_name": w.coaches[c].name,
            }))
        })
        .collect();
    Ok(json!({ "kid": id, "takers": rows }))
}

fn place(w: &mut World, args: &Value) -> Reply {
    let id = kid(args)?;
    let coach = args.get("coach").and_then(Value::as_u64).ok_or("Which coach?")? as usize;
    let claim = args.get("claim").and_then(Value::as_u64).unwrap_or(1).min(2) as u8;
    let worry = match args.get("worry").and_then(Value::as_str) {
        None | Some("") => None,
        Some(s) => Some(Worry::from(s).ok_or("No such worry.")?),
    };
    let said = favors::place(w, id, coach, claim, worry, None)?;
    let mut v = board(w)?;
    v["said"] = json!(said);
    Ok(v)
}

fn ask(w: &mut World, args: &Value) -> Reply {
    let person = who(args)?;
    let k = args.get("kid").and_then(Value::as_u64).map(|x| x as u32);
    let said = favors::ask(w, person, k)?;
    let mut v = board(w)?;
    v["said"] = json!(said);
    Ok(v)
}

fn rule(w: &mut World, args: &Value) -> Reply {
    let calls = args.get("calls").and_then(Value::as_u64).ok_or("How many calls?")?.min(3) as u8;
    let said = favors::set_rule(w, calls)?;
    let mut v = board(w)?;
    v["said"] = json!(said);
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::world::WorldOptions;
    use afm_engine::world::data::Division;
    use afm_engine::world::seat::{Job, You};

    fn you() -> You {
        let teaching = vec![12.0; afm_engine::world::coach::GROUP_COUNT];
        serde_json::from_value(json!({
            "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
            "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
            "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
            "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
            "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
        }))
        .unwrap()
    }

    /// A game on the tiny world with your career at a college, favors on or off.
    fn game(name: &str, favors: bool) -> Game {
        let home = std::env::temp_dir().join(format!("afm-favors-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let mut g = Game::new(home);
        let mut w = World::empty(WorldOptions {
            scale: 0.001,
            history_years: 0,
            start_year: 2037,
            favors,
            ..Default::default()
        });
        let job = Job::College(w.programs.iter().position(|p| p.division == Division::FBS).unwrap());
        w.start_career(&you(), job, 7);
        g.world = Some(w);
        g
    }

    #[test]
    fn the_favor_board_is_off_in_every_world_and_every_call_refuses() {
        let mut g = game("off", false);
        let b = g.call("ground_favors", &json!({})).unwrap();
        assert_eq!(b["on"], json!(false));
        assert!(g.call("ground_favor_place", &json!({ "kid": 1, "coach": 0 })).is_err());
        assert!(g.call("ground_favor_ask", &json!({ "kind": "coach", "id": 0, "kid": 1 })).is_err());
        assert!(g.call("ground_favor_rule", &json!({ "calls": 2 })).is_err());
        assert!(g.call("ground_favor_takers", &json!({ "kid": 1 })).is_err());
    }

    #[test]
    fn with_the_switch_on_the_board_speaks_in_words() {
        let mut g = game("on", true);
        let b = g.call("ground_favors", &json!({})).unwrap();
        assert_eq!(b["on"], json!(true));
        let r = g.call("ground_favor_rule", &json!({ "calls": 2 })).unwrap();
        assert!(r["said"].as_str().unwrap().contains("2 calls"));
        assert!(r["rule"].as_str().unwrap().contains("Trusted and Home"));
        assert!(g.call("ground_favor_place", &json!({ "kid": 1, "coach": 0, "worry": "luck" })).is_err());
        for side in ["owed", "owe"] {
            for x in r[side].as_array().unwrap() {
                let words = x["words"].as_str().unwrap();
                assert!(!words.chars().any(|c| c.is_ascii_digit()), "favors in words, never numbers: {words}");
            }
        }
    }
}
