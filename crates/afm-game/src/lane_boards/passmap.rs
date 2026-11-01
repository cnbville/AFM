//! V11, the pass map (a passer's Passing tab): every throw your film has
//! of him, where it went across the field and how deep, to whom, off what
//! pocket clock, under pressure or clean, against his level's own numbers.
//! Your own passer's throws come from your games; another team's passer
//! from the film your staff keeps of your coming opponents. Nobody else's
//! throws are kept, so their page says so.

use afm_engine::calendar::Level as CalLevel;
use afm_engine::game::stats::{PlayRecord, PlayType, expected_completion};
use afm_engine::world::World;
use afm_engine::world::charts::depth_bin;
use serde_json::{Value, json};

use super::{r1, season_of};
use crate::Reply;
use crate::views;

/// One throw as the board reads it.
struct Throw {
    season: i32,
    date: String,
    opp: String,
    game: Option<usize>,
    n: u32,
    q: u8,
    clock: f64,
    down: u8,
    togo: f64,
    air: f64,
    yac: f64,
    yards: f64,
    from_y: f64,
    to_y: f64,
    ok: bool,
    td: bool,
    int: bool,
    drop: bool,
    pressured: bool,
    ttt: f64,
    epa: f64,
    xcmp: f64,
    target: Option<u32>,
}

