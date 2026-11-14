//! The player and roster boards (update 2, lane 9A) on a small world, at a
//! college, an NFL and a high school seat: every board answers, every
//! rating in them is your staff's range (never one number, never a
//! ceiling), the needs map's starters and the career arc's reads are the
//! roster's, a backup moved up on the stadium board is the order the
//! staff plays, and the reads kept at a season's end survive a save while
//! a save from before them still loads.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// No hidden number anywhere in an answer: no potential or ceiling keys,
/// and every read a range at least two wide, or one wide where it touches
/// the 1-99 scale's end (`reads::range` clamps, so a 99 can read 98-99).
fn only_ranges(cmd: &str, v: &Value, path: &str) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                let p = format!("{path}.{k}");
                assert!(
                    !matches!(k.as_str(), "potential" | "ceiling" | "true" | "truth" | "pot" | "overall_true"),
                    "{cmd}{p}: a hidden number"
                );
                if (k == "read" || k == "ovr")
                    && let Some(a) = x.as_array()
                {
                    assert_eq!(a.len(), 2, "{cmd}{p}: {x}");
                    let (lo, hi) = (a[0].as_f64().unwrap(), a[1].as_f64().unwrap());
                    let least = if lo <= 1.0 || hi >= 99.0 { 1.0 } else { 2.0 };
                    assert!(hi - lo >= least, "{cmd}{p}: {x} is too sure to be a staff's read");
                }
                only_ranges(cmd, x, &p);
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                only_ranges(cmd, x, &format!("{path}[{i}]"));
            }
        }
        _ => {}
    }
}

fn board(g: &mut Game, cmd: &str, args: Value) -> Value {
    let v = ok(g, cmd, args);
    only_ranges(cmd, &v, "");
    v
}

