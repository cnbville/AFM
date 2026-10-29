//! Front office: the NFL desk's commands (lane 10). The front office home
//! (who handles what, the deadlines ahead, the log), the cap sheet by year,
//! your expiring players with tags and tenders, talks with agents, free
//! agency, the trade desk, the cut to 53, the practice squad and the wire.
//! Every number shown is the league's or your staff's read; the player's
//! hidden ideal and true ratings never leave the engine.

use afm_engine::positions::Position;
use afm_engine::world::front_office::{self as fo, Duty, Owner, deals, nfl, roster};
use afm_engine::world::pro::{self, Status, money};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::views::{self, TeamRef};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let r = match cmd {
        "fo_office" => game.world.as_ref().map(office),
        "fo_set_owner" => game.world.as_mut().map(|w| set_owner(w, args)),
        "fo_cap" => game.world.as_ref().map(cap_sheet),
        "fo_expiring" => game.world.as_ref().map(expiring),
        "fo_talk" => game.world.as_ref().map(|w| talk(w, id(args)?)),
        "fo_offer" => game.world.as_mut().map(|w| offer(w, args)),
        "fo_preview" => game.world.as_ref().map(|w| preview(w, args)),
        "fo_tag" => game.world.as_mut().map(|w| {
            let what = args.get("kind").and_then(Value::as_str).unwrap_or("franchise");
            nfl::tag(w, id(args)?, what).map(|t| json!({ "text": t }))
        }),
        "fo_tender" => game.world.as_mut().map(|w| {
            let level = args.get("level").and_then(Value::as_str).unwrap_or("original");
            nfl::tender(w, id(args)?, level).map(|t| json!({ "text": t }))
        }),
        "fo_rescind" => game.world.as_mut().map(|w| nfl::rescind(w, id(args)?).map(|t| json!({ "text": t }))),
        "fo_option" => game
            .world
            .as_mut()
            .map(|w| nfl::set_option(w, id(args)?, flag(args, "take")).map(|_| json!({ "text": "Noted." }))),
        "fo_sheet" => game
            .world
            .as_mut()
            .map(|w| nfl::set_sheet(w, id(args)?, flag(args, "match")).map(|_| json!({ "text": "Noted." }))),
        "fo_free_agents" => game.world.as_ref().map(|w| free_agents(w, args)),
        "fo_visit" => game.world.as_mut().map(|w| nfl::visit(w, id(args)?).map(|t| json!({ "text": t }))),
        "fo_restructure" => game.world.as_mut().map(|w| {
            let voids = args.get("voids").and_then(Value::as_u64).unwrap_or(0) as u32;
            nfl::restructure(w, id(args)?, voids).map(|t| json!({ "text": t }))
        }),
        "fo_release" => {
            game.world.as_mut().map(|w| nfl::release(w, id(args)?, flag(args, "june1")).map(|t| json!({ "text": t })))
        }
        "fo_trades" => game.world.as_ref().map(trade_desk),
        "fo_trade_team" => game.world.as_ref().map(|w| trade_team(w, args)),
        "fo_trade_eval" | "fo_trade_propose" => game.world.as_mut().map(|w| {
            let p = proposal(w, args)?;
            let a = if cmd == "fo_trade_eval" { deals::evaluate(w, &p)? } else { deals::propose(w, &p)? };
            Ok(answer_json(w, &a))
        }),
        "fo_shop" => game.world.as_mut().map(|w| {
            let list = deals::shop(w, id(args)?)?;
            Ok(json!({ "offers": list.iter().map(|(o, picks, v)| json!({
                "team": views::team_brief(w, TeamRef::Nfl(*o)),
                "picks": picks.iter().map(|p| pick_json(w, p)).collect::<Vec<_>>(),
                "points": v.round(),
            })).collect::<Vec<_>>() }))
        }),
        "fo_block" => game.world.as_mut().map(|w| {
            deals::set_block(w, id(args)?, flag(args, "on"));
            Ok(json!({ "ok": true }))
        }),
        "fo_call" => game.world.as_mut().map(|w| {
            let k = args.get("k").and_then(Value::as_u64).ok_or("Missing call.")? as usize;
            deals::answer_call(w, k, flag(args, "yes")).map(|t| json!({ "text": t }))
        }),
        "fo_cutdown" => game.world.as_ref().map(cutdown),
        "fo_plan" => game.world.as_mut().map(|w| {
            let cut = match args.get("cut").and_then(Value::as_str) {
                Some("keep") => Some(roster::Cut::Keep),
                Some("squad") => Some(roster::Cut::Squad),
                Some("release") => Some(roster::Cut::Release),
                _ => None,
            };
            roster::set_plan(w, id(args)?, cut).map(|_| json!({ "ok": true }))
        }),
        "fo_squad" => game.world.as_ref().map(practice_squad),
        "fo_protect" => game
            .world
            .as_mut()
            .map(|w| roster::set_protect(w, id(args)?, flag(args, "on")).map(|_| json!({ "ok": true }))),
        "fo_sign_squad" => {
            game.world.as_mut().map(|w| roster::sign_to_squad(w, id(args)?).map(|t| json!({ "text": t })))
        }
        "fo_sign_up" => game.world.as_mut().map(|w| roster::sign_up(w, id(args)?).map(|t| json!({ "text": t }))),
        "fo_wire" => game.world.as_ref().map(wire),
        "fo_claim" => game.world.as_mut().map(|w| {
            let drop = args.get("drop").and_then(Value::as_u64).map(|x| x as u32);
            roster::claim(w, id(args)?, drop, flag(args, "on")).map(|_| json!({ "ok": true }))
        }),
        _ => return None,
    };
    Some(r.unwrap_or_else(|| Err("No world is loaded.".into())))
}

fn id(args: &Value) -> Result<u32, String> {
    args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".into())
}

fn flag(args: &Value, k: &str) -> bool {
    args.get(k).and_then(Value::as_bool).unwrap_or(false)
}

fn my_club(w: &World) -> Result<usize, String> {
    match views::mine(w) {
        Some(TeamRef::Nfl(t)) => Ok(t),
        _ => Err("You don't run an NFL club.".into()),
    }
}

fn money_json(x: f64) -> Value {
    json!(x.round())
}