/// The board for a passer.
pub fn board(w: &World, id: u32) -> Reply {
    // A passer who has left football keeps his name in your games.
    let i = views::person_index(w, id);
    let (name, pos) = match i {
        Some(i) => (w.people[i].name.clone(), w.people[i].player.position.name().to_string()),
        None => w
            .seat_games()
            .iter()
            .find_map(|g| afm_engine::world::seat_names::gone(g, id))
            .map(|x| (x.name.clone(), x.pos.name().to_string()))
            .ok_or("No such player.")?,
    };
    let mine = views::mine(w);
    let mut throws: Vec<Throw> = Vec::new();
    // Throwaways, by season.
    let mut away: Vec<i32> = Vec::new();
    // Your games, and the days they were played (their film is the same).
    let mut seen: Vec<afm_engine::calendar::Date> = Vec::new();
    for (k, g) in w.seat_games().iter().enumerate() {
        let Some(side) = (0..2).find(|&s| g.stats.players[s].contains_key(&id)) else { continue };
        let season = season_of(g.level, g.date);
        for r in g.stats.plays.iter().filter(|r| r.passer == Some(id) && r.offense == side && !r.nullified) {
            if r.kind != PlayType::Pass {
                continue;
            }
            if !seen.contains(&g.date) {
                seen.push(g.date);
            }
            if r.throw_away {
                away.push(season);
                continue;
            }
            throws.push(of_play(r, season, views::date_text(g.date), g.names[1 - side].clone(), Some(k)));
        }
    }
    // Your coming opponents' film: always read, as it may be newer than
    // any game of yours he played in.
    {
        for t in w.gameday.tapes.iter().filter(|t| !seen.contains(&t.date)) {
            for s in t.snaps.iter().filter(|s| s.passer == Some(id) && !s.null) {
                let Some(th) = s.throw else { continue };
                if th.away {
                    away.push(season_of(t.level, t.date));
                    continue;
                }
                let opp = t.names[1 - s.off.min(1) as usize].clone();
                let air = th.air as f64;
                let sep = th.sep as f64 / 10.0;
                throws.push(Throw {
                    season: season_of(t.level, t.date),
                    date: views::date_text(t.date),
                    opp,
                    game: None,
                    n: 0,
                    q: s.q,
                    clock: s.clock as f64,
                    down: s.down,
                    togo: s.togo as f64,
                    air,
                    yac: th.yac as f64,
                    yards: s.yards as f64,
                    from_y: th.from as f64 / 2.0,
                    to_y: th.to as f64 / 2.0,
                    ok: s.complete,
                    td: s.td && s.complete,
                    int: s.turnover && !s.complete,
                    drop: th.drop,
                    pressured: s.pressured,
                    ttt: th.ttt as f64 / 10.0,
                    epa: s.epa as f64,
                    xcmp: expected_completion(air, sep, s.pressured),
                    target: s.target,
                });
            }
        }
    }
    // The newest season with throws, or the one asked for.
    let seasons: Vec<i32> = {
        let mut v: Vec<i32> = throws.iter().map(|t| t.season).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let season = seasons.last().copied();
    let throws: Vec<&Throw> = throws.iter().filter(|t| Some(t.season) == season).collect();
    let away = away.iter().filter(|&&s| Some(s) == season).count();
    // The games shown: yours and the film's.
    let count = |film: bool| {
        let mut v: Vec<(&str, &str)> =
            throws.iter().filter(|t| t.game.is_none() == film).map(|t| (t.date.as_str(), t.opp.as_str())).collect();
        v.sort_unstable();
        v.dedup();
        v.len()
    };
    let (seat_games, film_games) = (count(false), count(true));
    let games = seat_games + film_games;
    // A receiver who has left football keeps his name in your games.
    let gone = |r: u32| {
        w.seat_games().iter().find_map(|g| afm_engine::world::seat_names::gone(g, r)).map(|x| (x.name.clone(), x.pos))
    };
    // Who he threw to.
    let mut rec: Vec<u32> = throws.iter().filter_map(|t| t.target).collect();
    rec.sort_unstable();
    rec.dedup();
    let receivers: Vec<Value> = rec
        .iter()
        .map(|&r| {
            let (name, pos) = match views::person_index(w, r) {
                Some(j) => (w.people[j].name.clone(), w.people[j].player.position.name().to_string()),
                None => gone(r).map_or((format!("No. {r}"), String::new()), |(n, p)| (n, p.name().to_string())),
            };
            json!({ "id": r, "name": name, "pos": pos })
        })
        .collect();
    // His level's numbers this season (or last, early on).
    let (level, team) = i.map_or((None, None), |i| level_of(w, i));
    let base = level.map(|(lv, t)| {
        let (key, name) = super::level_key(w, lv, t);
        let now = super::season_now(w, lv);
        match w.level_tally(key, now) {
            Some((yr, t)) if t.throws > 0 => {
                let n = t.throws as f64;
                json!({
                    "name": name,
                    "season": yr,
                    "last": yr < now,
                    "adot": r1(t.air as f64 / n),
                    "ttt": (t.ttt as f64 / n * 100.0).round() / 100.0,
                    "depth": t.depth.iter().map(|&d| r1(100.0 * d as f64 / n)).collect::<Vec<_>>(),
                    "pressure": r1(100.0 * t.pressured as f64 / t.dropbacks.max(1) as f64),
                    "throws": t.throws,
                })
            }
            _ => json!({ "name": name, "none": true }),
        }
    });
    // Who on your staff speaks for the passing game: your coordinator (or
    // head coach) about your own passer, your defensive coordinator about
    // an opponent's.
    let own = team.is_some() && team == mine;
    let staff = mine.and_then(|t| speaker(w, t, own));
    let rows: Vec<Value> = throws
        .iter()
        .map(|t| {
            json!({
                "g": t.game, "n": t.n, "date": t.date, "opp": t.opp, "q": super::quarter(t.q), "clock": super::clock(t.clock),
                "d": t.down, "togo": t.togo.round(), "air": r1(t.air), "yac": r1(t.yac), "yds": r1(t.yards),
                "y0": r1(t.from_y), "y1": r1(t.to_y), "ok": t.ok, "td": t.td, "int": t.int, "drop": t.drop,
                "pr": t.pressured, "t": (t.ttt * 100.0).round() / 100.0, "epa": (t.epa * 100.0).round() / 100.0,
                "x": (t.xcmp * 1000.0).round() / 1000.0, "tgt": t.target, "bin": depth_bin(t.air),
            })
        })
        .collect();
    Ok(json!({
        "id": id,
        "name": name,
        "pos": pos,
        "team": team.map(|t| views::team_brief(w, t)),
        "mine": own,
        "season": season,
        "source": if throws.is_empty() {
            "none"
        } else if seat_games == 0 {
            "film"
        } else if film_games == 0 {
            "games"
        } else {
            "both"
        },
        "games": games,
        "seatGames": seat_games,
        "filmGames": film_games,
        "throwaways": away,
        "throws": rows,
        "receivers": receivers,
        "base": base,
        "staff": staff,
    }))
}

fn of_play(r: &PlayRecord, season: i32, date: String, opp: String, game: Option<usize>) -> Throw {
    let (from_y, to_y) = r.throw_y.map_or((26.65, 26.65), |(a, b)| (a as f64, b as f64));
    Throw {
        season,
        date,
        opp,
        game,
        n: r.n,
        q: r.quarter,
        clock: r.clock,
        down: r.down,
        togo: r.togo,
        air: r.air_yards,
        yac: r.yac,
        yards: r.yards,
        from_y,
        to_y,
        ok: r.complete,
        td: r.touchdown && r.complete,
        int: r.interception,
        drop: r.drop,
        pressured: r.pressured,
        ttt: r.time_to_throw,
        epa: r.epa,
        xcmp: expected_completion(r.air_yards, r.separation, r.pressured),
        target: r.target,
    }
}

/// His team now, by level, for his level's numbers.
fn level_of(w: &World, i: usize) -> (Option<(CalLevel, usize)>, Option<views::TeamRef>) {
    let t = views::team_of(w, &w.people[i]);
    let lv = t.map(|t| match t {
        views::TeamRef::Nfl(x) => (CalLevel::Nfl, x),
        views::TeamRef::College(x) => (CalLevel::College, x),
        views::TeamRef::HighSchool(x) => (CalLevel::HighSchool, x),
        views::TeamRef::Other(x) => (CalLevel::Other, x),
    });
    (lv, t)
}

/// The coach who talks about a passer: (name, role, id).
fn speaker(w: &World, t: views::TeamRef, offense: bool) -> Option<Value> {
    let (c, role) = match t {
        views::TeamRef::College(p) => {
            let pr = &w.programs[p];
            if offense {
                (pr.oc.or(pr.head_coach)?, if pr.oc.is_some() { "offensive coordinator" } else { "head coach" })
            } else {
                (pr.dc.or(pr.head_coach)?, if pr.dc.is_some() { "defensive coordinator" } else { "head coach" })
            }
        }
        views::TeamRef::Nfl(x) => {
            let tm = &w.nfl[x];
            if offense {
                (tm.oc.or(tm.head_coach)?, if tm.oc.is_some() { "offensive coordinator" } else { "head coach" })
            } else {
                (tm.dc.or(tm.head_coach)?, if tm.dc.is_some() { "defensive coordinator" } else { "head coach" })
            }
        }
        views::TeamRef::HighSchool(h) => (w.high_schools[h].coach?, "head coach"),
        views::TeamRef::Other(_) => return None,
    };
    // You, when you hold the job yourself.
    if w.career.as_ref().is_some_and(|k| k.coach == c) {
        return None;
    }
    let coach = w.coaches.get(c)?;
    Some(json!({ "id": c, "name": coach.name, "role": role }))
}
