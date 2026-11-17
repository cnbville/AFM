//! Capital's Pecking order screen (lane CS5, `docs/design/capital.md` CP29-CP31,
//! CP37, CP39, board 5): where the press puts your conference's facilities
//! each July, how you have moved, the rivals' drives you may know of and
//! what your own donors hear.
//!
//! - `capital_pecking`: the view. Every number is a rank, a share of a goal
//!   or a word (CP39); the press's own score and any dollar figure stay in
//!   the engine. Rivals' drives come only from `race::rivals_drives`: the
//!   public ones and the ones a donor of yours told you of, never a quiet
//!   phase you did not hear (CP30).
//!
//! It only reads: nothing here moves a result.

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::capital::drive::{self, Drive, Stage};
use afm_engine::world::capital::race::{self, Placed, Seen};
use afm_engine::world::finance::{Owner, ProjectKind};
use serde_json::{Value, json};

use crate::{Game, Reply};

/// The longest list of rumors the page shows.
const RUMORS: usize = 8;

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub fn call(game: &mut Game, cmd: &str, _args: &Value) -> Option<Reply> {
    Some(match cmd {
        "capital_pecking" => game.w().and_then(view),
        _ => return None,
    })
}

fn iso(d: Date) -> String {
    let (y, m, dd) = d.parts();
    format!("{y:04}-{m:02}-{dd:02}")
}

fn short(d: Date) -> String {
    let (_, m, dd) = d.parts();
    format!("{} {dd}", MONTHS[(m as usize).saturating_sub(1).min(11)])
}

fn pct(x: f64) -> i64 {
    (x * 100.0).round() as i64
}

/// The share of its goal a drive has raised, in whole percent.
fn share(w: &World, x: &Drive) -> i64 {
    if x.goal <= 0.0 { 0 } else { pct(drive::raised(&w.capital, x) / x.goal).clamp(0, 100) }
}

/// The money year a drive's building opens, while it is still going up.
fn going_up(w: &World, x: &Drive) -> Option<i32> {
    let j = drive::project_of(w, x)?;
    let y = &w.finance.projects[j];
    (y.owner == Owner::College(x.program as usize) && y.kind == ProjectKind::Facility && !y.done).then_some(y.opens)
}

/// What the Drive column says of one program: its own drive (you know all
/// of it), or a rival's public one, a building going up, or a rumor you
/// heard.
fn drive_cell(w: &World, p: usize, me: usize, seen: &[Seen]) -> Value {
    let cell = |kind: &str, words: String, share: Option<i64>| json!({ "kind": kind, "words": words, "share": share });
    if p == me {
        return match drive::latest(&w.capital, p).filter(|x| !x.shelved) {
            Some(x) if x.stage == Stage::Public => {
                let s = share(w, x);
                cell("yours", format!("Yours, public, {s}%"), Some(s))
            }
            Some(x) if matches!(x.stage, Stage::Study | Stage::Quiet) => {
                cell("yours", format!("Yours, in {}", x.stage.words()), None)
            }
            Some(x) => match going_up(w, x) {
                Some(o) => cell("yours", format!("Yours, building, opens {o}"), None),
                None => cell("none", "None".into(), None),
            },
            None => cell("none", "None".into(), None),
        };
    }
    for s in seen {
        match s {
            Seen::Public(x) if x.program as usize == p => {
                if x.stage == Stage::Public {
                    let sh = share(w, x);
                    return cell("public", format!("Public, {sh}%"), Some(sh));
                }
                if let Some(o) = going_up(w, x) {
                    return cell("building", format!("Building, opens {o}"), None);
                }
            }
            Seen::Rumored(r) if r.about as usize == p => return cell("rumored", "Rumored".into(), None),
            _ => {}
        }
    }
    cell("none", "None public".into(), None)
}

