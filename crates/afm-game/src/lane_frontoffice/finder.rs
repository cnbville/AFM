//! The trade finder's commands (TF1-TF19): the shop around one of your
//! players, its offers and talks, the closing, moving a bad contract and
//! draft night. The clubs' top prices stay in the engine: the screens get
//! the offers, your staff's ranges and your own chart's points.

use afm_engine::world::World;
use afm_engine::world::front_office::{self as fo, deals, finder};
use afm_engine::world::pro::{self, bargain, money};
use afm_engine::world::scouting::live;
use serde_json::{Value, json};

use super::nfl::{pick_json, prow};
use crate::views::{self, TeamRef};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let r = match cmd {
        "tf_view" => game.world.as_ref().map(|w| view(w, args)),
        "tf_start" => game.world.as_mut().map(|w| {
            let clubs: Vec<usize> = args
                .get("clubs")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|x| x.as_u64().map(|v| v as usize)).collect())
                .unwrap_or_default();
            finder::start(
                w,
                id(args)?,
                flag(args, "open"),
                ask(args),
                clubs,
                num(args, "paydown"),
                flag(args, "agent"),
            )?;
            view(w, args)
        }),
        "tf_counter" => game.world.as_mut().map(|w| {
            let picks: Vec<deals::Pick> = args
                .get("picks")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(pick_of).collect())
                .unwrap_or_default();
            let text = finder::counter(w, club(args)?, picks)?;
            with_text(w, args, text)
        }),
        "tf_rival" => game.world.as_mut().map(|w| {
            let rival = args.get("rival").and_then(Value::as_u64).ok_or("Missing rival.")? as usize;
            let text = finder::rival(w, club(args)?, rival, flag(args, "bluff"))?;
            with_text(w, args, text)
        }),
        "tf_accept" => game.world.as_mut().map(|w| {
            finder::accept(w, club(args)?)?;
            view(w, args)
        }),
        "tf_lapse" => game.world.as_mut().map(|w| {
            finder::lapse(w, club(args)?, flag(args, "walk"))?;
            view(w, args)
        }),
        "tf_close" => game.world.as_mut().map(|w| {
            finder::close(w)?;
            view(w, args)
        }),
        "tf_revised" => game.world.as_mut().map(|w| {
            finder::revised(w, flag(args, "yes"))?;
            view(w, args)
        }),
        "tf_dump" => game.world.as_mut().map(|w| {
            let text = finder::dump(w, id(args)?, club(args)?)?;
            Ok(json!({ "text": text }))
        }),
        "tf_night" => game.world.as_mut().map(|w| {
            let text = finder::night_accept(w, id(args)?, club(args)?)?;
            Ok(json!({ "text": text }))
        }),
        _ => return None,
    };
    Some(r.unwrap_or_else(|| Err("No world is loaded.".into())))
}

fn id(args: &Value) -> Result<u32, String> {
    args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".into())
}

fn club(args: &Value) -> Result<usize, String> {
    args.get("club").and_then(Value::as_u64).map(|v| v as usize).ok_or_else(|| "Missing club.".into())
}

fn flag(args: &Value, k: &str) -> bool {
    args.get(k).and_then(Value::as_bool).unwrap_or(false)
}

fn num(args: &Value, k: &str) -> f64 {
    args.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

fn ask(args: &Value) -> finder::Ask {
    match args.get("ask").and_then(Value::as_str) {
        Some("first") => finder::Ask::Round(1),
        Some("second") => finder::Ask::Round(2),
        Some("starter") => finder::Ask::StarterAndPick,
        _ => finder::Ask::Best,
    }
}

fn ask_key(a: finder::Ask) -> &'static str {
    match a {
        finder::Ask::Round(1) => "first",
        finder::Ask::Round(_) => "second",
        finder::Ask::StarterAndPick => "starter",
        finder::Ask::Best => "best",
    }
}

