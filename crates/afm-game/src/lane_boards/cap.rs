//! V14, where the money goes. In the NFL: every cap number on your books
//! for a league year as a block by position group, sized by the hit and
//! shaded by the years left, dead money and open space beside them, and a
//! player's deal with what cutting or trading him does. In college: the
//! NIL budget the same way (each player's deal for the money year, what's
//! promised to recruits and portal targets, what's left) and the
//! scholarships against the limit. A high school has neither.

use afm_engine::calendar::{Date, Level as CalLevel};
use afm_engine::positions::Position;
use afm_engine::world::front_office::college as desk;
use afm_engine::world::front_office::{Duty, Owner};
use afm_engine::world::pro::{self, money};
use afm_engine::world::{Stage, World, ncaa};
use serde_json::{Value, json};

use crate::Reply;
use crate::reads;
use crate::views::{self, TeamRef};

/// The position groups the blocks sit in, as a cap sheet groups them.
const GROUPS: [&str; 9] = ["QB", "EDGE and DT", "Offensive line", "WR", "CB", "LB", "S", "RB and TE", "Kicking"];

fn group(p: Position) -> usize {
    match p {
        Position::QB => 0,
        Position::EDGE | Position::DT => 1,
        Position::OT | Position::IOL => 2,
        Position::WR => 3,
        Position::CB => 4,
        Position::LB => 5,
        Position::S => 6,
        Position::RB | Position::TE => 7,
        Position::K | Position::P | Position::LS => 8,
    }
}

pub fn board(w: &World, args: &Value) -> Reply {
    match views::mine(w) {
        Some(TeamRef::Nfl(t)) => nfl(w, t, args),
        Some(TeamRef::College(j)) => college(w, j),
        Some(_) => Ok(json!({ "level": "none", "why": "A high school has no cap and no NIL budget to set." })),
        None => Err("Start a career first.".into()),
    }
}

/// Your staff's overall read of one of your players.
fn read_of(w: &World, i: usize, t: TeamRef) -> Value {
    let p = &w.people[i];
    let eye = views::eye_for(w, t, p.player.position);
    reads::overall(&p.player, true, eye, views::reader_key(w)).json()
}