/// A player's row with your staff's read (your own players) or the
/// league's public read (everyone else's).
fn prow(w: &World, i: usize) -> Value {
    let t = views::mine(w).unwrap_or(TeamRef::Nfl(0));
    let own = matches!(t, TeamRef::Nfl(c) if w.people[i].pro.as_ref().and_then(|p| p.team) == Some(c));
    let mut v = views::row(w, i, t, own, views::reader_key(w));
    if let Some(c) = w.people[i].pro.as_ref() {
        v["accrued"] = json!(c.accrued);
        v["team"] = c.team.map_or(Value::Null, |o| views::team_brief(w, TeamRef::Nfl(o)));
        v["out"] = json!(c.out_weeks);
    }
    v
}

/// What a restructure or a release of your player would do today: the
/// numbers the cap sheet's Moves dialog shows, for the player page and the
/// menu (minor bug 62). None when he isn't under contract with you.
pub(crate) fn cap_moves(w: &World, i: usize) -> Option<Value> {
    if !w.seat_head() {
        return None;
    }
    let t = my_club(w).ok()?;
    let c = w.people[i].pro.as_ref()?;
    let k = c.contract.as_ref().filter(|k| k.team == t && c.team == Some(t))?;
    if c.status == Status::PracticeSquad {
        return None;
    }
    let year = w.year;
    let today = w.today;
    let (dn, _) = k.dead_if_cut(year, today, false);
    let (dj, dj2) = k.dead_if_cut(year, today, true);
    let after = pro::after_june1(w, today);
    let used = w.front_office.nfl.june1.iter().filter(|x| x.1 == year).count();
    // A deal in its last year has no years to spread a bonus over without
    // void years (2016 on).
    let last = k.last_year() <= year;
    Some(json!({
        "re": money::capped(year)
            && k.year(year).is_some_and(|x| x.base > money::minimum(year, c.accrued) + 100_000.0)
            && (!last || nfl::void_years_allowed(year)),
        "last": last,
        "voids": nfl::void_years_allowed(year),
        "dead": dn.round(), "saves": k.cut_savings(year, today, false).round(),
        "dj": dj.round(), "dj2": dj2.round(), "sj": k.cut_savings(year, today, true).round(),
        "june1": if after { 0 } else { 2usize.saturating_sub(used) },
    }))
}

// --- The front office home ---------------------------------------------------------------------

fn office(w: &World) -> Reply {
    let seat = w.seat.ok_or("No career.")?;
    let level = seat.level;
    let duties: Vec<Value> = Duty::ALL
        .iter()
        .filter(|d| d.level() == level)
        .map(|&d| {
            json!({
                "key": d.key(), "name": d.name(),
                "owner": if w.front_office.owner(d) == Owner::You { "you" } else { "staff" },
                "staff": d.staff_does(),
            })
        })
        .collect();
    let today = w.today;
    let y = &w.dates;
    let dates: Vec<(afm_engine::calendar::Date, &str, &str)> = match level {
        afm_engine::calendar::Level::Nfl => vec![
            (y.tag_window_open, "The tag window opens", "fo_expiring"),
            (y.tag_deadline.date, "Franchise and transition tag deadline", "fo_expiring"),
            (y.tampering_open.map_or(y.league_year.date, |m| m.date), "The negotiating window opens", "fo_market"),
            (y.league_year.date, "The new league year: free agency and trades open", "fo_market"),
            (y.rfa_deadline, "Offer sheet deadline for restricted free agents", "fo_market"),
            (y.draft[0].date, "The draft", "calendar"),
            (y.fifth_year_option, "Fifth-year option deadline", "fo_expiring"),
            (y.june_1, "June 1: releases split their dead money", "fo_cap"),
            (y.tag_long_term_deadline.date, "Deadline for tagged players' long-term deals", "fo_expiring"),
            (y.cutdown.date, "Cutdown to 53", "fo_moves"),
            (y.trade_deadline.date, "The trade deadline", "fo_trades"),
        ],
        _ => Vec::new(),
    };
    let mut ahead: Vec<Value> = dates
        .into_iter()
        .filter(|x| x.0 >= today)
        .map(|(d, text, screen)| json!({ "date": views::date_text(d), "in": today.days_to(d), "text": text, "screen": screen }))
        .collect();
    ahead.sort_by_key(|v| v["in"].as_i64());
    let log: Vec<Value> = w
        .front_office
        .log
        .iter()
        .rev()
        .take(40)
        .map(|m| json!({ "date": views::date_text(m.date), "text": m.text, "id": m.player }))
        .collect();
    let mut out = json!({ "duties": duties, "ahead": ahead, "log": log, "level": format!("{level:?}") });
    if let Ok(t) = my_club(w) {
        let year = w.year;
        let carry = w.clubs[t].carryover;
        let cap = money::price_cap(year);
        out["cap"] = json!({
            "cap": money_json(cap), "room": money_json(pro::cap_room(w, t, year)),
            "carryover": money_json(carry), "dead": money_json(w.clubs[t].dead_in(year)),
            "next_room": money_json(pro::cap_room(w, t, year + 1)),
            "reserve": money_json(pro::season_reserve(w, year)),
            "voids": money_json(pro::voids_ahead(w, t)),
        });
        out["roster"] = json!({
            "on": roster::count(w, t, Status::Roster), "limit": roster::roster_limit(w),
            "squad": roster::count(w, t, Status::PracticeSquad), "squad_limit": w.rules.nfl_practice_squad,
            "reserve": roster::count(w, t, Status::Reserve),
        });
        out["expiring"] = json!(nfl::expiring_count(w, t));
        out["calls"] = json!(w.front_office.trades.calls.iter().filter(|c| !c.answered).count());
        out["talks"] = json!(
            w.front_office
                .nfl
                .talks
                .iter()
                .filter(|x| matches!(x.state, nfl::TalkState::Open | nfl::TalkState::Standing | nfl::TalkState::Agreed))
                .count()
        );
        out["gm"] = json!({ "negotiating": nfl::negotiating(w).round() });
        out["team"] = views::team_brief(w, TeamRef::Nfl(t));
    }
    Ok(out)
}

fn set_owner(w: &mut World, args: &Value) -> Reply {
    let d = args.get("duty").and_then(Value::as_str).and_then(Duty::from_key).ok_or("No such duty.")?;
    let o = if args.get("owner").and_then(Value::as_str) == Some("staff") { Owner::Staff } else { Owner::You };
    w.front_office.set_owner(d, o);
    office(w)
}

