//! Lane commands: a CFL club's scouting year and its drafts (SK77), from
//! your chair when you coach in the CFL. The draft class is people from
//! the fall: your club's board from its own file, the Bureau's three
//! editions, the combines and the Canadians down south, the draft room
//! (your pick, your staff's pick, calls for your pick) and the Global
//! Draft. Only your staff's reads show: ranges, words and counts.
//!
//! - `cfl_draft`: everything the CFL Draft screen shows.
//! - `cfl_pick` (`id`), `cfl_auto`: your pick, or your staff's.
//! - `cfl_offer` (`k`, `say`: take, more or no): a call for your pick.

use afm_engine::calendar::Date;
use afm_engine::world::pro::{cfl, other};
use afm_engine::world::scouting::{self as sc, Kind, file, readers};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::views::{TeamRef, date_text, team_brief};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let num = |k: &str| args.get(k).and_then(Value::as_u64);
    let text = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let r = match cmd {
        "cfl_draft" => (|| view(game.w()?))(),
        "cfl_pick" => (|| {
            let id = num("id").ok_or("Missing id.")? as u32;
            let w = game.wm()?;
            cfl::pick(w, id)?;
            view(w)
        })(),
        "cfl_auto" => (|| {
            let w = game.wm()?;
            cfl::auto_pick(w)?;
            view(w)
        })(),
        "cfl_offer" => (|| {
            let k = num("k").unwrap_or(0) as usize;
            let w = game.wm()?;
            let said = match text("say").as_str() {
                "take" => {
                    cfl::accept(w, k)?;
                    w.others.room.as_ref().and_then(|r| r.trades.last().cloned()).unwrap_or_default()
                }
                "more" => cfl::ask_more(w, k)?,
                _ => {
                    cfl::decline(w);
                    "You turn them down.".to_string()
                }
            };
            let mut v = view(w)?;
            v["said"] = json!(said);
            Ok(v)
        })(),
        _ => return None,
    };
    Some(r)
}

fn club_of(w: &World) -> Result<usize, String> {
    cfl::seat_club(w).ok_or_else(|| "Only a CFL club runs the CFL draft.".to_string())
}

/// Where a prospect plays, in a few words: "Laval (RSEQ)", "Germany" or
/// his American college.
fn from_words(w: &World, i: usize) -> String {
    let p = &w.people[i];
    cfl::pool_words(w, p.player.id).or_else(|| p.college.map(|c| w.program_name(c).to_string())).unwrap_or_default()
}

/// What a man is to the draft beyond his name: a fourth-year who may go
/// back to school, an NCAA Canadian's NFL news.
fn note_words(w: &World, i: usize) -> String {
    let p = &w.people[i];
    if let Some(x) = w.others.pool.get(&p.player.id) {
        return match x.year {
            4 => "4th year: may go back to school".into(),
            0 => String::new(),
            y => format!("{y}th year"),
        };
    }
    south_words(w, i)
}

/// An NCAA Canadian's spring as the CFL sees it: the NFL comes first.
fn south_words(w: &World, i: usize) -> String {
    let p = &w.people[i];
    let id = p.player.id;
    let combine = w.clubs.iter().any(|c| {
        c.files.get(&id).is_some_and(|f| {
            f.iter().any(|l| {
                l.who == afm_engine::world::pro::scouts::EVENT
                    && l.kind == afm_engine::world::pro::scouts::LookKind::Combine
            })
        })
    });
    match p.stage {
        Stage::College => "Senior season".into(),
        Stage::Declared if combine => "NFL combine invite; in the NFL draft".into(),
        Stage::Declared => "In the NFL draft".into(),
        Stage::Pro => {
            let c = p.pro.as_ref();
            let team = c.and_then(|c| c.team).map(|t| w.nfl_team_name(t));
            match (c.and_then(|c| c.draft), team) {
                (Some((_, round, _, _)), Some(t)) => format!("NFL draft round {round}, the {t}"),
                (None, Some(t)) => format!("Signed with the {t} after the NFL draft"),
                _ => "Not signed in the NFL".into(),
            }
        }
        _ => String::new(),
    }
}

