//! Lane commands: money (docs/final-build/plan.md). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! Lane 4 of the final build is the money side of the world (later extras
//! topic 1) as each seat sees it. An NFL head coach sees his club's public
//! numbers and the football budgets he spends; a college head coach sees
//! football's lines of the athletic department's budget and the
//! department's public report; a high school head coach sees the school's
//! football budget, the equipment budget and the booster club in full.
//! Any other club shows only what is published about it. The engine side
//! lives in `afm_engine::world::finance`.

use afm_engine::world::World;
use afm_engine::world::data::Division;
use afm_engine::world::finance::{self as fin, Owner, college, data, hs, nfl};
use serde_json::{Value, json};

use crate::views::{TeamRef, date_text, mine, team_brief};
use crate::{Game, Reply};

mod leagues;

pub(crate) use leagues::{career_rows as other_career_rows, team as league_team};

const COMMANDS: [&str; 7] =
    ["money_books", "money_league", "money_ask_facility", "money_hs_act", "money_card", "leagues", "league_team"];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_mut() else { return Some(Err("No world is loaded.".into())) };
    Some(match cmd {
        "money_books" => TeamRef::from_args(w, args).map(|t| books(w, t)),
        "money_league" => Ok(league(w, args)),
        "money_card" => TeamRef::from_args(w, args).and_then(|t| match t {
            TeamRef::Nfl(i) => Ok(card_json(w, i, true)),
            _ => Err("Report cards are the NFL players' union's.".into()),
        }),
        "money_ask_facility" => match mine(w) {
            Some(TeamRef::College(p)) => college::ask_facility(w, p).map(|text| json!({ "text": text })),
            _ => Err("Only a college head coach asks his athletic director for a building.".into()),
        },
        "money_hs_act" => hs_act(w, args),
        "leagues" => Ok(leagues::leagues(w, args)),
        "league_team" => match args.get("id").and_then(Value::as_u64) {
            Some(id) => leagues::team(w, id as usize),
            None => Err("Missing the team.".into()),
        },
        _ => Err("Unknown command.".into()),
    })
}

fn r0(x: f64) -> f64 {
    x.round()
}

/// A published estimate: rounded the way the business press rounds it.
fn est(x: f64) -> f64 {
    if x.abs() >= 1.0e9 {
        (x / 1.0e7).round() * 1.0e7
    } else if x.abs() >= 1.0e7 {
        (x / 1.0e6).round() * 1.0e6
    } else {
        (x / 1.0e4).round() * 1.0e4
    }
}

/// Lines of a book as rows: name, budget, spent so far (or last year's).
fn lines<const N: usize>(names: &[fin::Line], budget: &fin::Book<N>, now: &fin::Book<N>, keys: &[usize]) -> Vec<Value> {
    keys.iter()
        .map(|&k| json!({ "name": names[k].name, "income": names[k].income, "budget": r0(budget.l[k] as f64), "spent": r0(now.l[k] as f64) }))
        .collect()
}

fn books(w: &World, t: TeamRef) -> Value {
    let own = mine(w) == Some(t);
    let body = match t {
        TeamRef::Nfl(i) => nfl_books(w, i, own),
        TeamRef::College(p) => college_books(w, p, own),
        TeamRef::HighSchool(h) => hs_books(w, h, own),
        // The other leagues' clubs keep no books in the world (their money
        // is the players' pay on their pages and the coaches' salaries).
        TeamRef::Other(_) => Value::Null,
    };
    json!({ "team": team_brief(w, t), "own": own, "year": w.year, "today": date_text(w.today), "seat": seat_kind(t), "books": body })
}

fn seat_kind(t: TeamRef) -> &'static str {
    match t {
        TeamRef::Nfl(_) => "nfl",
        TeamRef::College(_) => "college",
        TeamRef::HighSchool(_) => "hs",
        TeamRef::Other(_) => "other",
    }
}

// ---------------------------------------------------------------------------
// The NFL
// ---------------------------------------------------------------------------