// --- The cap sheet -----------------------------------------------------------------------------

/// The cap sheet by year: every contract's cap number for five league
/// years, what cutting him saves now (and after June 1), dead money by
/// charge, the rule of 51 before the season, carryover and room by year,
/// and the picks you own.
fn cap_sheet(w: &World) -> Reply {
    let t = my_club(w)?;
    let year = w.year;
    let today = w.today;
    let years: Vec<i32> = (year..year + 5).collect();
    let mut rows = Vec::new();
    for &i in &w.clubs[t].roster {
        let p = &w.people[i];
        let Some(c) = p.pro.as_ref() else { continue };
        let Some(k) = c.contract.as_ref().filter(|k| k.team == t) else { continue };
        let mut v = prow(w, i);
        v["hits"] = json!(
            years
                .iter()
                .map(|&y| {
                    k.year(y).map(|x| json!({
            "hit": x.cap_hit().round(), "base": x.base.round(), "bonus": x.proration.round(),
            "roster": (x.roster_bonus + x.workout + x.per_game).round(), "gtd": (x.guaranteed + x.injury).round(),
            "void": x.void,
        })).unwrap_or(Value::Null)
                })
                .collect::<Vec<_>>()
        );
        let (dn, _) = k.dead_if_cut(year, today, false);
        let (dj, dj2) = k.dead_if_cut(year, today, true);
        v["cut"] = json!({
            "dead": dn.round(), "saves": k.cut_savings(year, today, false).round(),
            "dead_june1": dj.round(), "dead_june1_next": dj2.round(), "saves_june1": k.cut_savings(year, today, true).round(),
        });
        v["kind"] = json!(kind_words(k.kind));
        v["apy"] = json!(k.apy().round());
        v["total"] = json!(k.total().round());
        v["guaranteed"] = json!(k.guaranteed().round());
        v["gtd_left"] = json!(k.guaranteed_left(year, today).round());
        v["through"] = json!(k.last_year());
        v["years_left"] = json!((k.last_year() - year + 1).max(0));
        v["voids"] = json!(k.years.iter().filter(|x| x.void).count());
        v["no_trade"] = json!(k.no_trade);
        v["option"] = json!(k.option);
        v["where"] = json!(match c.status {
            Status::Roster => "Roster",
            Status::PracticeSquad => "Practice squad",
            Status::Reserve => "Reserve",
            _ => "",
        });
        // A deal in its last year restructures only with void years (2016 on).
        let last = k.last_year() <= year;
        v["last_year"] = json!(last);
        v["can_restructure"] = json!(
            money::capped(year)
                && k.year(year).is_some_and(|x| x.base > money::minimum(year, c.accrued) + 100_000.0)
                && c.status != Status::PracticeSquad
                && (!last || nfl::void_years_allowed(year))
        );
        v["ending"] = json!(k.last_year() < year);
        rows.push(v);
    }
    let book = |y: i32| {
        let committed = pro::payroll(w, t, y) - w.clubs[t].dead_in(y);
        let all: f64 = w.clubs[t]
            .roster
            .iter()
            .filter_map(|&i| w.people[i].pro.as_ref()?.contract.as_ref().filter(|k| k.team == t))
            .map(|k| k.cap_hit(y))
            .sum();
        json!({
            "year": y, "cap": money::price_cap(y).round(),
            "committed": committed.round(), "all": all.round(),
            "dead": w.clubs[t].dead_in(y).round(),
            "carryover": if y == year { w.clubs[t].carryover.round() } else { 0.0 },
            "room": pro::cap_room(w, t, y).round(),
            "reserve": pro::season_reserve(w, y).round(),
            // Void years that land on this year's cap at the league year.
            "voids": if y == year { pro::voids_ahead(w, t).round() } else { 0.0 },
            "contracts": w.clubs[t].roster.iter().filter(|&&i| w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).is_some_and(|k| k.team == t && k.year(y).is_some_and(|x| !x.void))).count(),
        })
    };
    // The rule of 51 holds until the first game (as `pro::payroll` counts it).
    let top51 = money::cap_books(year) && w.today < w.dates.nfl_kickoff;
    let dead: Vec<Value> = w.clubs[t]
        .dead
        .iter()
        .filter(|d| d.year >= year)
        .map(|d| {
            // A player the world no longer keeps (he left football) has no
            // page: his name without a link.
            let at = fo::index_of(w, d.player);
            let who = at.map(|i| w.people[i].name.clone()).unwrap_or_else(|| "A former player".into());
            json!({ "year": d.year, "amount": d.amount.round(), "id": at.map(|_| d.player), "name": who, "why": dead_words(d.why) })
        })
        .collect();
    let picks: Vec<Value> = deals::picks_of(w, t).iter().map(|p| pick_json(w, p)).collect();
    Ok(json!({
        "team": views::team_brief(w, TeamRef::Nfl(t)),
        "year": year, "years": years, "rows": rows,
        "owner": if w.front_office.owner(Duty::Cap) == Owner::You { "you" } else { "staff" },
        "books": years.iter().map(|&y| book(y)).collect::<Vec<_>>(),
        "top51": top51, "capped": money::capped(year),
        "dead": dead, "picks": picks,
        "after_june1": pro::after_june1(w, today),
        "june1_used": w.front_office.nfl.june1.iter().filter(|x| x.1 == year).count(),
        "voids_ok": nfl::void_years_allowed(year),
    }))
}

fn kind_words(k: money::Kind) -> &'static str {
    match k {
        money::Kind::Rookie => "Rookie",
        money::Kind::Undrafted => "Undrafted",
        money::Kind::Veteran => "Veteran",
        money::Kind::Minimum => "Minimum",
        money::Kind::Franchise => "Franchise tag",
        money::Kind::Transition => "Transition tag",
        money::Kind::Tender => "Tender",
        money::Kind::Future => "Future",
        money::Kind::PracticeSquad => "Practice squad",
        money::Kind::Extension => "Extension",
    }
}

fn dead_words(d: pro::DeadWhy) -> &'static str {
    use pro::DeadWhy::*;
    match d {
        CapCut => "Cap casualty",
        Released => "Released",
        Waived => "Guarantee after waivers",
        Settlement => "Injury settlement",
        PracticeSquad => "Moved to the squad",
        Retired => "Retired",
        Traded => "Traded (bonus kept)",
        Voided => "Void years",
        Incentives => "Incentives settled",
    }
}