/// Every board at the seat you hold; returns the roster's rows.
fn every_board(g: &mut Game, level: &str) -> Vec<Value> {
    let d = board(g, "sb_depth", json!({}));
    let tabs = d["tabs"].as_array().unwrap().clone();
    assert_eq!(tabs.len(), 4, "{level}: offense, defense, special teams, returners");
    for t in &tabs {
        for f in t["forms"].as_array().unwrap() {
            let v = board(g, "sb_depth", json!({ "tab": t["key"], "form": f["key"] }));
            assert_eq!(v["form"], f["key"], "{level}");
            for s in v["slots"].as_array().unwrap() {
                assert_eq!(s["picks"].as_array().map(Vec::len), Some(3), "{level}: three deep at {}", s["slot"]);
            }
        }
    }
    let off = board(g, "sb_depth", json!({ "tab": "offense", "form": "offense" }));
    assert_eq!(off["slots"].as_array().map(Vec::len), Some(11), "{level}: eleven on the field");
    assert!(
        off["slots"].as_array().unwrap().iter().all(|s| s["picks"][0].is_object()),
        "{level}: a starter at every spot"
    );
    assert!(off["units"].as_array().is_some_and(|u| u.len() == 6), "{level}: {}", off["units"]);

    let roster = board(g, "sb_roster", json!({}));
    let rows = roster["rows"].as_array().unwrap().clone();
    assert!(!rows.is_empty(), "{level}: a roster");
    quick_filter(level, &roster);
    quick_filter(level, &ok(g, "roster", json!({})));
    let ids: Vec<u64> = rows.iter().filter_map(|r| r["id"].as_u64()).collect();

    // The needs map's starters now are on the roster, with the roster's read.
    for year in 0..3 {
        let n = board(g, "sb_needs", json!({ "year": year }));
        assert_eq!(n["spots"].as_array().map(Vec::len), Some(24), "{level}");
        assert_eq!(n["years"].as_array().map(Vec::len), Some(3), "{level}");
        if year > 0 {
            continue;
        }
        for s in n["spots"].as_array().unwrap() {
            let Some(id) = s["names"][0]["id"].as_u64() else { continue };
            if s["names"][0]["commit"] == json!(true) {
                continue;
            }
            assert!(
                ids.contains(&id),
                "{level}: {} starts at {} but isn't on the roster",
                s["names"][0]["name"],
                s["key"]
            );
            let r = rows.iter().find(|r| r["id"].as_u64() == Some(id)).unwrap();
            if let (Some(o), Some(v)) = (r["ovr"].as_array(), s["vals"][0].as_f64())
                && r["pos"] == s["pos"]
            {
                let m = (o[0].as_f64().unwrap() + o[1].as_f64().unwrap()) / 2.0;
                assert!((m - v).abs() <= 0.5, "{level}: {} reads {m} on the roster, {v} on the needs map", s["key"]);
            }
        }
    }

    // A few players' shape, career arc and body: today's read is the roster's.
    for r in rows.iter().step_by(7).take(4) {
        let id = r["id"].clone();
        let sh = board(g, "sb_shape", json!({ "id": id }));
        assert_eq!(sh["families"].as_array().map(Vec::len), Some(8), "{level}");
        assert_eq!(sh["read"], r["ovr"], "{level}: the shape's read is the roster's");
        let arc = board(g, "sb_arc", json!({ "id": id }));
        let now = arc["points"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(now["now"], true);
        assert_eq!(now["read"], r["ovr"], "{level}: the arc's read today is the roster's");
        assert!(arc["peak"].is_null() || arc["peak"]["v"].is_number(), "{level}: {}", arc["peak"]);
        let seasons: Vec<i64> = arc["table"].as_array().unwrap().iter().filter_map(|t| t["season"].as_i64()).collect();
        assert!(seasons.windows(2).all(|x| x[0] < x[1]), "{level}: one row a season, in order: {seasons:?}");
        let body = board(g, "sb_body", json!({ "id": id }));
        assert_eq!(body["compares"].as_array().map(Vec::len), Some(3), "{level}");
        assert!(body["rows"].as_array().is_some_and(|x| x.iter().any(|t| t["key"] == "forty")), "{level}");
        projected(level, &body);
    }
    rows
}

/// Projected test numbers (Cain 2026-11-14): your players get a coming
/// testing day by their level, each projection's middle sits inside its
/// band, says whose read it is built on, and scores the same tests.
fn projected(level: &str, body: &Value) {
    let tg = body["targets"].as_array().unwrap();
    let keys: Vec<&str> = tg.iter().filter_map(|t| t["key"].as_str()).collect();
    match level {
        "nfl" => assert_eq!(keys, ["next"], "{level}"),
        "college" => assert_eq!(keys, ["combine"], "{level}"),
        _ => assert!(keys.contains(&"combine"), "{level}: {keys:?}"),
    }
    for t in tg {
        if t["error"].is_string() {
            continue;
        }
        assert!(matches!(t["whose"].as_str(), Some("yours" | "staff")), "{level}: {}", t["whose"]);
        let rows = t["rows"].as_array().unwrap();
        assert_eq!(rows.len(), body["rows"].as_array().unwrap().len(), "{level}");
        assert_eq!(t["scores"].as_array().map(Vec::len), Some(3), "{level}");
        for r in rows {
            let Some(v) = r["value"].as_f64() else { continue };
            assert!(v.is_finite() && v > 0.0, "{level}: {} {v}", r["key"]);
        }
        let forty = rows.iter().find(|r| r["key"] == "forty").unwrap();
        let v = forty["value"].as_f64().expect("a projected forty");
        assert!((4.2..=6.0).contains(&v), "{level}: forty {v}");
        let band = forty["band"].as_str().unwrap_or("");
        let ends: Vec<f64> = band.split('-').filter_map(|x| x.trim().parse().ok()).collect();
        if ends.len() == 2 {
            assert!(ends[0] <= v + 1e-9 && v <= ends[1] + 1e-9, "{level}: {v} outside {band}");
        }
        assert!(t["weights"].as_array().is_some_and(|w| !w.is_empty()), "{level}");
    }
}

/// The quick filter's data (Cain, 2026-11-04): every row carries each of
/// the 67 attributes as your staff's range (or not seen), never one
/// number, and the fields name the rating and the 67, never a ceiling.
fn quick_filter(level: &str, v: &Value) {
    let fields: Vec<&Value> = v["qf"]
        .as_array()
        .unwrap_or_else(|| panic!("{level}: no quick filter fields"))
        .iter()
        .flat_map(|g| g["fields"].as_array().unwrap())
        .collect();
    assert_eq!(fields.iter().filter(|f| f["kind"] == "read").count(), 68, "{level}: the rating and the 67 attributes");
    assert!(fields.iter().any(|f| f["label"] == "Speed"), "{level}");
    for f in &fields {
        let l = f["label"].as_str().unwrap().to_lowercase();
        assert!(!l.contains("potential") && !l.contains("ceiling"), "{level}: {l} is hidden");
    }
    for r in v["rows"].as_array().unwrap() {
        let a = r["attrs"].as_array().unwrap_or_else(|| panic!("{level}: {} has no attributes", r["name"]));
        assert_eq!(a.len(), 67, "{level}");
        for x in a.iter().filter(|x| !x.is_null()) {
            let (lo, hi) = (x[0].as_f64().unwrap(), x[1].as_f64().unwrap());
            let least = if lo <= 1.0 || hi >= 99.0 { 1.0 } else { 2.0 };
            assert!(hi - lo >= least, "{level}: {x} is too sure to be a staff's read");
        }
    }
}

/// Puts one of your players in the transfer portal (`held`: on hold for a
/// playoff run, with no day yet), or takes him out of it.
fn set_portal(g: &mut Game, id: u32, entry: Option<bool>) {
    use afm_engine::calendar::Date;
    let w = g.world.as_mut().unwrap();
    let today = w.today;
    let p = w.people.iter_mut().find(|p| p.player.id == id).unwrap();
    p.cal.portal = entry.map(|held| afm_engine::world::PortalEntry {
        entered: if held { Date(i32::MAX / 2) } else { today },
        decide_by: if held { Date(i32::MAX / 2) } else { today.add(20) },
        why: "playing time",
        held,
        looking: false,
    });
}

/// A starter who enters the transfer portal counts as gone on the needs
/// map (Cain 2026-11-12): he is off every fall, the next man starts, the
/// roster count drops, and the map says the place he would have had. An
/// entrant on hold for a playoff run plays this fall out.
fn portal_on_the_needs_map(g: &mut Game) {
    let before = board(g, "sb_needs", json!({}));
    assert_eq!(before["portal"], json!([]), "nobody in the portal yet");
    let spots = |v: &Value| v["spots"].as_array().unwrap().clone();
    // A spot whose starter holds it this fall and next.
    let spot = spots(&before)
        .into_iter()
        .find(|s| s["names"][0]["id"].is_u64() && s["names"][0]["id"] == s["names"][1]["id"])
        .expect("a starter who stays");
    let key = spot["key"].as_str().unwrap().to_string();
    let id = spot["names"][0]["id"].as_u64().unwrap() as u32;
    let at = |v: &Value| spots(v).into_iter().find(|s| s["key"] == key.as_str()).unwrap();
    let man = |v: &Value| v["men"].as_array().unwrap().iter().find(|m| m["id"] == json!(id)).unwrap().clone();
    let counted = |v: &Value| v["roster_line"].as_str().unwrap().split(' ').next().unwrap().parse::<u32>().unwrap();
    assert_eq!(man(&before)["spots"][0], key.as_str(), "the men list knows where he starts");
    assert_eq!(man(&before)["portal"], false);

    set_portal(g, id, Some(false));
    let after = board(g, "sb_needs", json!({}));
    let s = at(&after);
    for n in 0..3 {
        assert_ne!(s["names"][n]["id"], json!(id), "{key}: he is in the portal and still starts in fall {n}");
    }
    assert!(s["why"].as_str().unwrap().contains("is in the transfer portal and counts as gone"), "{key}: {}", s["why"]);
    let m = man(&after);
    assert_eq!(m["vals"], json!([null, null, null]), "off every fall");
    assert_eq!((&m["portal"], &m["gone"]["at"], &m["draft_risk"]), (&json!(true), &json!(0), &json!(false)));
    // Other positions' spots stand as they were.
    for (a, b) in spots(&before).iter().zip(&spots(&after)) {
        if a["pos"] != spot["pos"] {
            assert_eq!((&a["names"], &a["vals"]), (&b["names"], &b["vals"]), "{}", a["key"]);
        }
    }
    // The place he would have had: the spot he held, on the same read.
    assert_eq!(after["portal"].as_array().map(Vec::len), Some(1));
    let row = &after["portal"][0];
    assert_eq!((&row["id"], &row["on_hold"], &row["why"]), (&json!(id), &json!(false), &json!("playing time")));
    for n in 0..2 {
        let st = &row["stayed"][n];
        assert_eq!((&st["role"], &st["spot"], &st["fall"]), (&json!("Starter"), &json!(key), &after["falls"][n]));
        assert_eq!((&st["val"], &st["verdict"]), (&spot["vals"][n], &spot["verdicts"][n]), "the map's own numbers");
        assert!(st["words"].as_str().unwrap().starts_with("Starter at "), "{}", st["words"]);
    }
    assert!(after["coming"].as_u64().is_some_and(|c| c <= 1), "{}", after["coming"]);
    let leaving = after["leaving"].as_array().unwrap().iter().find(|l| l["label"] == "In the portal").unwrap();
    assert_eq!(leaving["n"], 1);
    assert_eq!(leaving["names"][0], spot["names"][0]["name"]);
    assert_eq!(counted(&after) + 1, counted(&before), "off the roster count");
    assert!(after["roster_line"].as_str().unwrap().contains("· 1 in the portal"), "{}", after["roster_line"]);

    // On hold for a playoff run: he plays this fall out and is gone after.
    set_portal(g, id, Some(true));
    let held = board(g, "sb_needs", json!({}));
    let s = at(&held);
    assert_eq!(s["names"][0]["id"], json!(id), "{key}: on hold, he still starts now");
    assert_ne!(s["names"][1]["id"], json!(id), "{key}: gone next fall");
    assert_eq!((&held["portal"][0]["on_hold"], &man(&held)["gone"]["at"]), (&json!(true), &json!(1)));
    assert_eq!(counted(&held), counted(&before), "still on the roster count");

    // Out of the portal again, the map is as it was.
    set_portal(g, id, None);
    assert_eq!(board(g, "sb_needs", json!({})), before);
}

/// The line and the named jobs (playtest bug 12): an untouched board shows
/// the men the game lines up at each spot, a tackle dropped at left tackle
/// plays left tackle, and only the position you changed becomes yours.
fn line_spots(g: &mut Game) {
    const LINE: [&str; 5] = ["LT", "LG", "C", "RG", "RT"];
    let seats = |g: &Game| {
        let w = g.world.as_ref().unwrap();
        let (team, level) = w.seat_game_team().expect("a team that can dress");
        afm_engine::game::flow::seating(&team, level)
    };
    let slots = |g: &mut Game, key: &str| -> Vec<(String, Option<u32>)> {
        let b = ok(g, "boards", json!({}));
        let b = b["boards"].as_array().unwrap().iter().find(|b| b["key"] == key).unwrap().clone();
        b["slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| (s["slot"].as_str().unwrap().to_string(), s["player"]["id"].as_u64().map(|x| x as u32)))
            .collect()
    };
    let at = |v: &[(String, Option<u32>)], name: &str| v.iter().position(|s| s.0 == name).expect("the slot");
    assert!(g.world.as_ref().unwrap().seat_depth.is_empty(), "nothing set by hand");
    let before = seats(g);
    // Untouched: each named spot shows the man the game plays there, on
    // the practice board, the stadium board and the flat chart.
    let off = slots(g, "offense");
    for name in LINE {
        assert_eq!(off[at(&off, name)].1, before.at(name, 0), "the board's {name}");
    }
    let def = slots(g, "defense");
    for name in ["SAM", "MIKE", "WILL", "FS", "SS"] {
        assert_eq!(def[at(&def, name)].1, before.at(name, 0), "the board's {name}");
    }
    let stadium = ok(g, "sb_depth", json!({ "tab": "offense", "form": "offense" }));
    for s in stadium["slots"].as_array().unwrap() {
        let name = s["slot"].as_str().unwrap();
        if LINE.contains(&name) {
            assert_eq!(
                s["picks"][0]["id"].as_u64().map(|x| x as u32),
                before.at(name, 0),
                "the stadium board's {name}"
            );
        }
    }
    let flat = ok(g, "depth", json!({}));
    let row = |flat: &Value, pos: &str| {
        flat["positions"].as_array().unwrap().iter().find(|p| p["pos"] == pos).unwrap().clone()
    };
    let ot = row(&flat, "OT");
    assert_eq!(ot["slots"], json!(["LT", "RT"]), "the flat chart names the tackles' rows");
    assert_eq!(ot["players"][0]["id"].as_u64().map(|x| x as u32), before.at("LT", 0));
    assert_eq!(ot["players"][1]["id"].as_u64().map(|x| x as u32), before.at("RT", 0));
    let iol = row(&flat, "IOL");
    assert_eq!(iol["slots"], json!(["LG", "C", "RG"]));
    for (k, name) in ["LG", "C", "RG"].into_iter().enumerate() {
        assert_eq!(iol["players"][k]["id"].as_u64().map(|x| x as u32), before.at(name, 0), "the flat chart's {name}");
    }

    // The tackles swapped on the board: the game follows, the inside
    // three stay, and only the tackles are yours.
    let (lt, rt) = (at(&off, "LT"), at(&off, "RT"));
    let mut ids: Vec<Value> = off.iter().map(|s| json!(s.1)).collect();
    ids.swap(lt, rt);
    ok(g, "board_set", json!({ "board": "offense", "ids": ids, "positions": ["OT"] }));
    let swapped = seats(g);
    assert_eq!(swapped.at("LT", 0), before.at("RT", 0), "the man dropped at left tackle plays left tackle");
    assert_eq!(swapped.at("RT", 0), before.at("LT", 0), "and the other plays right tackle");
    for name in ["LG", "C", "RG"] {
        assert_eq!(swapped.at(name, 0), before.at(name, 0), "the {name} stays");
    }
    assert_eq!(swapped.jobs, before.jobs, "the defense is as it was");
    let yours: Vec<_> = g.world.as_ref().unwrap().seat_depth.iter().map(|x| x.0).collect();
    assert_eq!(yours, vec![afm_engine::positions::Position::OT], "only the tackles are yours");
    let now = slots(g, "offense");
    assert_eq!((now[lt].1, now[rt].1), (off[rt].1, off[lt].1), "the board shows the swap");
    assert_eq!(row(&ok(g, "depth", json!({})), "OT")["players"][0]["id"].as_u64().map(|x| x as u32), off[rt].1);

    // The next tackle moved up at left tackle: the right tackle stays.
    let stadium = ok(g, "sb_depth", json!({ "tab": "offense", "form": "offense" }));
    let k = stadium["slots"].as_array().unwrap().iter().position(|s| s["slot"] == "LT").unwrap();
    if let Some(next) = stadium["slots"][k]["picks"][1]["id"].as_u64().map(|x| x as u32) {
        ok(g, "sb_depth_up", json!({ "tab": "offense", "form": "offense", "slot": k, "n": 2 }));
        let moved = seats(g);
        assert_eq!(moved.at("LT", 0), Some(next), "the backup plays left tackle");
        assert_eq!(moved.at("RT", 0), swapped.at("RT", 0), "the right tackle stays");
    }
    // Given back: the staff's seating again.
    ok(g, "sb_depth_reset", json!({ "tab": "offense", "form": "offense" }));
    assert!(g.world.as_ref().unwrap().seat_depth.is_empty(), "the staff has it back");
    assert_eq!(seats(g), before, "the staff's line again");

    // The nickel dropped at free safety: he plays free safety, the corners
    // stay, and another corner takes the nickel job (the Base board doesn't
    // show it, so the corners' order is written too).
    if let (Some(nickel), Some(fs)) = (before.at("NICKEL", 0), before.at("FS", 0)) {
        let def = slots(g, "defense");
        let mut ids: Vec<Value> = def.iter().map(|s| json!(s.1)).collect();
        ids[at(&def, "FS")] = json!(nickel);
        // The screens name the spot he lands on; he came from off the board.
        ok(g, "board_set", json!({ "board": "defense", "ids": ids, "positions": ["S"] }));
        let now = seats(g);
        assert_eq!(now.at("FS", 0), Some(nickel), "the nickel plays free safety");
        assert!(now.at("NICKEL", 0).is_some_and(|x| x != nickel && x != fs), "another corner plays nickel");
        let (mut a, mut b) = ([now.at("CB", 0), now.at("CB", 1)], [before.at("CB", 0), before.at("CB", 1)]);
        a.sort();
        b.sort();
        assert_eq!(a, b, "the outside corners stay");
        for name in ["SAM", "MIKE", "WILL"] {
            assert_eq!(now.at(name, 0), before.at(name, 0), "the {name} stays");
        }
        let mut yours: Vec<_> = g.world.as_ref().unwrap().seat_depth.iter().map(|x| x.0.name()).collect();
        yours.sort();
        assert_eq!(yours, vec!["CB", "S"], "the corners and safeties are yours, nobody else");
        ok(g, "sb_depth_reset", json!({ "tab": "defense", "form": "defense" }));
        assert!(g.world.as_ref().unwrap().seat_depth.is_empty(), "the staff has it back");
    }
}

/// Playtest bug 16: every backup cell takes a drop. A man put into a second
/// or third string cell holds it (read back from the stadium board), two
/// cells at one position swap, a backup put into the starter's cell swaps
/// with him, a man from another position group is refused with the reason,
/// and giving the order back restores the staff's.
fn backup_spots(g: &mut Game) {
    let view = |g: &mut Game| ok(g, "sb_depth", json!({ "tab": "offense", "form": "offense" }));
    let ids = |v: &Value, slot: &str| -> Vec<Option<u32>> {
        let s = v["slots"].as_array().unwrap().iter().find(|s| s["slot"] == slot).unwrap();
        s["picks"].as_array().unwrap().iter().map(|p| p["id"].as_u64().map(|x| x as u32)).collect()
    };
    let put = |g: &mut Game, slot: &str, n: u32, pid: u32| -> Result<Value, String> {
        let v = view(g);
        let k = v["slots"].as_array().unwrap().iter().position(|s| s["slot"] == slot).unwrap();
        g.call("sb_depth_put", &json!({ "tab": "offense", "form": "offense", "slot": k, "n": n, "pid": pid }))
    };
    let v0 = view(g);
    let start_lt = ids(&v0, "LT");
    let (lt, rt) = (ids(&v0, "LT"), ids(&v0, "RT"));
    if lt.len() != 3 || rt.len() != 3 || lt.iter().chain(rt.iter()).any(Option::is_none) {
        return;
    }
    let lt: Vec<u32> = lt.into_iter().flatten().collect();
    let rt: Vec<u32> = rt.into_iter().flatten().collect();

    // Third string up to second at one spot: the two swap.
    let v = put(g, "LT", 2, lt[2]).unwrap();
    assert_eq!(ids(&v, "LT"), vec![Some(lt[0]), Some(lt[2]), Some(lt[1])], "the third and second swap");
    assert_eq!(ids(&v, "RT"), vec![Some(rt[0]), Some(rt[1]), Some(rt[2])], "the right tackle's men stay");
    assert_eq!(v["yours"], true);
    assert_eq!(ids(&view(g), "LT")[1], Some(lt[2]), "kept on a fresh read");
    // The right tackle's second string into the left tackle's third: swap
    // across spots at the same position.
    let v = put(g, "LT", 3, rt[1]).unwrap();
    assert_eq!(ids(&v, "LT"), vec![Some(lt[0]), Some(lt[2]), Some(rt[1])]);
    assert_eq!(ids(&v, "RT"), vec![Some(rt[0]), Some(lt[1]), Some(rt[2])], "he took the other's cell");
    // A backup into the starter's cell: he starts, the starter is behind him.
    let v = put(g, "LT", 1, rt[1]).unwrap();
    assert_eq!(ids(&v, "LT"), vec![Some(rt[1]), Some(lt[2]), Some(lt[0])]);
    let mine: Vec<_> = g.world.as_ref().unwrap().seat_depth.iter().map(|x| x.0).collect();
    assert_eq!(mine, vec![afm_engine::positions::Position::OT], "only the tackles are written");
    // The same cell again says so; a fourth string and a man who isn't on
    // the roster are refused.
    let said = put(g, "LT", 1, rt[1]).unwrap();
    assert!(said["note"].as_str().unwrap().contains("already"), "{}", said["note"]);
    assert!(put(g, "LT", 4, lt[0]).is_err());
    assert!(put(g, "LT", 2, 4_000_000_000).is_err());
    // A man from another position group is refused, with the reason.
    let off = view(g);
    let wr =
        off["slots"].as_array().unwrap().iter().find(|s| s["pos"] == "WR").and_then(|s| s["picks"][0]["id"].as_u64());
    if let Some(wr) = wr {
        let e = put(g, "LT", 2, wr as u32).unwrap_err();
        assert!(e.contains("position group"), "{e}");
    }
    // Given back, the staff's order again.
    ok(g, "sb_depth_reset", json!({ "tab": "offense", "form": "offense" }));
    assert_eq!(ids(&view(g), "LT"), start_lt);
}

/// Plan ahead on the depth chart (Cain, 2026-11-13): the base board for each
/// of the next three falls. A planned fall holds only men who are still there
/// that fall (a senior is out the next fall), nothing in it touches the
/// chart now or the world, a drop saves to the plan and reads back, a man
/// who is gone by then is refused, giving the year back restores the
/// prefill, and the plan is a file beside the save, not part of it.
fn plan_ahead(g: &mut Game, rows: &[Value]) {
    let left: std::collections::HashMap<u64, u64> =
        rows.iter().filter_map(|r| Some((r["id"].as_u64()?, r["years_left"].as_u64()?))).collect();
    let view =
        |g: &mut Game, off: u32| board(g, "sb_depth", json!({ "tab": "offense", "form": "offense", "offset": off }));
    let ids = |v: &Value, slot: &str| -> Vec<Option<u32>> {
        let s = v["slots"].as_array().unwrap().iter().find(|s| s["slot"] == slot).unwrap();
        s["picks"].as_array().unwrap().iter().map(|p| p["id"].as_u64().map(|x| x as u32)).collect()
    };
    let now = view(g, 0);
    assert!(now["plan"].is_null() && now["plan_falls"].as_array().map(Vec::len) == Some(4), "{}", now["plan_falls"]);
    let before: Vec<Vec<Option<u32>>> =
        now["slots"].as_array().unwrap().iter().map(|s| ids(&now, s["slot"].as_str().unwrap())).collect();
    let seat_before = g.world.as_ref().unwrap().seat_depth.clone();

    // Every planned fall holds men who are still there, once each.
    for off in 1..=3u32 {
        let v = view(g, off);
        assert_eq!(v["plan"]["offset"], json!(off));
        assert_eq!(v["plan"]["fall"].as_i64(), Some(g.world.as_ref().unwrap().year as i64 + off as i64));
        assert!(v["units"].as_array().unwrap().is_empty() && v["next"].is_null(), "no games or unit cards in a plan");
        let mut seen = std::collections::HashSet::new();
        for s in v["slots"].as_array().unwrap() {
            for p in s["picks"].as_array().unwrap().iter().filter(|p| p.is_object()) {
                let id = p["id"].as_u64().unwrap();
                assert!(seen.insert(id), "{id} is planned twice in fall +{off}");
                if let Some(l) = left.get(&id) {
                    assert!(*l > off as u64, "{} has {l} seasons left and is planned in fall +{off}", p["name"]);
                }
                assert_eq!(p["health"], "full", "no injury tags that far out");
            }
        }
    }
    // The next fall holds fewer of today's starters when seniors go.
    let seniors: Vec<u64> = left.iter().filter(|(_, l)| **l == 1).map(|(id, _)| *id).collect();
    let one = view(g, 1);
    let one_ids: std::collections::HashSet<u64> = one["slots"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["picks"].as_array().unwrap().iter().filter_map(|p| p["id"].as_u64()))
        .collect();
    assert!(seniors.iter().all(|id| !one_ids.contains(id)), "a man out of eligibility is not in the next fall");

    // A drop saves to the plan and reads back; the chart now is untouched.
    let spots = one["slots"].as_array().unwrap().clone();
    let k = spots
        .iter()
        .position(|s| s["picks"].as_array().unwrap().iter().filter(|p| p.is_object()).count() >= 2)
        .expect("a spot with two men planned");
    let slot = spots[k]["slot"].as_str().unwrap().to_string();
    let men = ids(&one, &slot);
    let (a, b) = (men[0].unwrap(), men[1].unwrap());
    let put = json!({ "tab": "offense", "form": "offense", "offset": 1, "slot": k, "n": 2, "pid": a });
    let after = ok(g, "sb_plan_put", put);
    let planned = ids(&after, &slot);
    assert_eq!(planned[0], Some(b), "the starter and the second man swapped in the plan: {planned:?}");
    assert_eq!(planned[1], Some(a));
    assert_eq!(after["yours"], true, "the reset button shows once a spot is yours");
    assert_eq!(ids(&view(g, 1), &slot), planned, "read back on a fresh view");
    let still = view(g, 0);
    let now_ids: Vec<Vec<Option<u32>>> =
        still["slots"].as_array().unwrap().iter().map(|s| ids(&still, s["slot"].as_str().unwrap())).collect();
    assert_eq!(now_ids, before, "the chart now is untouched");
    assert_eq!(g.world.as_ref().unwrap().seat_depth, seat_before, "nothing in the world changed");
    // The same spot two falls out is still the prefill: a plan is for one fall.
    assert_eq!(view(g, 2)["yours"], false);

    // A man who is gone by then is refused with the reason.
    if let Some(sr) = seniors.first() {
        let e = g
            .call(
                "sb_plan_put",
                &json!({ "tab": "offense", "form": "offense", "offset": 1, "slot": k, "n": 3, "pid": sr }),
            )
            .unwrap_err();
        assert!(e.contains("won't be with you"), "{e}");
    }
    // Only the base boards are planned.
    assert!(g.call("sb_depth", &json!({ "tab": "offense", "form": "jumbo", "offset": 1 })).is_err());

    // The plan is a file beside the save and goes with it.
    let file = afm_game::depth_plan::path_for(g);
    assert!(file.exists(), "written the moment you drop: {}", file.display());
    let saved = ok(g, "save", json!({ "name": "planned" }))["saved"].as_str().unwrap().to_string();
    let beside = afm_game::depth_plan::beside(std::path::Path::new(&saved));
    assert!(beside.exists(), "beside the save: {}", beside.display());
    assert_eq!(ids(&view(g, 1), &slot), planned, "still there after a save");
    assert!(!std::fs::read_to_string(&saved).map(|t| t.contains("afmplan")).unwrap_or(false));

    // Giving the year back restores the prefill.
    ok(g, "sb_plan_reset", json!({ "tab": "offense", "form": "offense", "offset": 1 }));
    assert_eq!(ids(&view(g, 1), &slot), men, "the prefill is back");
    assert!(!afm_game::depth_plan::read(&beside).years.contains_key(&(g.world.as_ref().unwrap().year + 1).to_string()));
}

#[test]
fn the_boards_at_every_seat() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-squadboards-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 17, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    println!("world built in {:.0}s", t.elapsed().as_secs_f64());
    let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();
    let base =
        json!({ "name": "Test Coach", "age": 46, "background": "college_assistant", "seed": 5, "calls": "none" });
    let start = |g: &mut Game, job: Value| {
        let mut a = base.clone();
        a["job"] = job;
        ok(g, "start_career", a);
    };

    // A college head coach, a game into the season.
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    start(&mut g, job);
    // Continue stops for the day's news too: step until a game is kept.
    let mut steps = 0;
    while ok(&mut g, "games", json!({}))["games"].as_array().unwrap().is_empty() && steps < 400 {
        ok(&mut g, "day", json!({}));
        steps += 1;
    }
    let rows = every_board(&mut g, "college");
    assert!(
        rows.iter().any(|r| r["last6"].as_array().is_some_and(|x| x.iter().any(Value::is_number))),
        "grades from the game"
    );
    // Another team's player nobody on your staff has looked at: no read,
    // no outlook, and the tape says so (minor bug 355: the Shape tab broke
    // on him). A projection comes only with a read.
    let others: Vec<u32> = {
        let w = g.world.as_ref().unwrap();
        w.people.iter().filter(|p| p.stage == afm_engine::world::Stage::College).map(|p| p.player.id).collect()
    };
    let mut unseen = 0;
    for id in others.iter().step_by(97).take(60) {
        let sh = board(&mut g, "sb_shape", json!({ "id": id }));
        assert_eq!(sh["read"].is_null(), sh["outlook"].is_null(), "an outlook comes with a read: {id}");
        if sh["own"] == false && sh["read"].is_null() {
            assert_eq!(sh["tape"], "Nobody on your staff has looked at him yet", "{id}");
            unseen += 1;
        }
    }
    assert!(unseen > 0, "the sample holds a player your staff hasn't seen");
    // The quick filter's production: the game's lines, this season.
    let squad = ok(&mut g, "squad", json!({}));
    assert!(
        squad["qf"].as_array().unwrap().iter().any(|g| g["name"] == "Production, this season in your games"),
        "{}",
        squad["qf"]
    );
    assert!(
        squad["rows"].as_array().unwrap().iter().any(|r| r["prod"]["g"].as_u64().unwrap_or(0) > 0),
        "production from the game"
    );

    // A backup moved up is the order the staff plays; given back, it's theirs.
    let off = ok(&mut g, "sb_depth", json!({ "tab": "offense", "form": "offense" }));
    let k = off["slots"].as_array().unwrap().iter().position(|s| s["slot"] == "QB").unwrap();
    let (first, second) = (off["slots"][k]["picks"][0]["id"].clone(), off["slots"][k]["picks"][1]["id"].clone());
    let moved = ok(&mut g, "sb_depth_up", json!({ "tab": "offense", "form": "offense", "slot": k, "n": 2 }));
    assert_eq!(moved["slots"][k]["picks"][0]["id"], second, "the backup starts");
    assert_eq!(moved["slots"][k]["picks"][1]["id"], first, "the starter is second");
    assert_eq!(moved["yours"], true);
    let order = ok(&mut g, "depth", json!({}));
    let qbs = order["positions"].as_array().unwrap().iter().find(|p| p["pos"] == "QB").unwrap().clone();
    assert_eq!(qbs["players"][0]["id"], second, "the flat depth chart agrees");
    let back = ok(&mut g, "sb_depth_reset", json!({ "tab": "offense", "form": "offense" }));
    assert_eq!(back["yours"], false, "the staff has it back");
    line_spots(&mut g);
    backup_spots(&mut g);
    plan_ahead(&mut g, &rows);

    // The reads kept at a season's end: a career's first turn keeps none
    // (it starts mid-year), the next keeps every player the staff follows,
    // with the read the boards showed that day.
    let id = rows[0]["id"].as_u64().unwrap() as u32;
    {
        let w = g.world.as_mut().unwrap();
        let season = w.year - 1;
        w.career.as_mut().unwrap().kept_through = 0;
        afm_game::lane_squadboards::after_days(w);
        let c = w.career.as_ref().unwrap();
        assert_eq!(c.kept_through as i32, season, "the first turn only marks the season");
        assert!(c.kept_reads.is_empty(), "nothing kept for a season the staff didn't follow");
        w.career.as_mut().unwrap().kept_through = (season - 1) as i16;
        afm_game::lane_squadboards::after_days(w);
        let c = w.career.as_ref().unwrap();
        assert_eq!(c.kept_of(id).len(), 1, "a roster player's read is kept");
        assert_eq!(c.kept_of(id)[0].season as i32, season);
    }
    let kept = g.world.as_ref().unwrap().career.as_ref().unwrap().kept_of(id)[0];
    assert_eq!(json!(kept.ovr), rows[0]["ovr"], "the kept read is the roster's that day");
    let arc = board(&mut g, "sb_arc", json!({ "id": id }));
    assert!(
        arc["points"].as_array().unwrap().iter().any(|p| p["now"] == false && p["read"] == rows[0]["ovr"]),
        "the arc shows the kept read: {}",
        arc["points"]
    );
    // Own potential (Cain 2026-11-13): your own players are in your
    // practices, so your own read of potential judges them, your staff's
    // beside it; your own read is kept at the season's end beside the
    // staff's; the road starts from yours and bends toward the ceiling read.
    {
        let w = g.world.as_ref().unwrap();
        let k = w.people.iter().position(|p| p.player.id == id).unwrap();
        let s = afm_game::reads::seen(w, k);
        assert!(s.mine_looks > 0 && s.mine_ceiling.is_some(), "your own player has your own ceiling read");
        let pot = afm_game::reads::potential_words(w, k);
        assert_eq!(pot["whose"], "yours", "{pot}");
        assert!(pot["staff"]["tier"].is_string() && pot["yours"]["tier"] == pot["tier"], "{pot}");
        let c = w.career.as_ref().unwrap();
        assert_eq!(c.kept_mine(id).len(), 1, "your own read is kept beside the staff's");
        assert_eq!(json!(c.kept_mine(id)[0].ovr), json!(s.mine().json()), "the kept own read is today's");
        // Two players read the same today with different ceiling reads get
        // different roads; the higher ceiling ends higher, the middle never
        // passes the ceiling read on the way up.
        let seasons: Vec<(i32, u32, u8)> = (1..=6).map(|n| (w.year + n, 19 + n as u32, 1)).collect();
        let lo = afm_game::reads::Judging { now: (60.0, 3.0), ceiling: (64.0, 4.0), yours: true };
        let hi = afm_game::reads::Judging { ceiling: (80.0, 4.0), ..lo };
        let (a, b) = (afm_game::road::walk(w, &lo, &seasons), afm_game::road::walk(w, &hi, &seasons));
        assert!(b[5].q[2] > a[5].q[2] + 3.0, "a higher ceiling read, a higher road: {:?} {:?}", a[5].q, b[5].q);
        assert!(a.iter().all(|r| r.q[2] <= 64.0 + 1e-9), "never past the ceiling read: {a:?}");
        assert!(b[5].q[4] - b[5].q[0] >= 2.0, "a band");
    }
    let arc = board(&mut g, "sb_arc", json!({ "id": id }));
    assert_eq!(arc["whose"], "yours", "the road starts from your own read");
    assert!(
        arc["points"].as_array().unwrap().iter().any(|p| p["now"] == false && !p["yours"].is_null()),
        "{}",
        arc["points"]
    );
    // Kept through a save; a career saved before the kept reads loads empty.
    let saved = ok(&mut g, "save", json!({ "name": "kept" }))["saved"].as_str().unwrap().to_string();
    ok(&mut g, "load", json!({ "path": saved }));
    assert_eq!(g.world.as_ref().unwrap().career.as_ref().unwrap().kept_of(id), &[kept], "kept through a save");
    let mut old = serde_json::to_value(g.world.as_ref().unwrap().career.as_ref().unwrap()).unwrap();
    old.as_object_mut().unwrap().remove("kept_reads");
    old.as_object_mut().unwrap().remove("kept_through");
    let old: afm_engine::world::seat::Career = serde_json::from_value(old).expect("an older career loads");
    assert!(old.kept_reads.is_empty() && old.kept_through == 0);

    portal_on_the_needs_map(&mut g);

    // An NFL head coach.
    ok(&mut g, "load", json!({ "path": fresh.clone() }));
    let job = ok(&mut g, "jobs", json!({ "level": "nfl" }))["jobs"][0]["ref"].clone();
    start(&mut g, job);
    let rows = every_board(&mut g, "nfl");
    assert!(rows.iter().any(|r| r["money"]["label"] == "Cap"), "contracts on the NFL roster");

    // A high school head coach at the biggest public school in Texas.
    ok(&mut g, "load", json!({ "path": fresh }));
    let hs = ok(&mut g, "jobs", json!({ "level": "hs" }));
    let big = hs["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|j| j["private"] != true && j["state"] == "TX")
        .max_by_key(|j| j["enrollment"].as_u64().unwrap_or(0))
        .unwrap()["ref"]
        .clone();
    start(&mut g, big);
    let rows = every_board(&mut g, "high school");
    assert!(rows.iter().all(|r| r["years"]["n"] == 4), "four years of high school");
    println!("done in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