/// A prospect's row: his name, spot, where from, your staff's read.
fn row(w: &World, staff: readers::Staff, i: usize) -> Value {
    let p = &w.people[i];
    json!({
        "id": p.player.id,
        "name": p.name,
        "pos": p.player.position.name(),
        "from": from_words(w, i),
        "note": note_words(w, i),
        "read": crate::lane_scouting::brief_read(w, staff, p.player.id),
    })
}

/// The steps of the draft year, each done, now or next.
fn steps(w: &World, year: i32) -> Vec<Value> {
    let today = w.today;
    let draft = cfl::draft_date(year);
    let mut list = vec![
        ("Bureau, fall", "September · top 20", Date::ymd(year - 1, 9, 15)),
        ("Bureau, winter", "January · top 20", Date::ymd(year, 1, 14)),
        ("Invitational combine", "Early March", Date::ymd(year, 3, 6)),
        ("CFL Combine", "Mid March · tests and interviews", Date::ymd(year, 3, 20)),
        ("Bureau, spring", "April, after the combine", Date::ymd(year, 4, 15)),
    ];
    list.push(("CFL Draft", "Global Draft the next day", draft));
    if !cfl::global_draft(year) {
        list.last_mut().unwrap().1 = "Eight rounds";
    }
    let now = list.iter().position(|s| s.2 >= today).unwrap_or(list.len());
    list.iter()
        .enumerate()
        .map(|(k, (t, d, date))| {
            json!({ "t": t, "d": d, "date": date_text(*date),
                "state": if k < now { "done" } else if k == now { "now" } else { "next" } })
        })
        .collect()
}

/// The testing numbers of a combine (every club gets the same).
fn tests(
    w: &World,
    staff: readers::Staff,
    list: &[usize],
    year: i32,
    rank: &std::collections::HashMap<u32, usize>,
) -> Vec<Value> {
    list.iter()
        .map(|&i| {
            let p = &w.people[i];
            let c = file::tests(&p.player, Kind::Combine, year);
            let (ht, wt) = file::listed(&p.player);
            let mut r = row(w, staff, i);
            r["ht"] = json!(format!("{}'{}\"", (ht / 12.0).floor() as i32, (ht % 12.0).round() as i32));
            r["wt"] = json!(wt.round());
            r["forty"] = json!(c.forty);
            r["bench"] = json!(c.bench.round());
            r["vert"] = json!(c.vertical);
            r["broad"] = json!(c.broad.round());
            r["shuttle"] = json!(c.shuttle);
            r["board"] = json!(rank.get(&p.player.id).map(|k| k + 1));
            r
        })
        .collect()
}

fn pick_row(w: &World, p: &other::Pick, n: usize) -> Value {
    let rights = w.others.rights.get(&p.id).map(|r| r.0 as usize);
    let i = sc::place(w, p.id);
    let now = i.map(|i| &w.people[i]).map_or(String::new(), |q| match q.stage {
        Stage::CflPool => "Back at school: rights held".into(),
        Stage::Pro if rights.is_some() => south_words(w, i.unwrap()) + ": rights held",
        _ => String::new(),
    });
    json!({
        "n": p.pick as usize + 1,
        "round": p.pick as usize / n.max(1) + 1,
        "club": team_brief(w, TeamRef::Other(p.team as usize)),
        "id": p.id,
        "name": p.name,
        "pos": p.pos.name(),
        "from": p.from,
        "now": now,
    })
}