fn pick_json(w: &World, p: &deals::Pick) -> Value {
    let t = views::mine(w).and_then(|t| if let TeamRef::Nfl(c) = t { Some(c) } else { None }).unwrap_or(0);
    json!({
        "year": p.year, "round": p.round, "orig": p.orig, "comp": p.comp,
        "from": views::team_brief(w, TeamRef::Nfl(p.orig)),
        "own": p.orig == t,
        "points": deals::pick_points(w, t, p).round(),
        "text": deals::pick_words(w, p),
    })
}

// --- Expiring deals, tags and tenders ------------------------------------------------------------

fn expiring(w: &World) -> Reply {
    let t = my_club(w)?;
    let year = w.year;
    let mut rows = Vec::new();
    for &i in &w.clubs[t].roster {
        let p = &w.people[i];
        let Some(c) = p.pro.as_ref() else { continue };
        let Some(k) = c.contract.as_ref() else { continue };
        let ending = k.last_year() < year;
        let tagged_now = k.signed == year
            && matches!(k.kind, money::Kind::Franchise | money::Kind::Transition | money::Kind::Tender);
        // Options from the 2011 class on, decided in its fourth season (2014).
        let option = c.draft.is_some_and(|d| d.1 == 1 && d.0 == year - 3)
            && k.kind == money::Kind::Rookie
            && k.option.is_none()
            && year >= 2014;
        // Due an extension: anyone in his last year, and a star with two
        // left (the Eagles' habit of extending early).
        let star = afm_engine::world::pro::market::consensus(w, i) >= 78.0;
        let extend = !ending
            && nfl::negotiable(w, i) == Some(nfl::TalkKind::Extension)
            && (k.last_year() == year || star && k.last_year() == year + 1);
        if !(ending || tagged_now || option || extend) {
            continue;
        }
        let mut v = prow(w, i);
        let accrued = c.accrued;
        v["accrued"] = json!(accrued);
        v["kind"] = json!(kind_words(k.kind));
        v["apy"] = json!(k.apy().round());
        v["through"] = json!(k.last_year());
        v["status_fa"] = json!(if !ending {
            ""
        } else if year < 1993 {
            "Rights held"
        } else if accrued >= money::ufa_seasons(year) {
            "Unrestricted"
        } else if accrued >= 3 {
            "Restricted"
        } else {
            "Exclusive rights"
        });
        let (lo, hi) = nfl::market_read(w, i);
        v["market"] = json!([lo, hi]);
        if ending && accrued >= money::ufa_seasons(year) {
            let (f, x, tr) = nfl::tag_prices(w, i);
            v["tags"] = json!({ "franchise": f.round(), "exclusive": x.round(), "transition": tr.round() });
        }
        if ending && (3..money::ufa_seasons(year)).contains(&accrued) {
            v["tenders"] = json!({
                "first": money::tender_price(year, money::Tender::First).max(k.cap_hit(year - 1) * 1.1).round(),
                "second": money::tender_price(year, money::Tender::Second).max(k.cap_hit(year - 1) * 1.1).round(),
                "original": money::tender_price(year, if c.draft.is_some() { money::Tender::Original } else { money::Tender::RightOfFirstRefusal }).max(k.cap_hit(year - 1) * 1.1).round(),
                "drafted": c.draft.map(|d| d.1),
            });
        }
        if ending && accrued < 3 {
            v["erfa"] = json!(money::minimum(year, c.accrued).round());
        }
        v["tagged"] = json!(tagged_now.then(|| kind_words(k.kind)));
        v["option"] = json!(option);
        v["option_price"] = json!(option.then(|| nfl::option_price(w, i).round()));
        v["option_choice"] = json!(w.front_office.nfl.options.iter().find(|x| x.0 == p.player.id).map(|x| x.1));
        v["extend"] = json!(extend);
        v["talk"] = talk_brief(w, p.player.id);
        let sheet = w.market.sheets.iter().any(|s| s.person == i && s.old == t);
        let (group, order) = if sheet {
            ("Offer sheets to match", 0)
        } else if tagged_now {
            ("Tagged and tendered", 1)
        } else if ending && accrued >= money::ufa_seasons(year) || ending && year < 1993 {
            ("Unrestricted free agents", 2)
        } else if ending && accrued >= 3 {
            ("Restricted free agents", 3)
        } else if ending {
            ("Exclusive rights", 4)
        } else if option {
            ("Fifth-year options", 5)
        } else {
            ("Due an extension", 6)
        };
        v["group"] = json!(group);
        v["order"] = json!(order);
        v["sheet"] = json!(w.market.sheets.iter().find(|s| s.person == i && s.old == t).map(|s| json!({
            "team": views::team_brief(w, TeamRef::Nfl(s.offer.team)), "apy": s.offer.apy.round(), "years": s.offer.years,
            "until": views::date_text(s.until),
            "choice": w.front_office.nfl.sheets.iter().find(|x| x.0 == p.player.id).map(|x| x.1),
        })));
        rows.push(v);
    }
    rows.sort_by(|a, b| {
        let m = |v: &Value| v["ovr"].as_array().map_or(0.0, |r| r.iter().filter_map(Value::as_f64).sum::<f64>());
        a["order"].as_i64().cmp(&b["order"].as_i64()).then(m(b).total_cmp(&m(a)))
    });
    Ok(json!({
        "team": views::team_brief(w, TeamRef::Nfl(t)),
        "year": year, "rows": rows,
        "owner": if w.front_office.owner(Duty::Contracts) == Owner::You { "you" } else { "staff" },
        "owner_tags": if w.front_office.owner(Duty::Tags) == Owner::You { "you" } else { "staff" },
        "window": nfl::tag_window(w),
        "window_open": views::date_text(w.dates.tag_window_open),
        "deadline": views::date_text(w.dates.tag_deadline.date),
        "league_year": views::date_text(w.dates.league_year.date),
        "option_deadline": views::date_text(w.dates.fifth_year_option),
        "before_league_year": w.today < w.dates.league_year.date,
        "one_tag": year >= 2011,
        "room": pro::cap_room(w, t, year).round(),
    }))
}

