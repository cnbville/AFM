//! World trial 3: the debug dashboard export and full player pages, as JSON
//! for the mock GUI and the player pages. Hidden values included: debug only.

use std::collections::BTreeMap;

use afm_engine::debug;
use afm_engine::world::coach::Group;
use afm_engine::world::data::Division;
use afm_engine::world::game_log::{GameSummary, PlayerSeason};
use afm_engine::world::geo::STATE_CENTERS;
use afm_engine::world::{Alumnus, World};
use serde_json::{Value, json};

fn r1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// A player's college seasons from the game log, by year.
fn college_stats(w: &World, id: u32) -> Vec<Value> {
    let mut out = Vec::new();
    for y in (w.year - 20)..w.year {
        if let Some(s) = w.game_log.players.get(&(y, id)) {
            out.push(json!({
                "year": y, "school": w.programs[s.program].real_name, "g": s.games, "pos": s.pos.map(|p| p.name()),
                "cmp": s.cmp, "att": s.att, "pyd": s.pass_yds.round(), "ptd": s.pass_td, "int": s.int_thrown,
                "sk": s.sacked, "car": s.rush, "ryd": s.rush_yds.round(), "rtd": s.rush_td,
                "rec": s.rec, "recyd": s.rec_yds.round(), "rectd": s.rec_td,
                "tkl": s.tackles, "tfl": r1(s.tfl as f64), "sacks": r1(s.sacks as f64), "dint": s.ints,
                "pbu": s.pbus, "pr": s.pressures, "kr": s.kr, "kryd": s.kr_yds.round(),
                "fg": format!("{}/{}", s.fg_made, s.fg_att),
                "grade": if s.graded > 0 { r1(s.grade_sum as f64 / s.graded as f64) } else { 0.0 }
            }));
        }
    }
    out
}

/// A full player page: every attribute with its hidden ceiling, body and
/// combine, season-by-season ratings, coaches, college stats and the story.
pub fn card(w: &World, a: &Alumnus, tag: &str) -> Value {
    let p = a.player.as_ref().unwrap();
    let ceil = debug::ceilings(p);
    let attrs: Vec<Value> = afm_engine::attributes::Attr::ALL
        .iter()
        .map(|&t| json!([format!("{t:?}"), p.attributes[t].round(), ceil[t].round()]))
        .collect();
    let seasons: Vec<Value> = p
        .history
        .iter()
        .map(|h| {
            json!({
                "age": h.age, "level": h.level.name(), "pos": h.position.name(), "role": h.playing_time.name(),
                "start": h.overall_start.round(), "end": h.overall.round(),
                "perf": h.performance.map(|v| v.round()), "lr": h.level_rating, "lr_end": h.level_rating_end,
                "pot": debug::season_potential(h).round()
            })
        })
        .collect();
    let body = afm_engine::body::body_now(p);
    let mut rng = afm_engine::rng::Rng::new(p.id ^ 0xC0B1);
    let cmb = afm_engine::body::combine(p, &mut rng);
    let staff: Vec<Value> = a
        .college_staff
        .iter()
        .map(|&(y, prog, hc, pc)| {
            json!({
                "year": y, "school": w.programs[prog].real_name,
                "hc": hc.map(|c| w.coaches[c].name.clone()),
                "pc": pc.map(|c| w.coaches[c].name.clone()),
                "pc_role": Group::of(a.position).coach_title(),
            })
        })
        .collect();
    json!({
        "ethnicity": a.ethnicity.name(), "gpa": a.gpa_at_signing.map(|g| (g * 100.0).round() / 100.0),
        "team": a.nfl_team, "staff": staff,
        "positions": p.position_history.iter().map(|(age, pos)| json!({"age": age, "pos": pos.name()})).collect::<Vec<_>>(),
        "body": {"height": afm_engine::body::height_text(body.height), "weight": body.weight.round(),
            "arm": (body.arm * 8.0).round() / 8.0, "hand": (body.hand * 8.0).round() / 8.0,
            "wingspan": (body.wingspan * 8.0).round() / 8.0},
        "combine": {"forty": cmb.forty, "bench": cmb.bench, "vertical": cmb.vertical,
            "broad": cmb.broad, "cone": cmb.cone, "shuttle": cmb.shuttle},
        "tag": tag, "name": a.name, "pos": a.position.name(), "stars": a.stars,
        "hs": w.school_name(a.high_school), "state": STATE_CENTERS[a.home_state].0,
        "path": a.colleges.iter().map(|&c| format!("{} ({})", w.programs[c].real_name, w.programs[c].division.name())).collect::<Vec<_>>().join(" -> "),
        "pick": a.pick, "year": a.year, "class_of": a.class_of, "juco": a.juco,
        "round": a.pick.map(|pk| afm_engine::career::draft_round(pk, 257)),
        "overall": a.overall.round(), "potential": debug::alumnus_potential(a).round(),
        "visibility": (debug::alumnus_visibility(a) * 100.0).round() / 100.0,
        "background": a.background_summary, "notes": a.notes,
        "personality": {"work_ethic": p.personality.work_ethic.round(), "coachability": p.personality.coachability.round(),
            "competitiveness": p.personality.competitiveness.round(), "love_of_game": p.personality.love_of_game.round()},
        "bloom": (p.bloom * 10.0).round() / 10.0, "injury": p.injury_proneness.round(),
        "attributes": attrs, "seasons": seasons, "college_stats": college_stats(w, p.id)
    })
}