fn newest(w: &World, p: usize) -> Value {
    w.programs[p]
        .campus
        .halls
        .iter()
        .max_by_key(|h| h.opened)
        .map_or(Value::Null, |h| json!(format!("{}, {}", h.name, h.opened)))
}

fn row(w: &World, n: usize, x: &Placed, me: usize, prev: Option<&race::Pecking>, seen: &[Seen]) -> Value {
    let p = x.program as usize;
    let moved = prev.and_then(|q| q.rank(p)).map(|r| r as i64 - n as i64);
    json!({
        "n": n,
        "program": w.program_name(p),
        "me": p == me,
        "move": moved,
        "newest": newest(w, p),
        "because": race::because(w, x),
        "drive": drive_cell(w, p, me, seen),
    })
}

fn rumor_line(w: &World, seen: &[Seen]) -> Vec<Value> {
    let mut v: Vec<&race::Rumor> = seen
        .iter()
        .filter_map(|s| match s {
            Seen::Rumored(r) => Some(*r),
            Seen::Public(_) => None,
        })
        .collect();
    v.sort_by_key(|r| std::cmp::Reverse(r.date));
    v.into_iter()
        .take(RUMORS)
        .map(|r| {
            let them = w.program_name(r.about as usize);
            let who = w.capital.donors.iter().find(|d| d.id == r.from).map_or("A donor", |d| d.name.as_str());
            let mut detail =
                vec![if r.direct { "He was in the room for it." } else { "He heard it from friends." }.to_string()];
            if let Some(h) = r.heard {
                detail.push(race::progress_words(h).to_string());
            }
            if r.lead_in {
                detail.push("A lead gift is in.".to_string());
            }
            json!({
                "title": race::headline(r.stage, them, &r.building),
                "source": format!("{who}, {}", short(r.date)),
                "detail": detail.join(" "),
                "program": them,
            })
        })
        .collect()
}