fn talk_brief(w: &World, id: u32) -> Value {
    w.front_office.nfl.talk(id).map_or(Value::Null, |t| {
        json!({ "state": format!("{:?}", t.state), "kind": format!("{:?}", t.kind), "patience": t.patience,
            "offer": t.offer.map(|p| proposal_json(&p)), "ask": t.ask.map(|p| proposal_json(&p)) })
    })
}

fn proposal_json(p: &nfl::Proposal) -> Value {
    json!({ "years": p.years, "apy": p.apy.round(), "gtd": p.gtd, "bonus": p.bonus, "void_years": p.void_years,
        "incentives": p.incentives.round(), "no_trade": p.no_trade, "start": p.start, "ramp": p.ramp, "low_first": p.low_first, "total": p.total().round() })
}

// --- A talk with an agent ----------------------------------------------------------------------

fn talk(w: &World, id: u32) -> Reply {
    let t = my_club(w)?;
    let i = fo::index_of(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let c = p.pro.as_ref().ok_or("Not a pro.")?;
    let year = w.year;
    let kind = nfl::negotiable(w, i);
    let agent = c.agent.map(|a| {
        let ag = &w.market.agents[a];
        let rel = ag.rel.get(t).copied().unwrap_or(0.0);
        json!({
            "name": ag.name, "agency": ag.agency, "clients": ag.clients,
            "size": match ag.size { 0 => "a one-man shop", 1 => "a small agency", 2 => "a mid-sized agency", _ => "one of the giants" },
            "history": if rel > 0.3 { "Your club and he have done good business." } else if rel < -0.3 { "He has burned your club before." } else { "No real history with your club." },
        })
    });
    let contract = c.contract.as_ref().map(|k| json!({
        "kind": kind_words(k.kind), "apy": k.apy().round(), "through": k.last_year(), "team": views::team_brief(w, TeamRef::Nfl(k.team)),
        "hit": k.cap_hit(year).round(), "guaranteed": k.guaranteed().round(),
    }));
    let (lo, hi) = nfl::market_read(w, i);
    let talk = w.front_office.nfl.talk(id).or_else(|| w.front_office.nfl.talks.iter().rev().find(|x| x.player == id));
    let visited = w.front_office.nfl.visits.iter().any(|v| v.0 == id);
    // The season just played counts from the Super Bowl on.
    let accrued = c.accrued;
    let minimum = money::minimum(year, accrued);
    let mut v = prow(w, i);
    v["contract"] = json!(contract);
    v["agent"] = json!(agent);
    v["market"] = json!([lo, hi]);
    v["kind"] = json!(kind.map(|k| format!("{k:?}")));
    v["kind_words"] = json!(match kind {
        Some(nfl::TalkKind::ReSign) =>
            "Re-signing before free agency: only you can talk to him until the market opens.",
        Some(nfl::TalkKind::Extension) =>
            "An extension: new years on top of his deal. The new money starts next league year (a tagged player's deal replaces the tag).",
        Some(nfl::TalkKind::FreeAgent) if w.today < w.dates.league_year.date =>
            "The negotiating window: clubs may agree terms with his agent; nothing is signed and he can't visit until the league year opens.",
        Some(nfl::TalkKind::FreeAgent) =>
            "A free agent: your offer sits on the table beside every other club's, and he takes the one that fits what he wants.",
        Some(nfl::TalkKind::OfferSheet) =>
            "A restricted free agent: if he signs your offer sheet, his club has five days to match or take the draft pick his tender names.",
        None => "You can't talk terms with him today.",
    });
    v["lines"] = json!(talk.map_or(Vec::new(), |t| {
        t.lines
            .iter()
            .map(|l| json!({ "date": views::date_text(l.date), "ours": l.ours, "text": l.text }))
            .collect::<Vec<_>>()
    }));
    v["talk"] = talk_brief(w, id);
    v["accrued"] = json!(accrued);
    // Your own player you know; a free agent tells you on a visit or after
    // a few rounds of talks.
    let known = visited
        || matches!(kind, Some(nfl::TalkKind::ReSign | nfl::TalkKind::Extension))
        || talk.is_some_and(|x| x.lines.len() >= 6);
    v["wants"] = json!(known.then(|| nfl::wants_words(w, i)));
    v["visited"] = json!(visited);
    v["can_visit"] = json!(
        kind == Some(nfl::TalkKind::FreeAgent) && w.today >= w.dates.league_year.date && !nfl::visited_recently(w, id)
    );
    v["minimum"] = json!(minimum.round());
    v["voids_ok"] = json!(nfl::void_years_allowed(year));
    v["room"] = json!(pro::cap_room(w, t, year).round());
    v["room_next"] = json!(pro::cap_room(w, t, year + 1).round());
    v["cap"] = json!(money::price_cap(year).round());
    v["negotiating"] = json!(nfl::negotiating(w).round());
    v["year"] = json!(year);
    Ok(v)
}

fn proposal_of(args: &Value) -> nfl::Proposal {
    let num = |k: &str| args.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    nfl::Proposal {
        years: num("years").round().max(1.0) as u32,
        apy: num("apy"),
        gtd: num("gtd").clamp(0.0, 1.0),
        bonus: num("bonus").clamp(0.0, 1.0),
        void_years: num("void_years") as u32,
        incentives: num("incentives"),
        no_trade: flag(args, "no_trade"),
        start: flag(args, "start"),
        ramp: args.get("ramp").and_then(Value::as_f64).unwrap_or(0.06).clamp(0.0, 0.3),
        low_first: flag(args, "low_first"),
    }
}

/// The deal laid out by year, exactly as it would be signed: base, bonus
/// proration, roster and workout bonuses, the cap number and what's
/// guaranteed each year, void years at the end.
fn preview(w: &World, args: &Value) -> Reply {
    let t = my_club(w)?;
    let i = fo::index_of(w, id(args)?).ok_or("No such player.")?;
    let p = proposal_of(args);
    let ext = nfl::negotiable(w, i) == Some(nfl::TalkKind::Extension);
    let tagged = w.people[i]
        .pro
        .as_ref()
        .and_then(|c| c.contract.as_ref())
        .is_some_and(|k| matches!(k.kind, money::Kind::Franchise | money::Kind::Transition));
    let start = if ext && !tagged { w.year + 1 } else { w.year };
    let k = money::build(t, money::Kind::Veteran, start, &p.terms(start, w.people[i].player.age));
    // Your room after the deal: the years it replaces of his current deal
    // come off the books, the new cap numbers go on.
    let old = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).filter(|o| o.team == t);
    // His old deal's pay for the years the new one covers goes; its bonus
    // spread stays on the cap (an extension keeps it, a deal that ran out
    // turns it into dead money).
    let freed = |o: &money::Contract, y: i32| o.cap_hit(y) - o.year(y).map_or(0.0, |x| x.proration);
    // A deal that ran out: its later void years land on this year's cap.
    let ended = old.filter(|o| o.last_year() < w.year);
    let voids = |y: i32| {
        ended.map_or(0.0, |o| {
            if y == w.year {
                -pro::void_ahead(w, i)
            } else {
                o.year(y).filter(|_| y > w.year).map_or(0.0, |x| x.proration)
            }
        })
    };
    let after = |y: i32| {
        pro::cap_room(w, t, y) + old.map_or(0.0, |o| if y >= start { freed(o, y) } else { 0.0 }) + voids(y)
            - k.cap_hit(y)
    };
    let short = after(start);
    let years: Vec<Value> = k
        .years
        .iter()
        .map(|y| {
            json!({
                "year": y.year, "base": y.base.round(), "bonus": y.proration.round(),
                "other": (y.roster_bonus + y.workout + y.per_game).round(), "hit": y.cap_hit().round(),
                "gtd": y.guaranteed.round(), "injury": y.injury.round(), "void": y.void,
                "share": (y.cap_hit() / money::price_cap(y.year)),
                "room": (y.year >= w.year && y.year <= w.year + 4).then(|| after(y.year).round()),
            })
        })
        .collect();
    Ok(json!({
        "years": years, "start": start, "total": k.total().round(), "apy": k.apy().round(),
        "guaranteed": k.guaranteed().round(), "bonus": k.signing_bonus.round(),
        "first_cash": k.years.first().map_or(0.0, |y| y.cash()).round(),
        "text": nfl::describe_proposal(&p, w.year),
        "over": money::capped(start).then_some(-short).filter(|x| *x > 0.0).map(f64::round),
    }))
}