fn nfl_books(w: &World, t: usize, own: bool) -> Value {
    let Some(c) = w.finance.nfl.get(t) else { return Value::Null };
    let year = w.year;
    let last = c.years.last();
    let revenue = last.map_or(nfl::Club::revenue(&c.budget), nfl::Club::revenue);
    let costs = last.map_or(nfl::Club::costs(&c.budget), nfl::Club::costs);
    let ranked = |f: &dyn Fn(&nfl::Club) -> f64| {
        let me = f(c);
        1 + w
            .finance
            .nfl
            .iter()
            .enumerate()
            .filter(|(k, x)| *k != t && afm_engine::world::franchise::active(*k, year) && f(x) > me)
            .count()
    };
    let (city, state) = nfl::city(w, t, year);
    let history: Vec<Value> = c
        .years
        .iter()
        .zip(&c.seasons)
        .map(|(b, s)| {
            json!({
                "year": b.year, "revenue": est(nfl::Club::revenue(b)), "income": est(nfl::Club::revenue(b) - nfl::Club::costs(b)),
                "value": est(s.value as f64), "attendance": if s.home_games > 0 { s.attendance / s.home_games as u32 } else { 0 },
                "capacity": s.capacity, "sellouts": s.sellouts, "home_games": s.home_games, "blackouts": s.blackouts,
                "price": r0(s.price as f64), "renewal": s.renewal, "waitlist": s.waitlist,
            })
        })
        .collect();
    let gates: Vec<Value> = c
        .gates
        .iter()
        .map(|g| {
            json!({
                "date": date_text(g.date), "opp": team_brief(w, TeamRef::Nfl(g.opp as usize)), "attendance": g.attendance,
                "sold_out": g.sold_out, "blackout": g.blackout,
                "kind": if g.preseason { "Preseason" } else if g.playoff { "Playoff" } else { "Regular season" },
            })
        })
        .collect();
    let projects: Vec<Value> = projects(w, Owner::Nfl(t));
    let h = &c.home;
    let mut v = json!({
        "public": {
            "revenue": est(revenue), "income": est(revenue - costs), "value": est(c.value as f64),
            "value_rank": ranked(&|x: &nfl::Club| x.value as f64), "revenue_rank": ranked(&|x: &nfl::Club| x.years.last().map_or(0.0, nfl::Club::revenue)),
            "league_revenue": w.finance.league.last().map(|x| est(x.1 as f64)),
            "price": r0(c.price as f64), "season_tickets": c.season_tickets, "renewal": c.renewal, "waitlist": c.waitlist,
            "city": city, "state": state,
            "moving": c.moving.as_ref().map(|(city, st, y)| json!({ "city": city, "state": st, "year": y })),
        },
        "stadium": {
            "name": h.name, "capacity": h.capacity, "opened": h.opened, "renovated": h.renovated,
            "dome": h.dome, "cost": est(h.cost as f64), "public": est(h.public as f64), "age": h.age(year),
        },
        "owner": { "name": c.owner_name, "since": c.owner_since, "words": if own { Some(nfl::owner_words(w, t)) } else { None } },
        "history": history,
        "gates": gates,
        "projects": projects,
        "card": card_json(w, t, own),
    });
    if own {
        let keys =
            [nfl::PLAYERS, nfl::BENEFITS, nfl::COACHES, nfl::SCOUTING, nfl::MEDICAL, nfl::TRAVEL, nfl::AMENITIES];
        v["football"] = json!(lines(&nfl::LINES, &c.budget, &c.now, &keys));
        v["spend"] = json!(nfl::spend_level(w, t));
        v["cap"] = json!(afm_engine::world::pro::money::price_cap(year));
    }
    v
}

