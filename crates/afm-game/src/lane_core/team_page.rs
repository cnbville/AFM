//! The team page's shared template (`docs/game.md`, "The team page"): the
//! key numbers, key players, strength by unit, the next four games, the
//! group's table, and each level's own middle (an NFL club's cap and
//! picks; a college's class, portal and pipeline). Every block reads what
//! the world already keeps; a block it cannot fill is left out.
//!
//! Reads judge as everywhere: your own read of a player first, your
//! staff's next, and for another team's man nobody of yours has seen, the
//! public read off film (`reads::public`).

use std::collections::HashMap;

use afm_engine::positions::Position;
use afm_engine::world::front_office::{college as desk, deals};
use afm_engine::world::norms::Rated;
use afm_engine::world::pro::{self, money};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::lane_squadboards::{self as sb, mid};
use crate::reads::{self, Read};
use crate::views::{self, TeamRef};

/// The units the page ranks, one per job (PR #374: tackles and the
/// interior, edge and tackle, corner and safety are different jobs).
const UNITS: [(&str, &[(Position, usize)]); 12] = {
    use Position::*;
    [
        ("QB", &[(QB, 1)]),
        ("RB", &[(RB, 1)]),
        ("WR", &[(WR, 3)]),
        ("TE", &[(TE, 1)]),
        ("OT", &[(OT, 2)]),
        ("IOL", &[(IOL, 3)]),
        ("EDGE", &[(EDGE, 2)]),
        ("DT", &[(DT, 2)]),
        ("LB", &[(LB, 3)]),
        ("CB", &[(CB, 2)]),
        ("S", &[(S, 2)]),
        ("ST", &[(K, 1), (P, 1), (LS, 1)]),
    ]
};

/// The positions a class's need bars show, one each, never lumped.
const NEED_POS: [Position; 11] = {
    use Position::*;
    [QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S]
};

/// The read that judges a player here, and whose it is.
fn judged(w: &World, k: usize, own: bool) -> (Read, &'static str) {
    match reads::mine(w, k) {
        r @ Read::Range(..) => (r, "yours"),
        Read::NotSeen => match reads::overall(w, k) {
            r @ Read::Range(..) => (r, "staff"),
            Read::NotSeen if !own => (reads::public(w, k), "film"),
            Read::NotSeen => (Read::NotSeen, "staff"),
        },
    }
}

/// A word for a share of the level's starters a man beats (proposed, not
/// confirmed).
fn level_word(pct: f64) -> &'static str {
    match pct {
        x if x >= 0.9 => "Elite",
        x if x >= 0.7 => "Strong",
        x if x >= 0.4 => "Solid",
        x if x >= 0.15 => "Starter",
        _ => "Depth",
    }
}