/// Match engine averages per team game for one set of world games.
pub fn engine_row(games: &[&GameSummary]) -> BTreeMap<&'static str, f64> {
    let mut m: BTreeMap<&'static str, f64> = BTreeMap::new();
    let n = games.len().max(1) as f64;
    let mut t = |k: &'static str, v: f64| *m.entry(k).or_default() += v;
    for g in games {
        let margin = (g.score[0] - g.score[1]).abs() as f64;
        t("games", 1.0);
        t("home_wins", (g.score[0] > g.score[1]) as u32 as f64);
        t("overtime", g.overtime as u32 as f64);
        t("blowouts", (margin >= 28.0) as u32 as f64);
        t("one_score", (margin <= 8.0) as u32 as f64);
        t("margin", margin);
        for s in 0..2 {
            let ts = &g.stats[s];
            t("points", g.score[s] as f64);
            t("plays", ts.plays as f64);
            t("dropbacks", ts.dropbacks as f64);
            t("att", ts.att as f64);
            t("cmp", ts.cmp as f64);
            t("pass_yds", ts.pass_yards);
            t("carries", ts.carries as f64);
            t("rush_yds", ts.rush_yards);
            t("sacks", ts.sacks as f64);
            t("ints", ts.ints as f64);
            t("third_att", ts.third_att as f64);
            t("third_conv", ts.third_conv as f64);
            t("turnovers", ts.turnovers as f64);
            t("penalties", ts.penalties as f64);
            t("explosive", ts.explosive as f64);
            t("punts", ts.punts as f64);
            t("fg_att", ts.fg_att as f64);
            t("fg_made", ts.fg_made as f64);
            t("kr", g.kr[s].0 as f64);
            t("kr_yds", g.kr[s].1);
            t("pr", g.pr[s].0 as f64);
            t("pr_yds", g.pr[s].1);
        }
    }
    let g = |k: &str| m.get(k).copied().unwrap_or(0.0);
    let tg = 2.0 * n;
    BTreeMap::from([
        ("games", n),
        ("points", r1(g("points") / tg)),
        ("plays", r1(g("plays") / tg)),
        ("pass_rate", r1(100.0 * g("dropbacks") / g("plays").max(1.0))),
        ("cmp_pct", r1(100.0 * g("cmp") / g("att").max(1.0))),
        ("ypa", (100.0 * g("pass_yds") / g("att").max(1.0)).round() / 100.0),
        ("ypc", (100.0 * g("rush_yds") / g("carries").max(1.0)).round() / 100.0),
        ("sack_rate", r1(100.0 * g("sacks") / g("dropbacks").max(1.0))),
        ("int_rate", r1(100.0 * g("ints") / g("att").max(1.0))),
        ("third_down", r1(100.0 * g("third_conv") / g("third_att").max(1.0))),
        ("turnovers", (100.0 * g("turnovers") / tg).round() / 100.0),
        ("penalties", r1(g("penalties") / tg)),
        ("explosive", r1(g("explosive") / tg)),
        ("punts", r1(g("punts") / tg)),
        ("fg_pct", r1(100.0 * g("fg_made") / g("fg_att").max(1.0))),
        ("kr_avg", r1(g("kr_yds") / g("kr").max(1.0))),
        ("pr_avg", r1(g("pr_yds") / g("pr").max(1.0))),
        ("home_win", r1(100.0 * g("home_wins") / n)),
        ("overtime", r1(100.0 * g("overtime") / n)),
        ("blowout", r1(100.0 * g("blowouts") / n)),
        ("one_score", r1(100.0 * g("one_score") / n)),
        ("margin", r1(g("margin") / n)),
    ])
}