fn offer(w: &mut World, args: &Value) -> Reply {
    let id = id(args)?;
    let p = proposal_of(args);
    let a = nfl::propose(w, id, p)?;
    let mut v = talk(w, id)?;
    v["answer"] = json!({
        "verdict": format!("{:?}", a.verdict), "text": a.text, "claim": a.claim, "read": a.read, "gap": a.gap,
        "ask": a.ask.map(|p| proposal_json(&p)),
    });
    Ok(v)
}

// --- Free agency --------------------------------------------------------------------------------

fn free_agents(w: &World, args: &Value) -> Reply {
    let t = my_club(w)?;
    let year = w.year;
    let pos =
        args.get("pos").and_then(Value::as_str).and_then(|s| Position::ALL.iter().find(|p| p.name() == s).copied());
    let street = args.get("street").and_then(Value::as_bool).unwrap_or(false);
    let mut list: Vec<(usize, f64)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::Pro {
            continue;
        }
        let Some(c) = p.pro.as_ref() else { continue };
        if pos.is_some_and(|x| x != p.player.position) {
            continue;
        }
        let fa = c.status == Status::FreeAgent && !c.on_waivers;
        let window = nfl::negotiable(w, i) == Some(nfl::TalkKind::FreeAgent);
        let sheet = nfl::negotiable(w, i) == Some(nfl::TalkKind::OfferSheet);
        if !(fa || window || sheet) {
            continue;
        }
        let read = pro::market::consensus(w, i);
        if !street && read < 62.0 && !sheet {
            continue;
        }
        list.push((i, read));
    }
    list.sort_by(|a, b| b.1.total_cmp(&a.1));
    list.truncate(250);
    let rows: Vec<Value> = list
        .iter()
        .map(|&(i, _)| {
            let p = &w.people[i];
            let mut v = prow(w, i);
            let (lo, hi) = nfl::market_read(w, i);
            v["market"] = json!([lo, hi]);
            v["kind"] = json!(nfl::negotiable(w, i).map(|k| format!("{k:?}")));
            v["former"] = p.pro.as_ref().and_then(|c| c.former.or(c.team)).map_or(Value::Null, |o| views::team_brief(w, TeamRef::Nfl(o)));
            let pending = w.market.pending.iter().find(|x| x.person == i);
            v["agreed"] = pending.map_or(Value::Null, |x| json!({ "team": views::team_brief(w, TeamRef::Nfl(x.offer.team)), "mine": x.offer.team == t, "until": views::date_text(x.until), "verbal": x.verbal }));
            v["talk"] = talk_brief(w, p.player.id);
            let visited = w.front_office.nfl.visits.iter().any(|x| x.0 == p.player.id);
            v["visited"] = json!(visited);
            // The row brings him in, as his talk does (from the league year).
            v["can_visit"] = json!(
                nfl::negotiable(w, i) == Some(nfl::TalkKind::FreeAgent)
                    && w.today >= w.dates.league_year.date
                    && !nfl::visited_recently(w, p.player.id)
            );
            v["rfa"] = json!(p.pro.as_ref().and_then(|c| c.rfa).map(|r| format!("{r:?}")));
            v
        })
        .collect();
    let lyd = w.dates.league_year.date;
    let opened = w.dates.tampering_open.map_or(lyd, |m| m.date);
    let phase = if w.today < opened {
        "Free agency opens with the negotiating window"
    } else if w.today < lyd {
        "The negotiating window: agree terms, sign when the league year opens"
    } else if w.today < w.dates.veterans_report {
        "Free agency is open"
    } else {
        "Camp and the season: the street"
    };
    Ok(json!({
        "team": views::team_brief(w, TeamRef::Nfl(t)), "rows": rows, "phase": phase,
        "opens": views::date_text(opened), "league_year": views::date_text(lyd),
        "room": pro::cap_room(w, t, year).round(),
        "roster": roster::count(w, t, Status::Roster), "limit": roster::roster_limit(w),
        "owner": if w.front_office.owner(Duty::FreeAgency) == Owner::You { "you" } else { "staff" },
        "positions": Position::ALL.iter().map(|p| p.name()).collect::<Vec<_>>(),
    }))
}

// --- Trades -------------------------------------------------------------------------------------