/// A club's latest NFLPA report card (P1): public in 2023 and 2024, the
/// union's own after; your own club's card reaches you either way.
fn card_json(w: &World, t: usize, own: bool) -> Value {
    let Some(latest) = nfl::card(w, t) else { return Value::Null };
    // Another club's: the last published card, with a word on the newer
    // ones the union keeps.
    let card = if own { Some(latest) } else { nfl::public_card(w, t) };
    let Some(card) = card else { return json!({ "year": latest.year, "hidden": true }) };
    let public = card.year <= data::CARDS_PUBLIC_TO;
    let n = w.finance.cards.iter().filter(|c| c.year == card.year).count();
    json!({
        "year": card.year, "rank": card.rank, "of": n, "public": public,
        "newer_hidden": (latest.year > card.year).then_some(latest.year),
        "grades": data::CARD_PARTS.iter().zip(card.grades).map(|(part, g)| json!({ "part": part, "grade": data::GRADES[g as usize], "score": g })).collect::<Vec<_>>(),
    })
}

fn projects(w: &World, owner: Owner) -> Vec<Value> {
    w.finance
        .projects
        .iter()
        .filter(|p| p.owner == owner)
        .rev()
        .take(12)
        .map(|p| {
            json!({
                "name": p.name, "kind": format!("{:?}", p.kind), "cost": est(p.cost as f64), "public": est(p.public as f64),
                "pledged": est(p.pledged as f64), "started": p.started, "opens": p.opens, "capacity": p.capacity,
                "done": p.done, "asked": p.asked,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// College
// ---------------------------------------------------------------------------

/// Departments at private schools don't publish their reports (the open
/// records laws that bring out public schools' don't reach them): only
/// the federal totals (EADA) are known.
fn private(w: &World, p: usize) -> bool {
    data::private_school(&w.programs[p].real_name, &w.programs[p].conference)
}

const TIERS: [&str; 4] = ["Power conference", "Group of Five", "FCS", "Division II and below"];

fn college_books(w: &World, p: usize, own: bool) -> Value {
    let Some(d) = w.finance.college.get(p) else { return Value::Null };
    let prog = &w.programs[p];
    let year = w.year;
    let last = d.years.last();
    let shown = |b: &fin::Book<{ college::N }>| {
        json!({
            "year": b.year, "revenue": est(college::Dept::revenue(b)), "expenses": est(college::Dept::costs(b)),
            "support": est((b.l[college::SUPPORT] + b.l[college::STUDENT_FEES]) as f64),
            "conference": est(b.l[college::CONFERENCE] as f64), "tickets": est(b.l[college::TICKETS] as f64),
            "donations": est((b.l[college::SEAT_DONATIONS] + b.l[college::GIFTS]) as f64),
            "ncaa": est(b.l[college::NCAA] as f64), "media_rights": est(b.l[college::LICENSING] as f64),
            "guarantees_in": est(b.l[college::GUARANTEES_IN] as f64),
            "coaches": est(b.l[college::COACHES] as f64), "scholarships": est(b.l[college::SCHOLARSHIPS] as f64),
            "rev_share": est(b.l[college::REV_SHARE] as f64), "debt": est(b.l[college::DEBT] as f64),
        })
    };
    let closed = !private(w, p) || own;
    let history: Vec<Value> = d
        .years
        .iter()
        .zip(&d.seasons)
        .map(|(b, s)| {
            let mut v = json!({
                "year": b.year, "revenue": est(college::Dept::revenue(b)), "expenses": est(college::Dept::costs(b)),
                "attendance": if s.home_games > 0 { s.attendance / s.home_games as u32 } else { 0 },
                "home_games": s.home_games, "sellouts": s.sellouts, "price": r0(s.price as f64),
                "season_tickets": s.season_tickets, "renewal": s.renewal,
            });
            if closed {
                v["support"] = json!(est((b.l[college::SUPPORT] + b.l[college::STUDENT_FEES]) as f64));
            }
            v
        })
        .collect();
    let deal = w.finance.deals.iter().find(|x| x.conference == prog.conference);
    let mut v = json!({
        "private": private(w, p),
        "tier": TIERS[college::tier(w, p, year).min(3) as usize],
        "conference": prog.conference,
        "deal": deal.map(|x| json!({ "per_school": est(x.per_school as f64), "until": x.until })),
        "media_value": est(college::media_worth(w, p, year)),
        "report": last.filter(|_| closed).map(shown),
        "eada": last.map(|b| json!({ "year": b.year, "revenue": est(college::Dept::revenue(b)), "expenses": est(college::Dept::costs(b)) })),
        "stadium": { "seats": prog.stadium, "crowd_seats": prog.crowd_seats() },
        "tickets": {
            "price": r0(d.price as f64), "season_tickets": d.season_tickets, "renewal": d.renewal,
            "priority": r0(d.priority as f64), "home_games": d.home_games, "sellouts": d.sellouts,
            "attendance": if d.home_games > 0 { d.attendance / d.home_games as u32 } else { 0 },
        },
        "history": history,
        "projects": projects(w, Owner::College(p)),
        "facilities": prog.facilities,
    });
    if own {
        let keys = [
            college::COACHES,
            college::STAFF,
            college::RECRUITING,
            college::SCHOLARSHIPS,
            college::REV_SHARE,
            college::TRAVEL,
            college::GUARANTEES_OUT,
            college::BOWL_TRIP,
            college::BUYOUTS,
        ];
        v["football"] = json!(lines(&college::LINES, &d.budget, &d.now, &keys));
        v["level"] = json!(d.level);
        v["asked"] = json!(d.asked == year);
        v["under_way"] = json!(w.finance.projects.iter().any(|x| x.owner == Owner::College(p) && !x.done));
        v["ad"] = json!(prog.ad.as_ref().map(|a| a.name.clone()));
    }
    v
}

// ---------------------------------------------------------------------------
// High school
// ---------------------------------------------------------------------------

fn hs_books(w: &World, h: usize, own: bool) -> Value {
    let Some(s) = w.finance.hs.get(h) else { return Value::Null };
    let school = &w.high_schools[h];
    let mut v = json!({
        "crowd": s.crowd, "fee": s.fee, "enrollment": school.enrollment, "private": school.private,
    });
    if own {
        let all: Vec<usize> = (0..hs::N).collect();
        v["budget"] = json!(lines(&hs::LINES, &s.budget, &s.now, &all));
        if s.last.year != 0 {
            let last: Vec<Value> = all
                .iter()
                .map(|&k| json!({ "name": hs::LINES[k].name, "income": hs::LINES[k].income, "amount": r0(s.last.l[k] as f64) }))
                .collect();
            v["last"] = json!({ "year": s.last.year, "lines": last });
        }
        v["seat"] = hs::seat_json(w).unwrap_or(Value::Null);
        // The booster club's account, kept on the front office's school desk.
        let b = &w.front_office.school.boosters;
        if w.front_office.school.school == Some(h) {
            v["boosters"] = json!({
                "balance": r0(b.balance), "members": b.members,
                "raised": r0(b.raised.iter().sum::<f64>() + b.fundraised), "fundraised": r0(b.fundraised),
                "spent": r0(b.spent.iter().sum::<f64>()),
            });
        }
        v["fundraisers"] = json!(
            data::FUNDRAISERS
                .iter()
                .map(|f| {
                    let take = hs::fundraiser_take(w, h, f.key).map(|(lo, hi)| [r0(lo), r0(hi)]);
                    json!({ "key": f.key, "name": f.name, "what": f.what, "weeks": f.weeks, "take": take })
                })
                .collect::<Vec<_>>()
        );
        v["purchases"] = json!(data::PURCHASES
            .iter()
            .map(|p| json!({ "key": p.key, "name": p.name, "what": p.what, "price": r0(p.price * data::hs_index(w.year)) }))
            .collect::<Vec<_>>());
    }
    v
}

fn hs_act(w: &mut World, args: &Value) -> Reply {
    let what = args.get("what").and_then(Value::as_str).unwrap_or("");
    let key = args.get("key").and_then(Value::as_str).unwrap_or("");
    let text = match what {
        "fundraiser" => hs::start_fundraiser(w, key)?,
        "buy" => hs::buy(w, key)?,
        "district" => {
            let (yes, text) = hs::ask_district(w)?;
            return Ok(json!({ "text": text, "yes": yes }));
        }
        _ => return Err("Unknown action.".into()),
    };
    Ok(json!({ "text": text, "yes": true }))
}

// ---------------------------------------------------------------------------
// The league's published numbers
// ---------------------------------------------------------------------------

fn league(w: &World, args: &Value) -> Value {
    let level = args.get("level").and_then(Value::as_str).unwrap_or(match mine(w) {
        Some(TeamRef::College(_)) => "college",
        _ => "nfl",
    });
    let year = w.year;
    if level == "college" {
        let rows: Vec<Value> = w
            .finance
            .college
            .iter()
            .enumerate()
            .filter(|(p, d)| {
                let prog = &w.programs[*p];
                matches!(prog.division, Division::FBS | Division::FCS) && prog.plays(year) && !d.years.is_empty()
            })
            .map(|(p, d)| {
                let b = d.years.last().unwrap();
                let s = d.seasons.last();
                let hidden = private(w, p) && mine(w) != Some(TeamRef::College(p));
                json!({
                    "team": team_brief(w, TeamRef::College(p)), "conference": w.programs[p].conference,
                    "division": if w.programs[p].division == Division::FBS { "FBS" } else { "FCS" },
                    "revenue": est(college::Dept::revenue(b)), "expenses": est(college::Dept::costs(b)),
                    "support": if hidden { Value::Null } else { json!(est((b.l[college::SUPPORT] + b.l[college::STUDENT_FEES]) as f64)) },
                    "attendance": s.map_or(0, |s| if s.home_games > 0 { s.attendance / s.home_games as u32 } else { 0 }),
                    "private": private(w, p),
                })
            })
            .collect();
        let deals: Vec<Value> = w
            .finance
            .deals
            .iter()
            .map(|d| json!({ "conference": d.conference, "per_school": est(d.per_school as f64), "until": d.until }))
            .collect();
        let moves: Vec<Value> = w
            .calendar
            .all()
            .filter(|e| e.rule == college::MOVE_RULE || college::officially(&e.text).is_some() && e.level == afm_engine::calendar::Level::College)
            .map(|e| json!({ "date": date_text(e.when.date), "text": e.text, "team": e.program.map(|p| team_brief(w, TeamRef::College(p))) }))
            .collect();
        return json!({ "level": "college", "year": year, "rows": rows, "deals": deals, "moves": moves });
    }
    let rows: Vec<Value> = w
        .finance
        .nfl
        .iter()
        .enumerate()
        .filter(|(t, c)| afm_engine::world::franchise::active(*t, year) && !c.years.is_empty())
        .map(|(t, c)| {
            let b = c.years.last().unwrap();
            let s = c.seasons.last();
            let card = nfl::public_card(w, t);
            json!({
                "team": team_brief(w, TeamRef::Nfl(t)), "value": est(c.value as f64),
                "revenue": est(nfl::Club::revenue(b)), "income": est(nfl::Club::revenue(b) - nfl::Club::costs(b)),
                "attendance": s.map_or(0, |s| if s.home_games > 0 { s.attendance / s.home_games as u32 } else { 0 }),
                "capacity": c.home.capacity, "stadium": c.home.name, "opened": c.home.opened,
                "owner": c.owner_name, "since": c.owner_since, "card": card.map(|x| x.rank),
                "blackouts": s.map_or(0, |s| s.blackouts),
            })
        })
        .collect();
    let league: Vec<Value> =
        w.finance.league.iter().map(|(y, v)| json!({ "year": y, "revenue": est(*v as f64) })).collect();
    let card_year = w.finance.cards.iter().map(|c| c.year).filter(|&y| y <= data::CARDS_PUBLIC_TO).max();
    json!({ "level": "nfl", "year": year, "rows": rows, "league": league, "card_year": card_year,
        "cards_internal": w.finance.cards.iter().any(|c| c.year > data::CARDS_PUBLIC_TO) })
}