const DIVS: [Division; 4] = [Division::FBS, Division::FCS, Division::D2, Division::JUCO];

/// What the game log adds to the dashboard: match engine averages by
/// division (all history and by season), champions, and FBS stat leaders.
pub fn extras(w: &World, years: u32) -> Value {
    let first = w.year - years as i32;
    let log = &w.game_log;
    let mut engine = serde_json::Map::new();
    for d in DIVS {
        let gs: Vec<&GameSummary> = log.games.iter().filter(|g| g.division == d).collect();
        let by_year: Vec<Value> = (first..w.year)
            .map(|y| {
                let ys: Vec<&GameSummary> = gs.iter().copied().filter(|g| g.year == y).collect();
                json!([y, engine_row(&ys)])
            })
            .collect();
        engine.insert(d.name().into(), json!({"all": engine_row(&gs), "by_year": by_year}));
    }
    let mut champs: BTreeMap<i32, BTreeMap<&str, String>> = BTreeMap::new();
    for p in &w.programs {
        for s in p.seasons.iter().filter(|s| s.national_title && s.year >= first) {
            champs
                .entry(s.year)
                .or_default()
                .insert(p.division.name(), format!("{} ({}-{})", p.real_name, s.wins, s.losses));
        }
    }
    let mut leaders: BTreeMap<i32, Value> = BTreeMap::new();
    for y in first..w.year {
        let rows: Vec<(u32, &PlayerSeason)> = log
            .players
            .iter()
            .filter(|((yy, _), s)| *yy == y && w.programs[s.program].division == Division::FBS)
            .map(|((_, id), s)| (*id, s))
            .collect();
        let top = |f: &dyn Fn(&PlayerSeason) -> f64| -> Vec<Value> {
            let mut v: Vec<(f64, u32, usize)> = rows.iter().map(|(id, s)| (f(s), *id, s.program)).collect();
            v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            v.iter()
                .take(5)
                .map(|&(val, id, prog)| {
                    let (name, stars, class) =
                        log.names.get(&id).map(|n| (n.0.as_str(), n.2, n.3)).unwrap_or(("?", 0, 0));
                    let pos = log.players[&(y, id)].pos.map_or("?", |p| p.name());
                    json!([name, pos, stars, class, w.programs[prog].real_name, r1(val)])
                })
                .collect()
        };
        leaders.insert(
            y,
            json!({
                "Passing yards": top(&|s| s.pass_yds as f64), "Passing TDs": top(&|s| s.pass_td as f64),
                "Rushing yards": top(&|s| s.rush_yds as f64), "Receiving yards": top(&|s| s.rec_yds as f64),
                "Sacks": top(&|s| s.sacks as f64), "Interceptions": top(&|s| s.ints as f64),
                "Tackles": top(&|s| s.tackles as f64),
            }),
        );
    }
    json!({"engine": engine, "champs": champs, "leaders": leaders, "games": log.games.len()})
}

/// The match engine's averages per team game in the world, by division.
pub fn print_engine(w: &World) {
    println!("## Match engine in the world (per team game, every college game of the history)");
    let keys = [
        "games",
        "points",
        "plays",
        "pass_rate",
        "cmp_pct",
        "ypa",
        "ypc",
        "sack_rate",
        "int_rate",
        "third_down",
        "turnovers",
        "kr_avg",
        "pr_avg",
        "fg_pct",
        "home_win",
        "blowout",
        "one_score",
    ];
    println!("| | {} |", keys.join(" | "));
    for d in DIVS {
        let gs: Vec<&GameSummary> = w.game_log.games.iter().filter(|g| g.division == d).collect();
        let row = engine_row(&gs);
        println!("| {} | {} |", d.name(), keys.iter().map(|k| row[k].to_string()).collect::<Vec<_>>().join(" | "));
    }
    println!();
}