fn view(w: &World) -> Reply {
    let me = afm_engine::world::ground::week::seat_program(w)
        .ok_or("The press ranks college programs. High school and NFL buildings come with their own screen.")?;
    let name = w.program_name(me).to_string();
    let seen = race::rivals_drives(w);
    let (this, prev) = race::standing(w, me);
    let pecking = this.map(|t| {
        // The last July's order only counts if the same conference was ranked.
        let prev = prev.filter(|q| q.conference == t.conference && q.division == t.division);
        let rows: Vec<Value> = t.order.iter().enumerate().map(|(k, x)| row(w, k + 1, x, me, prev, &seen)).collect();
        let rank = t.rank(me);
        json!({
            "year": t.year,
            "conference": t.conference,
            "of": t.order.len(),
            "rank": rank,
            "before": prev.and_then(|q| q.rank(me)),
            "rows": rows,
            "why": t.order.iter().find(|x| x.program as usize == me).map(|x| race::because(w, x)),
        })
    });
    let years: Vec<Value> =
        race::ranks(w, me).into_iter().map(|(y, r, of)| json!({ "year": y, "rank": r, "of": of })).collect();
    Ok(json!({
        "program": name,
        "today": iso(w.today),
        "pecking": pecking,
        "years": years,
        "rumors": rumor_line(w, &seen),
        "rivals": seen.len(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::world::capital::race::{Pecking, Placed, Rumor};

    fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
        match g.call(cmd, &args) {
            Ok(v) => v,
            Err(e) => panic!("{cmd}: {e}"),
        }
    }

    /// The pecking order screen on a small world: your conference ranked
    /// with your move, a rival's public drive and a rumor from a donor, in
    /// ranks and words (no dollar, CP39); a quiet phase you did not hear of
    /// stays off the page (CP30).
    #[test]
    fn the_pecking_screen_ranks_the_conference_and_shows_only_what_you_know() {
        let dir = std::env::temp_dir().join(format!("afm-capital-pecking-screen-{}", std::process::id()));
        let mut g = Game::new(dir.clone());
        ok(&mut g, "build_world", json!({ "seed": 23, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
        let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
        let job = jobs["jobs"][0]["ref"].clone();
        ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));
        {
            // Start clean: nothing ranked yet.
            let w = g.wm().unwrap();
            w.capital.pecking.clear();
            w.capital.rumors.clear();
        }
        let none = ok(&mut g, "capital_pecking", json!({}));
        assert!(none["pecking"].is_null(), "no July yet: {none}");
        assert!(none["rumors"].as_array().unwrap().is_empty());

        let (me, rival, quiet);
        {
            let w = g.wm().unwrap();
            me = afm_engine::world::ground::week::seat_program(w).expect("a college seat");
            let (div, conf) = (w.programs[me].division, w.programs[me].conference.clone());
            let mates: Vec<usize> = (0..w.programs.len())
                .filter(|&p| p != me && w.programs[p].division == div && w.programs[p].conference == conf)
                .take(3)
                .collect();
            assert!(mates.len() >= 2, "the conference has rivals");
            rival = mates[0];
            quiet = mates[1];
            let place = |p: usize, s: f32| Placed { program: p as u32, score: s, ..Placed::default() };
            let year = w.year;
            w.capital.pecking.push(Pecking {
                year: year - 1,
                division: div,
                conference: conf.clone(),
                order: vec![place(rival, 80.0), place(me, 70.0), place(quiet, 40.0)],
            });
            w.capital.pecking.push(Pecking {
                year,
                division: div,
                conference: conf.clone(),
                order: vec![place(me, 90.0), place(rival, 80.0), place(quiet, 40.0)],
            });
            // A rival's public drive, and a quiet phase nobody told you of.
            let today = w.today;
            let mk = |id: u32, p: usize, stage: Stage, public: bool| Drive {
                id,
                program: p as u32,
                kind: "football building".into(),
                goal: 50.0e6,
                stage,
                public_on: public.then(|| today.add(-30)),
                crowd: 25.0e6,
                ..Drive::default()
            };
            w.capital.drives.push(mk(9_100_001, rival, Stage::Public, true));
            w.capital.drives.push(mk(9_100_002, quiet, Stage::Quiet, false));
        }
        let v = ok(&mut g, "capital_pecking", json!({}));
        let p = &v["pecking"];
        assert_eq!(p["rank"], 1);
        assert_eq!(p["before"], 2);
        let rows = p["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["me"], true);
        assert_eq!(rows[0]["move"], 1, "up from second");
        assert_eq!(rows[1]["move"], -1, "down from first");
        assert_eq!(rows[1]["drive"]["kind"], "public");
        assert_eq!(rows[1]["drive"]["share"], 50);
        assert_eq!(rows[2]["drive"]["kind"], "none", "a quiet phase you did not hear of stays off the page");
        assert_eq!(v["years"].as_array().unwrap().len(), 2);
        assert!(!v.to_string().contains('$'), "no dollar figure on screen: {v}");

        // A donor tells you of the quiet phase: now it is rumored, with his line.
        {
            let w = g.wm().unwrap();
            let donor = w.capital.donors[0].id;
            let day = w.today.add(-3);
            w.capital.rumors.push(Rumor {
                date: day,
                about: quiet as u32,
                drive: 9_100_002,
                stage: Stage::Quiet,
                from: donor,
                direct: true,
                building: "a new football building".into(),
                heard: Some(0.4),
                truth: Some(0.5),
                lead_in: true,
                text: String::new(),
            });
        }
        let v = ok(&mut g, "capital_pecking", json!({}));
        assert_eq!(v["pecking"]["rows"][2]["drive"]["kind"], "rumored");
        let r = &v["rumors"][0];
        assert!(r["title"].as_str().unwrap().contains("quietly raising money"), "{r}");
        assert!(r["detail"].as_str().unwrap().contains("A lead gift is in."), "{r}");
        assert!(!v.to_string().contains('$'));
        let _ = std::fs::remove_dir_all(dir);
    }
}