/// The three men a team leans on: the best reads on its roster, with
/// where each sits among the level's starters at his spot.
fn key_players(w: &World, t: TeamRef) -> Vec<Value> {
    let own = views::is_mine(w, t);
    let mut men: Vec<(f64, usize, Read, &'static str)> = views::roster_people(w, t)
        .into_iter()
        .filter(|&k| !w.people[k].player.position.is_specialist())
        .filter_map(|k| {
            let (r, whose) = judged(w, k, own);
            mid(r).map(|m| (m, k, r, whose))
        })
        .collect();
    men.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    men.into_iter()
        .take(3)
        .map(|(m, k, r, whose)| {
            let p = &w.people[k];
            let pos = p.player.position;
            let pct = sb::marks(w, t, pos, Rated::Overall).map(|mk| mk.below(m));
            let line = pct.map(|x| {
                let top = ((1.0 - x) * 100.0).round().max(1.0);
                if top <= 50.0 {
                    format!("In the top {top}% of {}.", sb::starters_words(w, t, pos))
                } else {
                    format!("Behind most of {}.", sb::starters_words(w, t, pos))
                }
            });
            // One fact a coach would add: a deal that runs out, a last year.
            let note = match t {
                TeamRef::Nfl(c) => p
                    .pro
                    .as_ref()
                    .and_then(|x| x.contract.as_ref())
                    .filter(|x| x.team == c && x.last_year() == ending_year(w))
                    .map(|_| {
                        if ending_year(w) < w.year {
                            "His contract runs out when the new league year opens.".to_string()
                        } else {
                            "His contract ends after this season.".to_string()
                        }
                    }),
                TeamRef::College(_) => (p.stage == Stage::College && views::class_text(w, p).ends_with("Sr"))
                    .then(|| "In his last season.".to_string()),
                _ => None,
            };
            json!({
                "id": p.player.id,
                "name": p.name,
                "pos": pos.name(),
                "age": p.age_on(w.today),
                "class": views::class_text(w, p),
                "read": r.json(),
                "value": m.round(),
                "whose": whose,
                "word": pct.map(level_word),
                "line": line,
                "note": note,
            })
        })
        .collect()
}

/// Each unit's starters against the level's (the bar) and its rank among
/// the teams the page is measured against.
fn units(w: &World, t: TeamRef) -> Value {
    let rows: Vec<Value> = UNITS.iter().map(|(name, spots)| sb::unit_card(w, t, name, spots)).collect();
    let among = rows.first().and_then(|r| r["among"].as_str()).unwrap_or("").to_string();
    json!({ "rows": rows, "among": among })
}

/// The division (NFL), conference (college) or group (high school) as it
/// stands, with each team's point margin from the scores played.
fn table(w: &World, t: TeamRef) -> Value {
    let row = |r: TeamRef, wn: u32, l: u32, ti: u32, conf: Option<(u32, u32)>, diff: Option<i64>| {
        json!({
            "team": views::team_brief(w, r), "this": r == t, "mine": views::is_mine(w, r),
            "w": wn, "l": l, "t": ti, "cw": conf.map(|c| c.0), "cl": conf.map(|c| c.1), "diff": diff,
        })
    };
    match t {
        TeamRef::Nfl(me) => {
            let Some((cf, dv)) = w.nfl_alignment() else { return Value::Null };
            let names =
                ["AFC East", "AFC North", "AFC South", "AFC West", "NFC East", "NFC North", "NFC South", "NFC West"];
            // As the standings page groups them.
            let in_div = |x: usize, d: usize| cf[x] * 4 + dv[x] % 4 == d || dv[x] == d;
            let Some(d) = (0..8).find(|&d| in_div(me, d)) else { return Value::Null };
            let teams: Vec<usize> = (0..cf.len()).filter(|&x| in_div(x, d)).collect();
            let order = w.nfl_order(&teams);
            let mut diff: HashMap<usize, i64> = HashMap::new();
            for f in w.nfl_fixtures(None) {
                if let Some([h, a]) = f.score.map(|s| [s[0] as i64, s[1] as i64]) {
                    *diff.entry(f.home).or_default() += h - a;
                    *diff.entry(f.away).or_default() += a - h;
                }
            }
            let rows: Vec<Value> = order
                .iter()
                .map(|&x| {
                    let (wn, l, ti) = w.nfl_record(x);
                    row(TeamRef::Nfl(x), wn, l, ti, None, Some(diff.get(&x).copied().unwrap_or(0)))
                })
                .collect();
            let place = order.iter().position(|&x| x == me).map(|k| k + 1);
            json!({ "name": names[d], "rows": rows, "place": place })
        }
        TeamRef::College(me) => {
            let p = &w.programs[me];
            let ids: Vec<usize> = w
                .programs
                .iter()
                .filter(|q| q.conference == p.conference && q.division == p.division && q.plays(w.year))
                .map(|q| q.id)
                .collect();
            if ids.len() < 2 {
                return Value::Null;
            }
            let teams = w.college_conference_table(&ids, false);
            let mut diff: HashMap<usize, i64> = HashMap::new();
            for f in w.college_fixtures(None) {
                if let Some([h, a]) = f.score.map(|s| [s[0] as i64, s[1] as i64]) {
                    *diff.entry(f.home).or_default() += h - a;
                    *diff.entry(f.away).or_default() += a - h;
                }
            }
            let rows: Vec<Value> = teams
                .iter()
                .map(|(x, r)| {
                    let mut v = row(
                        TeamRef::College(*x),
                        r.wins,
                        r.losses,
                        0,
                        Some((r.conf_wins, r.conf_losses)),
                        Some(diff.get(x).copied().unwrap_or(0)),
                    );
                    v["champ"] = json!(r.conf_title);
                    v
                })
                .collect();
            let place = teams.iter().position(|x| x.0 == me).map(|k| k + 1);
            let champ = teams.first().filter(|x| x.1.conf_title).map(|x| x.0 == me);
            json!({ "name": w.conference_name(&p.conference), "rows": rows, "place": place, "conf": true, "champ": champ })
        }
        TeamRef::HighSchool(h) => {
            let Some((name, rows)) = w.hs_standings(h) else { return Value::Null };
            let place = rows.iter().position(|x| x.0 == h).map(|k| k + 1);
            let rows: Vec<Value> =
                rows.iter().map(|&(s, wn, l)| row(TeamRef::HighSchool(s), wn, l, 0, None, None)).collect();
            json!({ "name": name, "rows": rows, "place": place })
        }
        TeamRef::Other(_) => Value::Null,
    }
}

/// The league year whose deals run out next: last year's until the new
/// league year opens in March, this year's after.
fn ending_year(w: &World) -> i32 {
    if w.today < w.dates.league_year.date { w.year - 1 } else { w.year }
}

/// An NFL club's cap this league year in shares, never dollars (Cain: no
/// dollar figures unless he asks), and the picks it holds in the next draft.
fn nfl_books(w: &World, c: usize) -> Value {
    let year = w.year;
    let cap = money::price_cap(year);
    if cap <= 0.0 || !money::capped(year) {
        return json!({ "capped": false, "picks": picks(w, c) });
    }
    let share = |x: f64| (100.0 * x / cap).round();
    let (mut off, mut def, mut st) = (0.0, 0.0, 0.0);
    let mut top: Option<(f64, usize)> = None;
    let mut ending = 0;
    for &i in &w.clubs[c].roster {
        let p = &w.people[i];
        let Some(k) = p.pro.as_ref().and_then(|x| x.contract.as_ref()).filter(|k| k.team == c) else { continue };
        let hit = k.cap_hit(year);
        match views::unit(p.player.position) {
            "OFF" => off += hit,
            "DEF" => def += hit,
            _ => st += hit,
        }
        if top.is_none_or(|t| hit > t.0) {
            top = Some((hit, i));
        }
        if k.last_year() == ending_year(w) {
            ending += 1;
        }
    }
    let dead = w.clubs[c].dead_in(year);
    let room = pro::cap_room(w, c, year);
    json!({
        "capped": true,
        "year": year,
        "off": share(off), "def": share(def), "st": share(st), "dead": share(dead),
        "room": share(room.max(0.0)), "over": room < 0.0,
        "top": top.map(|(hit, i)| json!({ "id": w.people[i].player.id, "name": w.people[i].name, "pos": w.people[i].player.position.name(), "share": share(hit) })),
        "ending": ending, "ending_now": ending_year(w) < year,
        "picks": picks(w, c),
    })
}

/// The picks a club holds in the next draft.
fn picks(w: &World, c: usize) -> Value {
    let all = deals::picks_of(w, c);
    let Some(first) = all.iter().map(|p| p.year).min() else { return json!({ "year": null, "list": [] }) };
    let mut list: Vec<&deals::Pick> = all.iter().filter(|p| p.year == first).collect();
    list.sort_by_key(|p| (p.round, p.comp, p.orig));
    json!({
        "year": first,
        "list": list.iter().map(|p| json!({
            "round": p.round, "comp": p.comp, "own": p.orig == c && !p.comp,
            "from": (p.orig != c).then(|| views::team_brief(w, TeamRef::Nfl(p.orig))),
            "text": deals::pick_words(w, p),
        })).collect::<Vec<_>>(),
    })
}

/// A college's class and portal: the class's need at each position with
/// its commitments, the services' class ranking, the portal's moves this
/// cycle, and where its recruits come from.
fn college_books(w: &World, j: usize) -> Value {
    let o = desk::outlook(w, j);
    let sign = o.sign_year;
    // Commitments and signees for the class, by position.
    let mut have = [0usize; afm_engine::positions::POSITION_COUNT];
    let mut total = 0;
    for p in &w.people {
        if p.class_of != sign {
            continue;
        }
        let here = match (p.cal.signed, p.cal.commit) {
            (Some(s), _) => s.program == j && !s.walk_on,
            (None, Some(c)) => c.0 == j && !c.2,
            _ => false,
        };
        if here {
            have[p.player.position.index()] += 1;
            total += 1;
        }
    }
    let needs: Vec<Value> = NEED_POS
        .iter()
        .filter(|pos| o.need[pos.index()] + have[pos.index()] > 0)
        .map(|pos| json!({ "pos": pos.name(), "need": o.need[pos.index()].max(have[pos.index()]), "have": have[pos.index()] }))
        .collect();
    let (rank, of, conf_rank, conf_of) = crate::lane_frontoffice::college::class_ranks(w, j, sign);
    // The portal this cycle: who left (or is shopping) and who came in. A
    // transfer is new until his first development step at the school, the
    // marker the spring depth chart reads too (`pending_dip`).
    let (mut out, mut came, mut out_starters) = (0, Vec::new(), 0);
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::College {
            continue;
        }
        let shopping = p.cal.portal.is_some() && p.college == Some(j);
        let left =
            p.pending_dip.is_some() && p.path.len() >= 2 && p.path[p.path.len() - 2] == j && p.college != Some(j);
        if shopping || left {
            out += 1;
            if p.last_role == Some(afm_engine::player::PlayingTime::Starter) {
                out_starters += 1;
            }
        }
        if p.pending_dip.is_some() && p.path.len() >= 2 && p.path.last() == Some(&j) && p.college == Some(j) {
            came.push(i);
        }
    }
    came.sort_by_key(|&i| std::cmp::Reverse(w.people[i].stars));
    // Where its recruits come from: its last four signed classes by home state.
    let mut by: HashMap<usize, u32> = HashMap::new();
    let mut n = 0;
    for p in &w.people {
        if p.class_of >= sign - 4 && p.class_of < sign && p.path.first() == Some(&j) {
            *by.entry(p.home_state).or_default() += 1;
            n += 1;
        }
    }
    let home = w.programs[j].state;
    let mut states: Vec<(usize, u32)> = by.into_iter().collect();
    states.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut shown: Vec<(usize, u32)> = states.iter().take(6).copied().collect();
    if !shown.iter().any(|x| x.0 == home)
        && let Some(h) = states.iter().find(|x| x.0 == home)
    {
        shown.push(*h);
    }
    json!({
        "sign_year": sign,
        "needs": needs,
        "committed": total,
        "places": o.slots,
        "rank": rank, "rank_of": of, "conf_rank": conf_rank, "conf_of": conf_of,
        "division": w.programs[j].division.name(),
        "portal": {
            "out": out, "out_starters": out_starters, "in": came.len(),
            "in_names": came.iter().take(3).map(|&i| json!({ "id": w.people[i].player.id, "name": w.people[i].name, "pos": w.people[i].player.position.name() })).collect::<Vec<_>>(),
        },
        "pipeline": {
            "classes": [sign - 4, sign - 1],
            "total": n,
            "rows": shown.iter().map(|&(s, c)| json!({
                "state": views::state_name(&w.states[s].code), "n": c, "home": s == home,
            })).collect::<Vec<_>>(),
        },
    })
}

/// The template's blocks for one team, added to `team_page`.
pub fn blocks(w: &World, t: TeamRef, v: &mut Value) {
    if matches!(t, TeamRef::Other(_)) {
        return;
    }
    v["key_players"] = json!(key_players(w, t));
    v["units"] = units(w, t);
    v["table"] = table(w, t);
    v["coming"] = json!(crate::lane_data::teams::coming_games(w, t, 4));
    match t {
        TeamRef::Nfl(c) => v["books"] = nfl_books(w, c),
        TeamRef::College(j) => v["books"] = college_books(w, j),
        TeamRef::HighSchool(h) => {
            let s = &w.high_schools[h];
            v["roster_count"] = json!(s.roster.len());
            v["alive"] = json!(w.hs_alive(h));
        }
        TeamRef::Other(_) => {}
    }
}