fn nfl(w: &World, t: usize, args: &Value) -> Reply {
    let first = w.year;
    let year = args.get("year").and_then(Value::as_i64).map_or(first, |y| y as i32).clamp(first, first + 4);
    let today = w.today;
    let club = &w.clubs[t];
    let mut blocks: Vec<Value> = Vec::new();
    let mut ending: Vec<(f64, usize)> = Vec::new();
    // The biggest cap number at each position across the league this year,
    // for the bar under a player's hit.
    let mut top = [0.0f64; 14];
    for c in &w.clubs {
        for &i in &c.roster {
            let p = &w.people[i];
            if let Some(k) = p.pro.as_ref().and_then(|x| x.contract.as_ref()) {
                let h = k.cap_hit(year);
                let s = &mut top[p.player.position as usize];
                *s = s.max(h);
            }
        }
    }
    let cap = money::price_cap(year);
    let capped = money::capped(year);
    // The offseason rule of 51 (`pro::payroll`): until the first game only
    // the 51 biggest cap hits count. The 51st biggest, and how many at it
    // still count.
    let top51 = money::cap_books(year) && (year > w.year || (year == w.year && today < w.dates.nfl_kickoff));
    let mut hits: Vec<f64> = club
        .roster
        .iter()
        .filter_map(|&i| w.people[i].pro.as_ref()?.contract.as_ref())
        .filter(|k| k.team == t && k.last_year() >= year)
        .map(|k| k.cap_hit(year))
        .collect();
    hits.sort_by(|a, b| b.total_cmp(a));
    let (bar, mut at_bar) = match hits.get(50) {
        Some(&h) if top51 => (h, hits.iter().take(51).filter(|&&x| x == h).count()),
        _ => (f64::NEG_INFINITY, usize::MAX),
    };
    let mut order: Vec<usize> = club.roster.clone();
    order.sort_by(|&a, &b| {
        let hit =
            |i: usize| w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).map_or(0.0, |k| k.cap_hit(year));
        hit(b).total_cmp(&hit(a))
    });
    let mut outside = 0.0;
    for &i in &order {
        let p = &w.people[i];
        let Some(c) = p.pro.as_ref() else { continue };
        // A deal's void years past its last real year are no cap hit of
        // his: they fall on the books as dead money (below).
        let Some(k) = c.contract.as_ref().filter(|k| k.team == t && k.last_year() >= year) else { continue };
        let Some(y) = k.year(year) else { continue };
        let hit = y.cap_hit();
        if hit <= 0.0 {
            continue;
        }
        let left = (k.last_year() - year + 1).max(0);
        if k.last_year() == year {
            ending.push((hit, i));
        }
        let (dead_now, _) = k.dead_if_cut(year, today, false);
        let (dj, dj2) = k.dead_if_cut(year, today, true);
        // A trade leaves the bonus still to count here; the salary and its
        // guarantees go with him.
        let bonus_left: f64 = k.years.iter().filter(|x| x.year >= year).map(|x| x.proration).sum();
        let pos = p.player.position;
        let counts = hit > bar || (hit == bar && at_bar > 0);
        if hit == bar && at_bar > 0 {
            at_bar -= 1;
        }
        if !counts {
            outside += hit;
        }
        blocks.push(json!({
            "id": p.player.id,
            "counts": counts,
            "name": p.name,
            "pos": pos.name(),
            "group": group(pos),
            "age": p.player.age,
            "hit": hit.round(),
            "pct": (1000.0 * hit / cap).round() / 10.0,
            "left": left,
            "through": k.last_year(),
            "salary": y.base.round(),
            "bonus": y.proration.round(),
            "bonusShare": if hit > 0.0 { (100.0 * y.proration / hit).round() } else { 0.0 },
            "gtd": k.guaranteed_left(year, today).round(),
            "topAtPos": top[pos as usize].round(),
            "read": read_of(w, i, TeamRef::Nfl(t)),
            "kind": format!("{:?}", k.kind),
            "squad": c.status == pro::Status::PracticeSquad,
            "moves": [
                { "k": "Cut now", "saves": k.cut_savings(year, today, false).round(), "now": dead_now.round(), "later": 0.0 },
                { "k": "Cut after 1 June", "saves": k.cut_savings(year, today, true).round(), "now": dj.round(), "later": dj2.round() },
                { "k": "Trade now", "saves": (hit - bonus_left).round(), "now": bonus_left.round(), "later": 0.0, "noTrade": k.no_trade },
            ],
        }));
    }
    let mut dead: Vec<Value> = club
        .dead
        .iter()
        .filter(|d| d.year == year && d.amount > 0.0)
        .map(|d| {
            let at = views::person_index(w, d.player);
            json!({
                "id": at.map(|_| d.player),
                "name": at.map_or_else(|| "A former player".to_string(), |i| w.people[i].name.clone()),
                "amount": d.amount.round(),
                // An unlikely incentive he earned last season, charged now.
                "incentive": d.why == pro::DeadWhy::Incentives,
            })
        })
        .collect();
    // Last season's per-game bonuses not earned come back as a credit
    // (`market::settle_incentives`), one line for them all.
    let credit: f64 = club.dead.iter().filter(|d| d.year == year && d.amount < 0.0).map(|d| d.amount).sum();
    if credit < 0.0 {
        dead.push(json!({ "id": null, "name": "Unearned bonuses back", "amount": credit.round(), "credit": true }));
    }
    // A deal that ran out last year leaves its void years' bonus here.
    for &i in &club.roster {
        let p = &w.people[i];
        let Some(k) = p.pro.as_ref().and_then(|c| c.contract.as_ref()).filter(|k| k.team == t) else { continue };
        if k.last_year() + 1 != year {
            continue;
        }
        let v: f64 = k.years.iter().filter(|x| x.year >= year).map(|x| x.proration).sum();
        if v > 0.0 {
            dead.push(json!({ "id": p.player.id, "name": p.name, "amount": v.round(), "void": true }));
        }
    }
    let committed = pro::payroll(w, t, year);
    let room = pro::cap_room(w, t, year);
    // Before the draft, the rookies you will sign: each pick's slot price
    // in its first year (the middle of the round where the order isn't
    // set; proposed, not confirmed).
    let draft = w.dates.draft[0].date;
    let rookies: f64 = if year == first && today < draft {
        afm_engine::world::front_office::deals::picks_of(w, t)
            .iter()
            .filter(|p| p.year == year)
            .map(|p| {
                let slot = (p.round.max(1) - 1) * 32 + 16;
                money::rookie_terms(year, slot, p.round).apy
            })
            .sum()
    } else {
        0.0
    };
    ending.sort_by(|a, b| b.0.total_cmp(&a.0));
    let years: Vec<Value> = (first..first + 5)
        .map(|y| {
            let c = pro::payroll(w, t, y);
            let cap = money::price_cap(y);
            json!({ "year": y, "pct": if cap > 0.0 { (100.0 * c / cap).round() } else { 0.0 } })
        })
        .collect();
    Ok(json!({
        "level": "nfl",
        "team": views::team_brief(w, TeamRef::Nfl(t)),
        "year": year,
        "years": years,
        "capped": capped,
        // The team's own cap: the league's plus what it carried over (or,
        // with no cap, the owner's budget), so the space is this less
        // what's committed.
        "cap": (room + committed).round(),
        "leagueCap": cap.round(),
        "carry": if capped { club.carryover.round() } else { 0.0 },
        "committed": committed.round(),
        "space": room.round(),
        "rookies": rookies.round(),
        "afterRookies": (room - rookies).round(),
        "groups": GROUPS,
        "blocks": blocks,
        "top51": top51 && outside > 0.0,
        "outside": outside.round(),
        "dead": dead,
        "ending": {
            "year": year,
            "count": ending.len(),
            "sum": ending.iter().map(|x| x.0).sum::<f64>().round(),
            "top": ending.iter().take(4).map(|&(h, i)| {
                let p = &w.people[i];
                json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "age": p.player.age, "hit": h.round(), "read": read_of(w, i, TeamRef::Nfl(t)) })
            }).collect::<Vec<_>>(),
        },
    }))
}