pub fn view(w: &World) -> Reply {
    let fr = club_of(w)?;
    let staff = readers::Staff::League(fr);
    let year = cfl::coming_draft(w);
    let teams = other::teams_in(other::CFL, year).len();
    // Your board: the class still on it, best first, graded by your club.
    let board = cfl::seat_board(w);
    let rank: std::collections::HashMap<u32, usize> =
        board.iter().enumerate().map(|(k, &(i, _))| (w.people[i].player.id, k)).collect();
    let (next_mine, there) = cfl::there_odds(w);
    let bureau_now = w.others.bureau.iter().filter(|b| b.draft == year).max_by_key(|b| b.edition);
    let room = w.others.room.as_ref();
    let rounds = room.map_or(cfl::rounds(year, false), |r| r.rounds);
    let rows: Vec<Value> = board
        .iter()
        .take(80)
        .enumerate()
        .map(|(k, &(i, _))| {
            let id = w.people[i].player.id;
            let mut r = row(w, staff, i);
            r["rank"] = json!(k + 1);
            r["grade"] = json!(cfl::round_words(k, teams, rounds));
            r["bureau"] = json!(bureau_now.and_then(|b| b.ids.iter().position(|&x| x == id)).map(|x| x + 1));
            r["there"] = json!(next_mine.map(|_| there.get(&id).copied().unwrap_or(0)));
            r
        })
        .collect();
    // The Bureau's editions for this draft.
    let bureau: Vec<Value> = w
        .others
        .bureau
        .iter()
        .filter(|b| b.draft == year)
        .map(|b| {
            let list: Vec<Value> = b
                .ids
                .iter()
                .enumerate()
                .filter_map(|(k, &id)| {
                    let i = sc::place(w, id)?;
                    let mut r = row(w, staff, i);
                    r["rank"] = json!(k + 1);
                    r["yours"] = json!(rank.get(&id).map(|x| x + 1));
                    Some(r)
                })
                .collect();
            json!({ "edition": cfl::EDITIONS[b.edition as usize % 3], "date": date_text(b.date), "rows": list })
        })
        .collect();
    // The combines, once held.
    let national =
        (w.today >= Date::ymd(year, 3, 20)).then(|| tests(w, staff, &cfl::invited(w, year, false), year, &rank));
    let invitational =
        (w.today >= Date::ymd(year, 3, 6)).then(|| tests(w, staff, &cfl::invited(w, year, true), year, &rank));
    // The Canadians at American colleges in this class.
    let south: Vec<Value> = cfl::class(w, year, false)
        .into_iter()
        .filter(|&i| w.people[i].stage != Stage::CflPool)
        .map(|i| {
            let mut r = row(w, staff, i);
            r["what"] = json!(south_words(w, i));
            r["board"] = json!(rank.get(&w.people[i].player.id).map(|k| k + 1));
            r
        })
        .collect();
    // The room.
    let live = room.map(|r| {
        let n = r.order.len() / r.rounds.max(1) as usize;
        let k = r.next();
        let made: Vec<Value> = w
            .others
            .draft
            .iter()
            .filter(|p| p.year == r.year && p.global == r.global)
            .rev()
            .take(12)
            .map(|p| pick_row(w, p, n))
            .collect();
        let offers: Vec<Value> = r
            .offers
            .iter()
            .enumerate()
            .map(|(j, o)| {
                let words = |s: usize| format!("pick {}", s + 1);
                json!({
                    "k": j,
                    "club": team_brief(w, TeamRef::Other(o.from as usize)),
                    "text": format!("{} for your {}", match o.extra {
                        Some(x) => format!("{} and {}", words(o.their as usize), words(x as usize)),
                        None => words(o.their as usize),
                    }, words(k)),
                })
            })
            .collect();
        let staff_pick = cfl::staff_choice(w).map(|i| row(w, staff, i));
        json!({
            "global": r.global,
            "rounds": r.rounds,
            "picks": r.order.len(),
            "round": k / n.max(1) + 1,
            "pick": k + 1,
            "in_round": k % n.max(1) + 1,
            "on_clock": r.on_clock(),
            "next_mine": next_mine.map(|s| s + 1),
            "offers": offers,
            "trades": r.trades,
            "made": made,
            "staff_pick": staff_pick,
        })
    });
    // Each draft's picks once made: the national, then the Global.
    let results = |global: bool| -> Vec<Value> {
        let n = teams.max(1);
        w.others.draft.iter().filter(|p| p.year == year && p.global == global).map(|p| pick_row(w, p, n)).collect()
    };
    let global_class: Vec<Value> = if cfl::global_draft(year) {
        cfl::class(w, year, true).into_iter().take(40).map(|i| row(w, staff, i)).collect()
    } else {
        Vec::new()
    };
    Ok(json!({
        "club": team_brief(w, TeamRef::Other(fr)),
        "year": year,
        "teams": teams,
        "rounds": rounds,
        "has_global": cfl::global_draft(year),
        "steps": steps(w, year),
        "board": rows,
        "bureau": bureau,
        "national": national,
        "invitational": invitational,
        "south": south,
        "live": live,
        "national_picks": results(false),
        "global_picks": results(true),
        "global_class": global_class,
        "draft_date": date_text(cfl::draft_date(year)),
    }))
}