fn pick_of(v: &Value) -> Option<deals::Pick> {
    Some(deals::Pick {
        year: v.get("year")?.as_i64()? as i32,
        round: v.get("round")?.as_u64()? as u32,
        orig: v.get("orig")?.as_u64()? as usize,
        comp: v.get("comp").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn with_text(w: &World, args: &Value, text: String) -> Reply {
    let mut v = view(w, args)?;
    v["text"] = json!(text);
    Ok(v)
}

fn package_json(w: &World, pk: &bargain::Package, from: usize) -> Value {
    json!({
        "picks": pk.picks.iter().map(|p| {
            let mut v = pick_json(w, p);
            if let Some((_, c)) = pk.cond.filter(|c| c.0 == *p) {
                v["cond"] = json!(c.words());
            }
            v
        }).collect::<Vec<_>>(),
        "back": pk.back.iter().filter_map(|&id| fo::index_of(w, id)).map(|j| prow(w, j)).collect::<Vec<_>>(),
        "swap": pk.swap.map(|(mine, theirs)| json!({ "give": pick_json(w, &mine), "get": pick_json(w, &theirs) })),
        "worth": pk.worth.round(),
        "text": finder::package_words(w, pk, from),
    })
}

fn state_words(s: &finder::State) -> (&'static str, String) {
    use finder::State::*;
    match s {
        Waiting => ("waiting", "Still thinking".into()),
        Offer => ("offer", "Offer".into()),
        Passed(why) => ("passed", why.clone()),
        Lapsed => ("lapsed", "Lapsed".into()),
        Dropped => ("dropped", "Dropped out".into()),
        Ended(why) => ("ended", why.clone()),
        Won => ("won", "Accepted".into()),
    }
}

fn step_json(s: &finder::Step) -> Value {
    json!({ "what": s.what, "due": views::date_text(s.due), "done": s.done })
}

/// How he's taking it (TF19): his trust in the club, and whether he knows.
fn mood(w: &World, i: usize, knows: bool) -> Value {
    let trust = w.people[i].pro.as_ref().map_or(0.5, |c| c.trust);
    let words = if !knows {
        "He doesn't know he's being shopped."
    } else if trust >= 0.6 {
        "He knows, and takes it as business."
    } else if trust >= 0.35 {
        "He knows, and he's hurt by it."
    } else {
        "He knows, and he's angry with the club."
    };
    json!({ "trust": (trust * 100.0).round(), "knows": knows, "words": words })
}

/// Everything the finder's tabs show. `id` picks the player (else the one
/// in the shop); the setup choices (`open`, `ask`, `paydown`, `agent`)
/// shape the market's read; `club` picks the talks; `move` adds the cost
/// of moving his contract; `night` the draft's offers for him.
fn view(w: &World, args: &Value) -> Reply {
    let me = match views::mine(w) {
        Some(TeamRef::Nfl(t)) => t,
        _ => return Err("You don't run an NFL club.".into()),
    };
    let shop = w.front_office.trades.shop.as_ref();
    let pid = id(args).ok().or(shop.map(|s| s.player)).ok_or("Pick a player to shop.")?;
    let i = fo::index_of(w, pid).ok_or("No such player.")?;
    let year = w.year;
    let k = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref());
    let mut player = prow(w, i);
    player["points"] = json!(deals::player_points(w, me, i).round());
    player["hit"] = json!(k.map_or(0.0, |k| k.cap_hit(year)).round());
    player["through"] = json!(k.map(|k| k.last_year()));
    player["leverage"] = json!(finder::leverage(w, pid));
    player["wants_out"] = json!(w.people[i].pro.as_ref().is_some_and(|c| c.wants_out));
    // Traded away (the shop closed on a deal): what he was worth is gone.
    let ours = w.people[i].pro.as_ref().and_then(|c| c.team) == Some(me);
    player["mine"] = json!(ours);
    if !ours {
        player["points"] = Value::Null;
        player["gone_to"] = w.people[i].pro.as_ref().and_then(|c| c.team).map_or(Value::Null, |o| json!(w.nfl[o].name));
    }
    let this = shop.filter(|s| s.player == pid);
    let ask_v = if args.get("ask").is_some() { ask(args) } else { this.map_or(finder::Ask::Best, |s| s.ask) };
    let paydown = if args.get("paydown").is_some() { num(args, "paydown") } else { this.map_or(0.0, |s| s.paydown) };
    let agent = if args.get("agent").is_some() { flag(args, "agent") } else { this.is_some_and(|s| s.agent) };
    let reads: Vec<Value> = if ours { finder::reads(w, pid, ask_v, paydown, agent)? } else { vec![] }
        .into_iter()
        .map(|r| {
            let mut v = views::team_brief(w, TeamRef::Nfl(r.club));
            let off = &w.clubs[r.club].office;
            json!({
                "team": std::mem::take(&mut v), "club": r.club, "gm": off.gm.name,
                "record": record(w, r.club), "mode": r.mode, "race": (r.race * 100.0).round(),
                "need": need_words(r.need), "need_n": r.need, "room": r.room.round(), "fits": r.fits,
                "low": r.low.round(), "high": r.high.round(), "pass": r.pass,
            })
        })
        .collect();
    let paydowns: Vec<Value> = [0.0, 0.25, 0.5]
        .iter()
        .map(|&f| json!({ "share": f, "amount": finder::paydown_amount(w, i, f).round() }))
        .collect();
    let mut out = json!({
        "team": views::team_brief(w, TeamRef::Nfl(me)),
        "player": player,
        "open": deals::window_open(w),
        "deadline": views::date_text(w.dates.trade_deadline.date),
        "deadline_day": w.today >= w.dates.trade_deadline.date,
        "today": views::date_text(w.today),
        "ask": ask_key(ask_v),
        "ask_points": finder::ask_points(w, me, i, ask_v).round(),
        "paydown": paydown, "agent": agent, "paydowns": paydowns,
        "reads": reads,
        "classic": w.clubs[me].office.gm.classic_chart,
        "mood": mood(w, i, this.is_some_and(|s| s.open || s.leaked) || shop.is_none() && w.people[i].pro.as_ref().is_some_and(|c| c.wants_out)),
        "shopping_other": shop.filter(|s| s.player != pid && !s.closed).and_then(|s| fo::index_of(w, s.player)).map(|j| w.people[j].name.clone()),
    });
    if let Some(s) = this {
        let mine_worth = deals::player_points(w, me, i).max(1.0);
        let bids: Vec<Value> = s
            .bids
            .iter()
            .map(|b| {
                let (key, words) = state_words(&b.state);
                json!({
                    "club": b.club, "team": views::team_brief(w, TeamRef::Nfl(b.club)),
                    "gm": w.clubs[b.club].office.gm.name, "state": key, "words": words,
                    "due": views::date_text(b.due), "until": views::date_text(b.until),
                    "package": b.package.as_ref().map(|p| package_json(w, p, b.club)),
                    "against": b.package.as_ref().map(|p| (100.0 * p.worth / mine_worth).round()),
                    "ext": b.ext.map(|o| json!({ "apy": o.apy.round(), "years": o.years })),
                    "patience": (b.patience * 100.0).round(), "counters": b.counters,
                })
            })
            .collect();
        let log: Vec<Value> = s
            .log
            .iter()
            .map(|l| json!({ "date": views::date_text(l.date), "club": l.club, "text": l.text, "you": l.you }))
            .collect();
        let closing = s.closing.as_ref().map(|c| {
            let pd = finder::paydown_amount(w, i, s.paydown);
            // Done: what your books carry for him; before, what they would.
            let kept = if ours {
                bargain::kept_now(w, i)
            } else {
                w.clubs[me].dead.iter().filter(|d| d.player == pid && d.year == year).map(|d| d.amount).sum::<f64>()
                    - pd
            };
            let hit = k.map_or(0.0, |k| k.cap_hit(year));
            let back_hits: f64 = c
                .package
                .back
                .iter()
                .filter_map(|&id| fo::index_of(w, id))
                .filter_map(|j| w.people[j].pro.as_ref().and_then(|p| p.contract.as_ref()).map(|k| k.cap_hit(year)))
                .sum();
            json!({
                "club": c.club, "team": views::team_brief(w, TeamRef::Nfl(c.club)),
                "package": package_json(w, &c.package, c.club),
                "revised": c.revised.as_ref().map(|p| package_json(w, p, c.club)),
                "owners": c.owners.iter().map(|(_, st)| step_json(st)).collect::<Vec<_>>(),
                "deal": c.deal.as_ref().map(step_json),
                "ext": c.ext.map(|o| json!({ "apy": o.apy.round(), "years": o.years })),
                "physical": step_json(&c.physical),
                "result": c.result,
                "cap": { "hit": hit.round(), "dead": (kept + pd).round(), "paydown": pd.round(),
                         "back": back_hits.round(), "room": pro::cap_room(w, me, year).round(),
                         "saved": (hit - kept - pd - back_hits).round() },
            })
        });
        let counts = |k: &str| bids.iter().filter(|b| b["state"] == k).count();
        out["shop"] = json!({
            "open": s.open, "opened": views::date_text(s.opened), "closed": s.closed, "leaked": s.leaked,
            "ask": ask_key(s.ask), "paydown": s.paydown, "agent": s.agent,
            "bids": bids, "log": log, "closing": closing,
            "count": { "offers": counts("offer"), "waiting": counts("waiting"),
                       "passed": counts("passed") + counts("dropped") + counts("ended") + counts("lapsed"),
                       "called": s.bids.len() },
        });
        // The talks with one club: his picks to ask for, the rivals to
        // name (true, or a bluff with your staff's read of how believable).
        let talk = args.get("club").and_then(Value::as_u64).map(|c| c as usize).or_else(|| {
            s.bids
                .iter()
                .filter(|b| b.state == finder::State::Offer)
                .max_by(|a, b| a.bid.total_cmp(&b.bid))
                .map(|b| b.club)
        });
        if let Some(c) = talk.filter(|&c| c < w.nfl.len()) {
            let theirs: Vec<Value> = deals::picks_of(w, c).iter().map(|p| pick_json(w, p)).collect();
            let me_bid = s.bids.iter().find(|b| b.club == c);
            // Any club can be named: one with a higher offer in hand is
            // the truth, the rest are bluffs, the likeliest believed first.
            let mut rivals: Vec<(bool, f64, Value)> = afm_engine::world::franchise::active_teams_in(w, w.year)
                .into_iter()
                .filter(|&r| r != c && r != me)
                .map(|r| {
                    let bid = s.bids.iter().find(|b| b.club == r && b.state == finder::State::Offer);
                    let truth = bid.is_some_and(|rb| me_bid.is_some_and(|b| rb.bid >= b.bid));
                    let believe = if truth { 100.0 } else { (finder::believable(w, c, r, i) * 100.0).round() };
                    let v = json!({
                        "club": r, "team": views::team_brief(w, TeamRef::Nfl(r)),
                        "true": truth,
                        "believe": believe,
                        "package": bid.and_then(|b| b.package.as_ref()).map(|p| package_json(w, p, r)),
                    });
                    (truth, believe, v)
                })
                .collect();
            rivals.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.total_cmp(&a.1)));
            let rivals: Vec<Value> = rivals.into_iter().map(|x| x.2).collect();
            out["talk"] = json!({
                "club": c, "team": views::team_brief(w, TeamRef::Nfl(c)), "gm": w.clubs[c].office.gm.name,
                "picks": theirs, "rivals": rivals,
                "lines": s.log.iter().filter(|l| l.club == Some(c)).map(|l| json!({ "date": views::date_text(l.date), "text": l.text, "you": l.you })).collect::<Vec<_>>(),
            });
        }
    }
    if flag(args, "move") && ours {
        let (worth, list) = finder::dumps(w, pid)?;
        let release = k.map(|k| {
            let june1 = pro::after_june1(w, w.today);
            let (now, later) = k.dead_if_cut(year, w.today, june1);
            json!({ "dead": now.round(), "later": later.round(), "saved": (k.cap_hit(year) - now).round() })
        });
        out["move"] = json!({
            "worth": worth.round(),
            "options": list.iter().map(|d| json!({
                "club": d.club, "team": views::team_brief(w, TeamRef::Nfl(d.club)), "kind": d.kind,
                "picks": d.picks.iter().map(|p| pick_json(w, p)).collect::<Vec<_>>(),
                "paydown": d.paydown, "back": d.back.and_then(|id| fo::index_of(w, id)).map(|j| prow(w, j)),
                "cost": d.cost.round(), "dead": d.dead.round(), "saved": d.saved.round(),
            })).collect::<Vec<_>>(),
            "release": release,
        });
    }
    if let Some(l) = w.scouting.live.as_ref() {
        let at = l.run.next as usize - 1;
        let mine_next = (at..l.run.board.len()).find(|&s| l.run.board[s].0 == me);
        let strip: Vec<Value> = (at.saturating_sub(2)..(at + 9).min(l.run.board.len()))
            .map(|s| json!({ "slot": s, "pick": s + 1, "round": l.run.board[s].1, "team": views::team_brief(w, TeamRef::Nfl(l.run.board[s].0)), "mine": l.run.board[s].0 == me, "now": s == at }))
            .collect();
        let quotes: Vec<Value> = mine_next
            .map(|m| (at..m).filter_map(|s| live::quote(w, l, s).map(|q| (s, q))).take(5).collect::<Vec<_>>())
            .unwrap_or_default()
            .iter()
            .map(|(s, q)| {
                let mut v = crate::lane_scouting::offer_json(w, l, q, 0);
                v["slot"] = json!(s);
                v["pick"] = json!(s + 1);
                v
            })
            .collect();
        let nights: Vec<Value> = if flag(args, "night") && ours {
            finder::night_offers(w, pid)
                .unwrap_or_default()
                .iter()
                .map(|n| {
                    json!({
                        "club": n.club, "team": views::team_brief(w, TeamRef::Nfl(n.club)),
                        "slots": n.slots.iter().map(|s| s + 1).collect::<Vec<_>>(),
                        "worth": n.worth.round(),
                    })
                })
                .collect()
        } else {
            vec![]
        };
        out["draft"] = json!({
            "on_clock": live::on_clock(l), "pick": at + 1, "round": l.run.board.get(at).map(|x| x.1),
            "my_next": mine_next.map(|s| s + 1),
            "strip": strip,
            "offers": l.offers.iter().enumerate().map(|(k, o)| crate::lane_scouting::offer_json(w, l, o, k)).collect::<Vec<_>>(),
            "quotes": quotes,
            "nights": nights,
        });
    }
    let _ = money::capped(year);
    Ok(out)
}

fn record(w: &World, t: usize) -> String {
    bargain::record(w, t)
        .map_or(String::new(), |(wn, l, tie)| if tie > 0 { format!("{wn}-{l}-{tie}") } else { format!("{wn}-{l}") })
}

/// His lift over a club's weakest starter at his spot, in words.
fn need_words(n: f64) -> &'static str {
    if n >= 8.0 {
        "Big hole"
    } else if n >= 2.0 {
        "Starter wanted"
    } else if n >= -3.0 {
        "Depth only"
    } else {
        "Set there"
    }
}
