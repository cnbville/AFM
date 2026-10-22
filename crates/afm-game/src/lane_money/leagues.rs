//! The other leagues on screen (later extras X7): each league's season
//! table, its playoffs and champions, its rule book, its teams and, for the
//! CFL, its draft; and each other-league club's own page: its staff, its
//! roster with the Canadians marked, its seasons and this year's games.

use afm_engine::world::World;
use afm_engine::world::pro::other::{self as o, CFL, LEAGUE_COUNT, Row};
use serde_json::{Value, json};

use crate::views::{date_text, other_brief};

/// The leagues that have a season in the world, the one asked for (the
/// CFL by default) and its season: the live table while it runs, else the
/// finished one.
pub(crate) fn leagues(w: &World, args: &Value) -> Value {
    let o_ = &w.others;
    let mut ks: Vec<u8> = (0..LEAGUE_COUNT as u8)
        .filter(|&k| {
            o::full(k, w.year) || o_.live.iter().any(|l| l.league == k) || o_.seasons.iter().any(|s| s.league == k)
        })
        .collect();
    // The ones playing now first, then by their last season.
    let last = |k: u8| {
        o_.seasons
            .iter()
            .filter(|s| s.league == k)
            .map(|s| s.year)
            .chain(o_.live.iter().filter(|l| l.league == k).map(|l| l.year))
            .max()
            .unwrap_or(0)
    };
    ks.sort_by_key(|&k| (!o::full(k, w.year), -last(k), k));
    let list: Vec<Value> = ks
        .iter()
        .map(|&k| {
            let years: Vec<i32> = o_.seasons.iter().filter(|s| s.league == k).map(|s| s.year).collect();
            let y = if o::full(k, w.year) { w.year } else { last(k) };
            json!({
                "k": k, "name": o::league_name(k, y), "short": o::league_short(k),
                "now": o::full(k, w.year), "from": years.iter().min(), "to": years.iter().max(),
            })
        })
        .collect();
    let k =
        args.get("league").and_then(Value::as_u64).map(|k| k as u8).filter(|k| ks.contains(k)).or(ks.first().copied());
    let Some(k) = k else {
        return json!({ "leagues": list, "league": Value::Null, "note": "No other league has played a season in this world yet." });
    };
    let mut years: Vec<i32> = o_.seasons.iter().filter(|s| s.league == k).map(|s| s.year).collect();
    years.extend(o_.live.iter().filter(|l| l.league == k && !l.done).map(|l| l.year));
    years.sort_unstable();
    years.dedup();
    years.reverse();
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| years.contains(y))
        .or(years.first().copied());
    let season = year.map(|y| season(w, k, y));
    let champions: Vec<Value> = o_
        .seasons
        .iter()
        .rev()
        .filter(|s| s.league == k)
        .map(|s| {
            json!({
                "year": s.year,
                "champion": s.champion.map(|f| other_brief_in(w, f as usize, s.year)),
                "runner_up": s.runner_up.map(|f| other_brief_in(w, f as usize, s.year)),
                "score": s.champion.map(|_| format!("{}-{}", s.final_score[0], s.final_score[1])),
                "note": s.note.to_string(),
            })
        })
        .collect();
    let ry = year.unwrap_or(w.year);
    let format = o::format(k, ry);
    let draft: Vec<Value> = if k == CFL {
        let n = o::teams_in(CFL, ry).len().max(1) as u16;
        o_.draft
            .iter()
            .filter(|d| d.year == ry)
            .map(|d| {
                // Still in football: a link to his page.
                let here = crate::views::person_index(w, d.id).is_some();
                json!({
                    "pick": d.pick, "round": (d.pick - 1) / n + 1,
                    "team": other_brief_in(w, d.team as usize, ry),
                    "player": { "id": here.then_some(d.id), "name": d.name, "pos": d.pos.name() },
                    "school": o::U_SPORTS.get(d.school as usize).copied().unwrap_or(""),
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let teams: Vec<Value> = o::teams_in(k, ry)
        .into_iter()
        .map(|fr| {
            let hc = o::head_coach(w, fr);
            json!({
                "team": other_brief_in(w, fr, ry),
                "division": o::def(fr, ry).map_or("", |d| o::division_name(k, ry, d.div)),
                "coach": hc.map(|c| json!({ "id": c, "name": w.coaches[c].name })),
                "titles": o_.teams.get(fr).map_or(0, |t| t.titles),
            })
        })
        .collect();
    json!({
        "leagues": list,
        "league": { "k": k, "name": o::league_name(k, ry), "short": o::league_short(k) },
        "years": years,
        "year": year,
        "season": season,
        "champions": champions,
        "draft": draft,
        "teams": teams,
        "rules": o::rules_words(k, ry),
        "format": format.map(|f| json!({
            "games": f.games, "start": date_text(f.start), "roster": f.roster, "title": f.title,
            "playoffs": match f.playoff {
                o::Playoff::Cfl => "Three from each division, a fourth crossing over on a better record: semi-finals, finals, the Grey Cup".to_string(),
                o::Playoff::Top(n) => format!("The best {n} by record, seeded, one game a round"),
                o::Playoff::PerDiv => "The best two of each division, then the winners".to_string(),
                o::Playoff::None => "The season stopped before its playoffs".to_string(),
            },
        })),
    })
}

/// A club as a link, named as it was in that season.
fn other_brief_in(w: &World, fr: usize, year: i32) -> Value {
    let mut b = other_brief(w, fr);
    b["name"] = json!(o::name(fr, year));
    b
}

fn row_json(w: &World, k: u8, year: i32, r: &Row) -> Value {
    json!({
        "team": other_brief_in(w, r.fr as usize, year),
        "division": o::def(r.fr as usize, year).map_or("", |d| o::division_name(k, year, d.div)),
        "w": r.w, "l": r.l, "t": r.t, "pf": r.pf, "pa": r.pa,
        "pct": (r.pct() * 1000.0).round() / 1000.0,
        "pts": (k == CFL).then(|| r.points()),
    })
}

/// One league season: the table (each division's when it has them), the
/// games played, and how it ended.
fn season(w: &World, k: u8, year: i32) -> Value {
    let live = w.others.live.iter().find(|l| l.league == k && l.year == year && !l.done);
    let done = w.others.seasons.iter().find(|s| s.league == k && s.year == year);
    let (table, games, status) = match (live, done) {
        (Some(l), _) => {
            let reg = l.weeks.len();
            let status = if l.round > 0 {
                "In the playoffs".to_string()
            } else if l.week == 0 {
                format!(
                    "Camp is open; the first game is {}",
                    o::format(k, year).map_or(String::new(), |f| f.start.pretty())
                )
            } else {
                format!("After week {} of {reg}", l.week)
            };
            (l.table.clone(), l.games.clone(), status)
        }
        (None, Some(s)) => {
            let status = match (s.champion, s.note.is_empty()) {
                (Some(c), _) => {
                    format!("{} won the {}", o::name(c as usize, year), o::format(k, year).map_or("title", |f| f.title))
                }
                (None, false) => format!("No champion: {}", s.note),
                (None, true) => "Finished".to_string(),
            };
            (s.table.clone(), s.games.clone(), status)
        }
        _ => return Value::Null,
    };
    let mut rows = table;
    rows.sort_by(|a, b| {
        let key = |r: &Row| if k == CFL { r.points() as f64 } else { r.pct() };
        key(b).total_cmp(&key(a)).then((b.pf as i32 - b.pa as i32).cmp(&(a.pf as i32 - a.pa as i32)))
    });
    let mut weeks: Vec<Value> = Vec::new();
    let mut by_week: std::collections::BTreeMap<(u8, u8), Vec<Value>> = std::collections::BTreeMap::new();
    for g in &games {
        by_week.entry((g.round, g.week)).or_default().push(json!({
            "home": other_brief_in(w, g.home as usize, year), "away": other_brief_in(w, g.away as usize, year),
            "score": g.score, "round": g.round,
        }));
    }
    let rounds = games.iter().map(|g| g.round).max().unwrap_or(0);
    for ((round, week), gs) in by_week.into_iter().rev() {
        let label = if round == 0 {
            format!("Week {}", week + 1)
        } else if round == rounds
            && w.others.seasons.iter().any(|s| s.league == k && s.year == year && s.champion.is_some())
        {
            o::format(k, year).map_or("The final".to_string(), |f| f.title.to_string())
        } else {
            format!("Playoffs, round {round}")
        };
        weeks.push(json!({ "label": label, "games": gs }));
    }
    json!({
        "year": year,
        "status": status,
        "table": rows.iter().map(|r| row_json(w, k, year, r)).collect::<Vec<_>>(),
        "weeks": weeks,
        "games_kept": !games.is_empty(),
    })
}

/// An other-league club's page.
pub(crate) fn team(w: &World, fr: usize) -> Result<Value, String> {
    if fr >= o::FRANCHISES {
        return Err("No such team.".into());
    }
    let k = o::league_of(fr);
    let year = w.year;
    let t = w.others.team(fr).cloned().unwrap_or_default();
    let playing = o::def(fr, year).is_some();
    let staff: Vec<Value> = [
        (afm_engine::world::coach::Role::HeadCoach, "Head coach"),
        (afm_engine::world::coach::Role::OffensiveCoordinator, "Offensive coordinator"),
        (afm_engine::world::coach::Role::DefensiveCoordinator, "Defensive coordinator"),
    ]
    .into_iter()
    .filter_map(|(r, label)| {
        let c = o::slot(w, fr, r)?;
        let co = &w.coaches[c];
        Some(json!({ "role": label, "id": c, "name": co.name, "age": co.age_on(w.today) }))
    })
    .collect();
    let mut roster: Vec<(usize, Value)> = t
        .roster
        .iter()
        .map(|&i| {
            let p = &w.people[i as usize];
            let id = p.player.id;
            let pos = p.player.position;
            let line = p
                .pro
                .as_ref()
                .and_then(|c| c.other.iter().rev().find(|s| s.team as usize == fr && s.year == year))
                .map(|s| s.line(pos));
            (
                pos as usize,
                json!({
                    "id": id, "name": p.name, "pos": pos.name(), "age": p.age_on(w.today),
                    "national": k == CFL && w.others.national(id),
                    "college": p.college.map(|c| w.program_name(c).to_string()),
                    "nfl_years": p.pro.as_ref().map_or(0, |c| c.accrued),
                    "line": line,
                }),
            )
        })
        .collect();
    roster.sort_by_key(|r| r.0);
    let nationals = roster.iter().filter(|r| r.1["national"] == json!(true)).count();
    let seasons: Vec<Value> = o::team_seasons(w, fr)
        .into_iter()
        .rev()
        .map(|(y, wi, l, note)| json!({ "year": y, "record": format!("{wi}-{l}"), "note": note, "league": o::league_short(o::league_of(fr)) }))
        .collect();
    let games: Vec<Value> = w
        .others
        .live
        .iter()
        .filter(|l| l.league == k && l.year == year)
        .flat_map(|l| l.games.iter())
        .filter(|g| g.home as usize == fr || g.away as usize == fr)
        .map(|g| {
            let home = g.home as usize == fr;
            let (us, them) = if home { (g.score[0], g.score[1]) } else { (g.score[1], g.score[0]) };
            json!({
                "week": g.week + 1, "round": g.round, "home": home,
                "opp": other_brief(w, if home { g.away } else { g.home } as usize),
                "result": format!("{} {us}-{them}", if us > them { "W" } else if us < them { "L" } else { "T" }),
            })
        })
        .collect();
    let upcoming: Vec<Value> = o::fixtures(w, fr)
        .into_iter()
        .filter(|f| f.0 >= w.today)
        .take(4)
        .map(|(d, opp, home)| json!({ "date": date_text(d), "opp": other_brief(w, opp), "home": home }))
        .collect();
    let row = w
        .others
        .live
        .iter()
        .rev()
        .find(|l| l.league == k && l.year <= year)
        .and_then(|l| l.table.iter().find(|r| r.fr as usize == fr).map(|r| (l.year, *r)));
    let record = row.map(|(y, r)| {
        let rec = if r.t > 0 { format!("{}-{}-{}", r.w, r.l, r.t) } else { format!("{}-{}", r.w, r.l) };
        json!({ "year": y, "text": rec, "pf": r.pf, "pa": r.pa })
    });
    let mut brief = other_brief(w, fr);
    brief["ref"] = json!({ "kind": "other", "id": fr });
    Ok(json!({
        "team": brief,
        "league": { "k": k, "name": o::league_name(k, year), "short": o::league_short(k) },
        "city": o::city(fr, year),
        "division": o::def(fr, year).map_or("", |d| o::division_name(k, year, d.div)),
        "playing": playing,
        "titles": t.titles,
        "staff": staff,
        "roster": roster.into_iter().map(|r| r.1).collect::<Vec<_>>(),
        "nationals": nationals,
        "seasons": seasons,
        "games": games,
        "upcoming": upcoming,
        "record": record,
        "pay": o::salary(fr, afm_engine::world::coach::Role::HeadCoach, year).round(),
    }))
}

/// A player's seasons in other leagues, for his page's career table.
pub(crate) fn career_rows(w: &World, i: usize) -> Vec<Value> {
    let p = &w.people[i];
    let Some(c) = &p.pro else { return Vec::new() };
    c.other
        .iter()
        .map(|s| {
            json!({
                "year": s.year, "level": o::league_short(s.league),
                "team": other_brief_in(w, s.team as usize, s.year),
                "pos": p.player.position.name(), "games": s.games, "starts": s.starts,
                "line": s.line(p.player.position),
                "pay": s.pay.round(),
            })
        })
        .collect()
}