fn trade_desk(w: &World) -> Reply {
    let t = my_club(w)?;
    let calls: Vec<Value> = w
        .front_office
        .trades
        .calls
        .iter()
        .enumerate()
        .rev()
        .map(|(k, c)| {
            let i = fo::index_of(w, c.player);
            json!({
                "k": k, "date": views::date_text(c.date), "until": views::date_text(c.until), "answered": c.answered,
                "team": views::team_brief(w, TeamRef::Nfl(c.buyer)),
                "player": i.map(|i| prow(w, i)),
                "picks": c.picks.iter().map(|p| pick_json(w, p)).collect::<Vec<_>>(),
            })
        })
        .collect();
    let block: Vec<Value> =
        w.front_office.trades.block.iter().filter_map(|&id| fo::index_of(w, id)).map(|i| prow(w, i)).collect();
    let teams: Vec<Value> = afm_engine::world::franchise::active_teams(w.year)
        .into_iter()
        .filter(|&o| o != t)
        .map(|o| {
            let mut v = views::team_brief(w, TeamRef::Nfl(o));
            let off = &w.clubs[o].office;
            v["mode"] = json!(mode_words(off.mode));
            v["gm"] = json!(off.gm.name);
            v["room"] = json!(pro::cap_room(w, o, w.year).round());
            v
        })
        .collect();
    let conds: Vec<Value> = w
        .front_office
        .trades
        .conditionals
        .iter()
        .map(|c| json!({ "pick": pick_json(w, &c.pick), "cond": c.cond.words(), "player": fo::index_of(w, c.player).map(|i| w.people[i].name.clone()), "from_me": c.from == t }))
        .collect();
    let made: Vec<Value> = w
        .front_office
        .trades
        .made
        .iter()
        .rev()
        .map(|(d, s)| json!({ "date": views::date_text(*d), "text": s }))
        .collect();
    let mine: Vec<Value> = w.clubs[t]
        .roster
        .iter()
        .filter(|&&i| tradable(w, t, i, Status::Roster) || tradable(w, t, i, Status::Reserve))
        .map(|&i| {
            let mut v = prow(w, i);
            v["points"] = json!(deals::player_points(w, t, i).round());
            v["hit"] = json!(
                w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).map_or(0.0, |k| k.cap_hit(w.year)).round()
            );
            v["block"] = json!(w.front_office.trades.block.contains(&w.people[i].player.id));
            v
        })
        .collect();
    Ok(json!({
        "team": views::team_brief(w, TeamRef::Nfl(t)),
        "open": deals::window_open(w),
        "deadline": views::date_text(w.dates.trade_deadline.date),
        "league_year": views::date_text(w.dates.league_year.date),
        "calls": calls, "block": block, "teams": teams, "conditionals": conds, "made": made,
        "picks": deals::picks_of(w, t).iter().map(|p| pick_json(w, p)).collect::<Vec<_>>(),
        "roster": mine,
        "classic": w.clubs[t].office.gm.classic_chart,
        "owner": if w.front_office.owner(Duty::Trades) == Owner::You { "you" } else { "staff" },
    }))
}

/// A player a club can trade today: his deal runs this league year.
fn tradable(w: &World, t: usize, i: usize, s: Status) -> bool {
    w.people[i]
        .pro
        .as_ref()
        .is_some_and(|c| c.status == s && c.contract.as_ref().is_some_and(|k| k.team == t && k.last_year() >= w.year))
}

fn mode_words(m: afm_engine::world::pro::office::Mode) -> &'static str {
    use afm_engine::world::pro::office::Mode;
    match m {
        Mode::Contend => "chasing a title",
        Mode::Middle => "in the middle",
        Mode::Rebuild => "rebuilding",
    }
}

fn trade_team(w: &World, args: &Value) -> Reply {
    let me = my_club(w)?;
    let o = args.get("team").and_then(Value::as_u64).ok_or("Missing team.")? as usize;
    if o >= w.clubs.len() {
        return Err("No such club.".into());
    }
    let rows: Vec<Value> = w.clubs[o]
        .roster
        .iter()
        .filter(|&&i| tradable(w, o, i, Status::Roster))
        .map(|&i| {
            let mut v = prow(w, i);
            v["points"] = json!(deals::player_points(w, me, i).round());
            let k = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref());
            v["hit"] = json!(k.map_or(0.0, |k| k.cap_hit(w.year)).round());
            v["through"] = json!(k.map(|k| k.last_year()));
            v["no_trade"] = json!(k.is_some_and(|k| k.no_trade));
            v
        })
        .collect();
    let mut team = views::team_brief(w, TeamRef::Nfl(o));
    let off = &w.clubs[o].office;
    team["mode"] = json!(mode_words(off.mode));
    team["gm"] = json!(off.gm.name);
    Ok(json!({
        "team": team, "rows": rows,
        "picks": deals::picks_of(w, o).iter().map(|p| pick_json(w, p)).collect::<Vec<_>>(),
        "room": pro::cap_room(w, o, w.year).round(),
    }))
}