fn college(w: &World, j: usize) -> Reply {
    let prog = &w.programs[j];
    let season = w.college_year().unwrap_or(w.year);
    let money_year = desk::deals_year(w);
    let era = ncaa::nil_era(w.today.year()) || w.today >= Date::ymd(ncaa::NIL_START, 6, 1);
    let b = desk::budget(w, j);
    let fo = &w.front_office.college;
    let set: std::collections::HashMap<u32, f64> =
        if fo.deals_year == money_year { fo.deals.iter().copied().collect() } else { Default::default() };
    let usual: std::collections::HashMap<u32, f64> = desk::usual_deals(w, j).into_iter().collect();
    // What 1 July pays: your deals when NIL is yours (a man without one
    // gets nothing), the collective's usual split when your staff runs it.
    // With NIL yours and nothing set, the blocks show the usual split as a
    // suggestion.
    let yours = w.front_office.owner(Duty::Nil) == Owner::You;
    let use_set = yours && !set.is_empty();
    let suggested = yours && set.is_empty();
    let mut to_roster = 0.0;
    let mut blocks: Vec<Value> = Vec::new();
    let mut on_aid = 0usize;
    let mut walk_ons = 0usize;
    let mut roster = 0usize;
    for &i in &prog.roster {
        let p = &w.people[i];
        if p.stage != Stage::College || p.college != Some(j) {
            continue;
        }
        roster += 1;
        if p.walk_on {
            walk_ons += 1;
        } else {
            on_aid += 1;
        }
        let id = p.player.id;
        // His deal this money year: the one set, else the collective's
        // usual split.
        let deal = if use_set { set.get(&id).copied().unwrap_or(0.0) } else { usual.get(&id).copied().unwrap_or(0.0) };
        if !era || deal <= 0.0 {
            continue;
        }
        to_roster += deal;
        let five = w.changes.rules(w.year).five_in_five;
        let left = (if five { 5u32 } else { 4 }).saturating_sub(p.college_seasons);
        blocks.push(json!({
            "id": id,
            "name": p.name,
            "pos": p.player.position.name(),
            "group": group(p.player.position),
            "cls": views::class_text(w, p),
            "hit": deal.round(),
            "pct": if b.total > 0.0 { (1000.0 * deal / b.total).round() / 10.0 } else { 0.0 },
            "left": left,
            "usual": suggested,
            "income": p.nil_income.round(),
            "read": read_of(w, i, TeamRef::College(j)),
            "walkOn": p.walk_on,
        }));
    }
    let (cap_now, signees) = ncaa::roster_limits(prog, season);
    let whole = ncaa::whole_roster_limit(prog, season);
    Ok(json!({
        "level": "college",
        "team": views::team_brief(w, TeamRef::College(j)),
        "era": era,
        "house": w.today.year() >= ncaa::HOUSE_START,
        "moneyYear": money_year,
        "groups": GROUPS,
        "blocks": blocks,
        "budget": {
            "total": b.total.round(), "revenue": b.revenue_share.round(), "collective": b.collective.round(),
            "roster": to_roster.round(), "recruits": b.recruits.round(), "portal": b.portal.round(),
            "left": (b.total - to_roster - b.recruits - b.portal).round(),
            "setAny": !set.is_empty(), "yours": yours, "usual": suggested,
        },
        "scholarships": {
            "season": season,
            "limit": cap_now,
            "signees": signees,
            "onAid": on_aid,
            "walkOns": walk_ons,
            "roster": roster,
            "whole": whole,
        },
        "level_name": match prog.division {
            afm_engine::world::data::Division::FBS => "FBS",
            afm_engine::world::data::Division::FCS => "FCS",
            _ => "college",
        },
        "seat": CalLevel::College.name(),
    }))
}