fn proposal(w: &World, args: &Value) -> Result<deals::Proposal, String> {
    let other = args.get("team").and_then(Value::as_u64).ok_or("Pick a club.")? as usize;
    let ids = |k: &str| -> Vec<u32> {
        args.get(k)
            .and_then(Value::as_array)
            .map_or(Vec::new(), |a| a.iter().filter_map(|x| x.as_u64().map(|v| v as u32)).collect())
    };
    let picks = |k: &str| -> Vec<deals::Pick> {
        args.get(k).and_then(Value::as_array).map_or(Vec::new(), |a| {
            a.iter()
                .filter_map(|x| {
                    Some(deals::Pick {
                        year: x.get("year")?.as_i64()? as i32,
                        round: x.get("round")?.as_u64()? as u32,
                        orig: x.get("orig")?.as_u64()? as usize,
                        comp: x.get("comp").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect()
        })
    };
    let give = deals::Side { players: ids("give_players"), picks: picks("give_picks") };
    let get = deals::Side { players: ids("get_players"), picks: picks("get_picks") };
    let conditional = args.get("conditional").filter(|c| !c.is_null()).and_then(|c| {
        let pk = deals::Pick {
            year: c.get("year")?.as_i64()? as i32,
            round: c.get("round")?.as_u64()? as u32,
            orig: c.get("orig")?.as_u64()? as usize,
            comp: false,
        };
        let n = c.get("n").and_then(Value::as_u64).unwrap_or(8) as u32;
        let cond = match c.get("kind").and_then(Value::as_str).unwrap_or("games") {
            "starts" => deals::Condition::Starts(n),
            "pro_bowl" => deals::Condition::ProBowl,
            _ => deals::Condition::Games(n),
        };
        Some((pk, cond, c.get("player")?.as_u64()? as u32))
    });
    let _ = w;
    Ok(deals::Proposal { other, give, get, conditional })
}

fn answer_json(w: &World, a: &deals::Answer) -> Value {
    json!({
        "accept": a.accept, "text": a.text,
        "counter": a.counter.as_ref().map(|v| v.iter().map(|p| pick_json(w, p)).collect::<Vec<_>>()),
        "points": [a.points.0.round(), a.points.1.round()],
    })
}

// --- Cutdown, the practice squad and the wire ----------------------------------------------------

fn cutdown(w: &World) -> Reply {
    let t = my_club(w)?;
    let (rank, keep53, squad) = roster::staff_cut(w, t);
    let plan = &w.front_office.roster.plan;
    // Players whose deals end before camp are gone by cutdown day.
    let year = w.year;
    let under =
        |i: usize| w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).is_some_and(|k| k.last_year() >= year);
    let mut rows: Vec<Value> = rank
        .iter()
        .enumerate()
        .filter(|(_, x)| under(x.0))
        .map(|(k, &(i, _))| {
            let id = w.people[i].player.id;
            let mut v = prow(w, i);
            v["rank"] = json!(k + 1);
            v["staff"] = json!(if keep53.contains(&i) {
                "keep"
            } else if squad.contains(&i) {
                "squad"
            } else {
                "release"
            });
            v["plan"] = json!(plan.iter().find(|x| x.0 == id).map(|x| match x.1 {
                roster::Cut::Keep => "keep",
                roster::Cut::Squad => "squad",
                roster::Cut::Release => "release",
            }));
            v["squad_ok"] = json!(roster::squad_ok(w, i));
            let k = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref());
            v["hit"] = json!(k.map_or(0.0, |k| k.cap_hit(w.year)).round());
            v["dead"] =
                json!(k.map_or(0.0, |k| k.dead_if_cut(w.year, w.today, pro::after_june1(w, w.today)).0).round());
            v["round"] = json!(w.people[i].pro.as_ref().and_then(|c| c.draft).map(|d| d.1));
            v["draft"] =
                json!(w.people[i].pro.as_ref().and_then(|c| c.draft).map(|d| format!("{} R{} · #{}", d.0, d.1, d.2)));
            v["pos_order"] = json!(w.people[i].player.position.index());
            v
        })
        .collect();
    // By position, each in the staff's order.
    rows.sort_by_key(|v| (v["pos_order"].as_u64(), v["rank"].as_u64()));
    Ok(json!({
        "team": views::team_brief(w, TeamRef::Nfl(t)),
        "rows": rows, "limit": w.rules.nfl_roster, "squad_limit": w.rules.nfl_practice_squad,
        "date": views::date_text(w.dates.cutdown.date), "in": w.today.days_to(w.dates.cutdown.date),
        "owner": if w.front_office.owner(Duty::Cutdown) == Owner::You { "you" } else { "staff" },
    }))
}

fn practice_squad(w: &World) -> Reply {
    let t = my_club(w)?;
    let year = w.year;
    let squad: Vec<Value> = w.clubs[t]
        .roster
        .iter()
        .filter(|&&i| w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::PracticeSquad))
        .map(|&i| {
            let mut v = prow(w, i);
            v["protected"] = json!(w.front_office.roster.protect.contains(&w.people[i].player.id));
            v["elevations"] = json!(
                w.people[i]
                    .pro
                    .as_ref()
                    .and_then(|c| c.seasons.iter().find(|s| s.year == year))
                    .map_or(0, |s| s.elevations)
            );
            v
        })
        .collect();
    let mut street: Vec<(usize, f64)> = (0..w.people.len())
        .filter(|&i| {
            w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers && c.out_weeks == 0)
                && roster::squad_ok(w, i)
        })
        .map(|i| (i, pro::market::consensus(w, i)))
        .collect();
    street.sort_by(|a, b| b.1.total_cmp(&a.1));
    street.truncate(60);
    Ok(json!({
        "team": views::team_brief(w, TeamRef::Nfl(t)),
        "squad": squad,
        "street": street.iter().map(|&(i, _)| prow(w, i)).collect::<Vec<_>>(),
        "limit": w.rules.nfl_practice_squad,
        "protect": if year >= 2020 { 4 } else { 0 },
        "open": w.today >= w.dates.cutdown.date && w.today <= w.dates.super_bowl,
        "roster": roster::count(w, t, Status::Roster), "roster_limit": roster::roster_limit(w),
        "owner": if w.front_office.owner(Duty::PracticeSquad) == Owner::You { "you" } else { "staff" },
    }))
}

fn wire(w: &World) -> Reply {
    let t = my_club(w)?;
    let rows: Vec<Value> = (0..w.people.len())
        .filter(|&i| w.people[i].pro.as_ref().is_some_and(|c| c.on_waivers))
        .map(|i| {
            let id = w.people[i].player.id;
            let mut v = prow(w, i);
            let k = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref());
            v["hit"] = json!(k.map_or(0.0, |k| k.cap_hit(w.year)).round());
            v["through"] = json!(k.map(|k| k.last_year()));
            v["from"] = w.people[i]
                .pro
                .as_ref()
                .and_then(|c| c.former)
                .map_or(Value::Null, |o| views::team_brief(w, TeamRef::Nfl(o)));
            v["claimed"] = json!(w.front_office.roster.claims.iter().any(|x| x.0 == id));
            v
        })
        .collect();
    Ok(json!({
        "team": views::team_brief(w, TeamRef::Nfl(t)),
        "rows": rows, "place": roster::waiver_place(w, t),
        "roster": roster::count(w, t, Status::Roster), "limit": roster::roster_limit(w),
        "room": pro::cap_room(w, t, w.year).round(),
        "owner": if w.front_office.owner(Duty::Waivers) == Owner::You { "you" } else { "staff" },
    }))
}
