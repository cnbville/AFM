//! Ground's leads and finds (lane G4): the commands the lead card (board
//! 4b, lane S4) and the ground report (lane S8) read. Every rating is your
//! staff's read; potential stays in words; the teller's eye and
//! straightness show only as his record.
//!
//! - `ground_leads`: your open leads, newest first.
//! - `ground_lead` (`id`): one lead's card: the kid, who told you and what
//!   he said, his record, the five blanks, the news and its order, who
//!   else knows, your word with what each risks, the bloom steps with what
//!   each pays, and your man's record.
//! - `ground_fill` (`id`, `blank`: size, speed, tape, live, character):
//!   asks for the look that fills a blank.
//! - `ground_lead_drop` (`id`).
//! - `ground_finds`: your finds with their blooms and setbacks, your
//!   misses, your men's records and your name as a finder.

use afm_engine::calendar::Date;
use afm_engine::world::ground::finds::{self, Find};
use afm_engine::world::ground::leads::{self, Blank, Lead};
use afm_engine::world::ground::news::{self, News};
use afm_engine::world::ground::people::{self, Who};
use afm_engine::world::ground::standing::{self, Level};
use afm_engine::world::scouting::{self as sc, readers, services};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::reads;
use crate::views::date_text;
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let id = || args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
    Some(match cmd {
        "ground_leads" => game.w().map(list),
        "ground_lead" => (|| {
            let pid = id()?;
            card(game.w()?, pid)
        })(),
        "ground_fill" => (|| {
            let pid = id()?;
            let b = args.get("blank").and_then(Value::as_str).and_then(Blank::from_key).ok_or("Missing blank.")?;
            let w = game.wm()?;
            Ok(json!({ "text": leads::fill(w, pid, b)? }))
        })(),
        "ground_lead_drop" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            Ok(json!({ "text": leads::drop(w, pid)? }))
        })(),
        "ground_finds" => game.w().map(finds_page),
        _ => return None,
    })
}

fn your_program(w: &World) -> Option<usize> {
    match readers::seat_staff(w)? {
        readers::Staff::Program(p) => Some(p),
        _ => None,
    }
}

fn iso(d: Date) -> String {
    let (y, m, dd) = d.parts();
    format!("{y:04}-{m:02}-{dd:02}")
}

fn level_name(k: u8) -> &'static str {
    Level::ALL[(k as usize).min(3)].name()
}

/// A person on your ground, as the card shows him: name, role, your
/// standing and its why, and his record with you.
fn person(w: &World, who: Who) -> Value {
    let prog = your_program(w);
    json!({
        "key": who_key(who),
        "name": people::name(w, who),
        "role": people::role(w, who),
        "level": prog.map(|p| standing::level(w, p, who).name()),
        "why": prog.map(|p| standing::why(w, p, who)),
        "record": leads::record_words(w, who),
        "held": leads::record(w, who).0,
        "of": leads::record(w, who).1,
    })
}

/// The person's key, the one every ground screen uses.
fn who_key(who: Who) -> String {
    crate::lane_groundweek::who_key(who)
}

/// A man of yours with his record as a finder.
fn man(w: &World, c: Option<u32>) -> Value {
    c.and_then(|c| w.coaches.get(c as usize).map(|x| (c, x))).map_or(Value::Null, |(c, x)| {
        let m = finds::man(w, c);
        json!({ "id": c, "name": x.name, "finds": m.finds, "blooms": m.blooms, "big": m.big })
    })
}

fn kid(w: &World, i: usize) -> Value {
    let p = &w.people[i];
    json!({
        "id": p.player.id,
        "name": p.name,
        "pos": p.player.position.name(),
        "class_of": p.class_of,
        "school": w.school_name(p.hs),
        "school_id": p.hs,
        "state": p.home_state,
        "stars": p.stars,
        "offers": p.cal.offers.iter().map(|&j| w.program_name(j)).collect::<Vec<_>>(),
        "staff": reads::overall(w, i).json(),
        "potential": reads::potential_words(w, i),
    })
}

fn row(w: &World, l: &Lead) -> Option<Value> {
    let i = sc::place(w, l.kid)?;
    let filled = leads::blanks(w, l.kid).iter().filter(|f| f.filled.is_some()).count();
    Some(json!({
        "kid": kid(w, i),
        "since": date_text(l.since),
        "how": l.how.key(),
        "how_words": l.how.words(),
        "from": l.from.map(|who| people::name(w, who)).or(l.tipper.clone()),
        "claim": l.claim.map(|c| afm_engine::world::folk::tips::CLAIMS[c as usize]),
        "filled": filled,
        "ready": filled == 5,
        "find": w.ground.finds.iter().any(|f| f.kid == l.kid),
        "word": l.word,
        "news": l.news.len(),
    }))
}

fn list(w: &World) -> Value {
    let rows: Vec<Value> = leads::open(w).into_iter().filter_map(|l| row(w, l)).collect();
    json!({ "leads": rows, "finder": finds::finder_words(w), "today": iso(w.today) })
}

fn news_json(w: &World, n: &News) -> Value {
    json!({
        "what": n.what.key(),
        "line": n.line,
        "date": date_text(n.date),
        "iso": iso(n.date),
        "from": n.source.map(|s| people::name(w, s)),
        "order": n.order.iter().map(|h| json!({
            "program": w.program_name(h.prog as usize),
            "program_id": h.prog,
            "day": news::when(h.day, n.date),
            "iso": iso(h.day),
            "level": level_name(h.level),
            "why": h.why.key(),
            "you": n.place > 0 && h.prog == n.you.prog,
        })).collect::<Vec<_>>(),
        "before_public": n.before_public,
        "public": news::when(n.public, n.date),
        "public_iso": iso(n.public),
        "you": { "day": news::when(n.you.day, n.date), "iso": iso(n.you.day), "place": n.place, "why": n.you.why.key() },
        "order_words": news::order_words(w, n),
        "why_words": news::why_words(w, n),
    })
}

/// Your word on him (board 4b): offer now, wait for his junior tape, fill
/// the last blank first, each with what it risks; and what each bloom step
/// would pay if you offered today.
fn word(w: &World, i: usize, open: &[Blank]) -> Value {
    let p = &w.people[i];
    let Some(prog) = your_program(w) else { return Value::Null };
    let line = if w.programs[prog].division == afm_engine::world::data::Division::FBS { 3 } else { 2 };
    let others: usize = {
        let mut v: Vec<usize> = p.cal.offers.iter().chain(p.cal.early_offers.iter().map(|x| &x.0)).copied().collect();
        v.sort_unstable();
        v.dedup();
        v.retain(|&j| j != prog);
        v.len()
    };
    let low = p.stars <= line && others <= 5;
    let pay = finds::pay(p.stars, line, others);
    let offered = p.cal.offers.contains(&prog) || p.cal.early_offers.iter().any(|o| o.0 == prog);
    let mut options = Vec::new();
    if !offered {
        options.push(json!({
            "key": "offer",
            "label": "Offer now",
            "risk": if low {
                format!("He becomes your find. A scholarship spent with {} of five blanks open.", open.len())
            } else {
                "The world rates him already: an offer now isn't a find.".to_string()
            },
        }));
    }
    if p.class_of - w.dates.season >= 2 {
        options.push(json!({
            "key": "wait",
            "label": "Wait for his junior tape",
            "risk": "Another staff may offer first, and he'd be their find.",
        }));
    }
    if let Some(b) = open.first() {
        let (what, time) = b.fills();
        options.push(json!({
            "key": "fill",
            "blank": b.key(),
            "label": format!("Fill {} first", b.name().to_lowercase()),
            "risk": format!("{what} ({time}). The services may rate him before it fills."),
        }));
    }
    let steps = [
        ("small", "A fourth star, or an offer from a level up"),
        ("bloom", "A starting job by his second year, or all-conference"),
        ("big", "Drafted, or All-American"),
    ];
    let size = ["a little", "more", "a lot"];
    json!({
        "offered": offered,
        "options": options,
        "find_if_offered": low,
        "pays": if low { pay_words(pay) } else { String::new() },
        "steps": steps.iter().enumerate().map(|(k, (key, what))| json!({
            "key": key,
            "what": what,
            "pays": if low {
                format!(
                    "The one who told you trusts you {}; your name as a finder grows {}.",
                    size[k],
                    size[k]
                )
            } else {
                String::new()
            },
        })).collect::<Vec<_>>(),
    })
}

/// What a find pays, in words: the lower the world rated him, the more.
fn pay_words(pay: f32) -> String {
    if pay >= 0.85 {
        "Nobody rated him: a bloom pays in full.".into()
    } else if pay >= 0.6 {
        "Few rated him: a bloom pays well.".into()
    } else {
        "Some rated him already: a bloom pays a little.".into()
    }
}

fn find_json(w: &World, f: &Find) -> Value {
    json!({
        "kid": f.kid,
        "name": f.name,
        "pos": f.pos.name(),
        "school": w.school_name(f.school as usize),
        "class_of": f.class_of,
        "date": date_text(f.date),
        "stars": f.stars,
        "offers": f.offers,
        "pays": pay_words(f.pay),
        "teller": f.teller.map(|t| people::name(w, t)).or(f.tipper.clone()),
        "man": man(w, f.man),
        "steps": f.steps,
        "blooms": f.blooms.iter().map(|b| json!({ "date": date_text(b.date), "step": b.step, "words": b.words })).collect::<Vec<_>>(),
        "lost": f.lost.map(|(d, j)| json!({ "date": date_text(d), "program": w.program_name(j as usize) })),
        "setback": f.setback.as_ref().map(|(d, k, s)| json!({ "date": date_text(*d), "kind": k.key(), "words": s })),
    })
}

/// The look that filled a blank, by its kind's name ("Your camp", "Calls").
fn look_kind(w: &World, id: u32, d: Date, code: u32) -> Option<&'static str> {
    w.scouting.dept.looks.get(&id)?.iter().find(|l| l.date == d && l.who == code).map(|l| l.kind.name())
}

/// A program's marks on a kid: the day (if known) and what happened.
type Marks = Vec<(Option<Date>, String)>;

/// The race in signs (GR73): the three programs that show most on him, each
/// with the days it heard (the order the news went) or offered, and your own
/// days: when you heard and each look that filled a blank. Dates are ISO;
/// an offer with no date is a mark with `iso: null`.
fn race(w: &World, id: u32, you: Option<usize>, blanks: &[leads::Filled; 5], filler: &dyn Fn(u32) -> String) -> Value {
    use std::collections::BTreeMap;
    let mut by: BTreeMap<usize, Marks> = BTreeMap::new();
    for n in leads::news_of(w, id) {
        for h in &n.order {
            let j = h.prog as usize;
            if Some(j) != you {
                by.entry(j).or_default().push((Some(h.day), format!("heard {}", n.what.words())));
            }
        }
    }
    if let Some(i) = sc::place(w, id) {
        let p = &w.people[i];
        for &(j, d) in &p.cal.early_offers {
            if Some(j) != you {
                by.entry(j).or_default().push((Some(d), "offered".into()));
            }
        }
        for &j in &p.cal.offers {
            let dated = p.cal.early_offers.iter().any(|o| o.0 == j);
            if Some(j) != you && !dated {
                by.entry(j).or_default().push((None, "offered".into()));
            }
        }
    }
    let mut rivals: Vec<(usize, Marks)> = by.into_iter().collect();
    for r in &mut rivals {
        r.1.sort_by_key(|m| m.0);
        r.1.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    }
    rivals.sort_by_key(|r| (!r.1.iter().any(|m| m.1 == "offered"), r.1.iter().filter_map(|m| m.0).min()));
    rivals.truncate(3);
    let mut mine: Vec<Value> = Vec::new();
    if let Some(l) = leads::lead(w, id) {
        mine.push(json!({ "iso": iso(l.since), "what": "you heard", "by": l.from.map(|who| people::name(w, who)).or(l.tipper.clone()) }));
    }
    if let (Some(j), Some(i)) = (you, sc::place(w, id))
        && let Some(&(_, d)) = w.people[i].cal.early_offers.iter().find(|o| o.0 == j)
    {
        mine.push(json!({ "iso": iso(d), "what": "your offer", "by": Value::Null }));
    }
    for f in blanks {
        if let Some((d, code)) = f.filled {
            mine.push(json!({
                "iso": iso(d),
                "what": look_kind(w, id, d, code).unwrap_or("a look"),
                "blank": f.blank.name(),
                "by": filler(code),
            }));
        }
    }
    mine.sort_by(|a, b| a["iso"].as_str().cmp(&b["iso"].as_str()));
    json!({
        "rivals": rivals.iter().map(|(j, ms)| json!({
            "program": w.program_name(*j),
            "id": j,
            "marks": ms.iter().map(|m| json!({ "iso": m.0.map(iso), "what": m.1 })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "you": mine,
    })
}

fn card(w: &World, id: u32) -> Result<Value, String> {
    let i = sc::place(w, id).ok_or("No such player.")?;
    let l = leads::lead(w, id);
    let p = &w.people[i];
    let blanks = leads::blanks(w, id);
    let staff = your_program(w).map(readers::Staff::Program);
    let group = afm_engine::world::coach::Group::of(p.player.position) as usize;
    let filler = |code: u32| {
        staff.and_then(|s| readers::reader(w, s, code, group)).map(|r| r.name).unwrap_or_else(|| "Your staff".into())
    };
    let open: Vec<Blank> = blanks.iter().filter(|f| f.filled.is_none()).map(|f| f.blank).collect();
    let measured = blanks.iter().any(|f| f.blank == Blank::Size && f.filled.is_some());
    let body = afm_engine::body::body_now(&p.player);
    let rated = services::class_ranks(w, p.class_of).is_some_and(|c| c.ids.contains(&id));
    let next = services::next_update(w.today);
    let prog_of_you = your_program(w);
    let offers_all: Vec<&str> = {
        let mut v: Vec<usize> = p.cal.offers.iter().chain(p.cal.early_offers.iter().map(|o| &o.0)).copied().collect();
        v.sort_unstable();
        v.dedup();
        v.iter().map(|&j| w.program_name(j)).collect()
    };
    let times: Vec<Value> = w
        .times
        .of(id)
        .iter()
        .map(|t| json!({ "date": date_text(t.date), "event": format!("{:?}", t.event), "timed": format!("{:?}", t.timed), "secs": t.secs, "lb": t.lb }))
        .collect();
    Ok(json!({
        "kid": kid(w, i),
        "lead": l.map(|l| json!({
            "since": date_text(l.since),
            "since_iso": iso(l.since),
            "how": l.how.key(),
            "how_words": l.how.words(),
            "from": l.from.map(|who| person(w, who)),
            "tipper": l.tipper,
            "level": level_name(l.level),
            "said": l.said,
            "claim": l.claim.map(|c| afm_engine::world::folk::tips::CLAIMS[c as usize]),
            "said_height": l.height.map(|h| afm_engine::body::height_text((h as f64 * 2.0).round() / 2.0)),
            "said_forty": l.forty,
            "word": l.word,
            "man": man(w, l.man),
            "dropped": l.dropped,
        })),
        "growth": {
            "measured": measured.then(|| json!({
                "height": afm_engine::body::height_text(body.height.round()),
                "height_in": body.height.round(),
                "weight": body.weight.round(),
            })),
            // A year back on his own growth curve: shown only once your
            // staff has measured him (the same curve the spring weigh-in's
            // news reads).
            "then": measured.then(|| {
                let age = p.player.age as f64 + f64::from(p.player.grow);
                let a = p.player.body;
                let h = (a.height * afm_engine::body::height_share(age - 1.0, p.player.bloom)).round();
                json!({
                    "height": afm_engine::body::height_text(h),
                    "height_in": h,
                    "weight": (a.weight * afm_engine::body::fill_share(&p.player, age - 1.0)).round(),
                })
            }),
            "times": times,
        },
        "services": {
            "rated": rated,
            "stars": rated.then_some(p.stars),
            "next": { "date": iso(next.0), "season": next.1 },
        },
        "today": iso(w.today),
        "offers_all": offers_all,
        "race": race(w, id, prog_of_you, &blanks, &filler),
        "blanks": blanks.iter().map(|f| {
            let (what, time) = f.blank.fills();
            json!({
                "key": f.blank.key(),
                "name": f.blank.name(),
                "filled": f.filled.map(|(d, code)| json!({
                    "date": date_text(d),
                    "iso": iso(d),
                    "by": filler(code),
                    "how": look_kind(w, id, d, code),
                })),
                "asked": f.asked,
                "fills": what,
                "time": time,
            })
        }).collect::<Vec<_>>(),
        "news": leads::news_of(w, id).into_iter().map(|n| news_json(w, n)).collect::<Vec<_>>(),
        "who_else": leads::who_else(w, id).into_iter().map(|(j, what)| json!({
            "program": w.program_name(j),
            "program_id": j,
            "what": what,
        })).collect::<Vec<_>>(),
        "find": w.ground.finds.iter().find(|f| f.kid == id).map(|f| find_json(w, f)),
        "word": if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) { word(w, i, &open) } else { Value::Null },
    }))
}

fn finds_page(w: &World) -> Value {
    json!({
        "finds": w.ground.finds.iter().map(|f| find_json(w, f)).collect::<Vec<_>>(),
        "misses": w.ground.misses.iter().map(|m| json!({
            "kid": m.kid,
            "name": m.name,
            "pos": m.pos.name(),
            "school": w.school_name(m.school as usize),
            "class_of": m.class_of,
            "date": date_text(m.date),
            "went": m.went.map(|j| w.program_name(j as usize)),
            "what": m.what,
            "coach": m.coach.map(|c| people::name(w, c)),
            "level": level_name(m.level),
            "why": m.why,
        })).collect::<Vec<_>>(),
        "men": w.ground.men.iter().map(|m| json!({
            "id": m.coach,
            "name": w.coaches.get(m.coach as usize).map(|c| c.name.clone()),
            "finds": m.finds,
            "blooms": m.blooms,
            "big": m.big,
        })).collect::<Vec<_>>(),
        "finder": finds::finder_words(w),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Game;

    /// A small world's college seat through its spring: news reaches your
    /// staff in order, leads come from work, and every card reads whole.
    #[test]
    fn a_college_spring_brings_leads_whose_cards_read_whole() {
        let dir = std::env::temp_dir().join(format!("afm-leads-{}", std::process::id()));
        let mut g = Game::new(dir.clone());
        g.call("build_world", &json!({ "seed": 181, "history_years": 1, "scale": 0.03, "engine_from": 2026 })).unwrap();
        let job = g.call("jobs", &json!({ "level": "college" })).unwrap()["jobs"][0]["ref"].clone();
        g.call(
            "start_career",
            &json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9 }),
        )
        .unwrap();
        let t = std::time::Instant::now();
        {
            let w = g.wm().unwrap();
            let start = w.today;
            let end = afm_engine::calendar::Date::ymd(start.year(), 7, 1).max(start.add(150));
            while w.today < end {
                w.advance_day();
            }
            let gr = &w.ground;
            let mut how: std::collections::BTreeMap<&str, usize> = Default::default();
            for l in &gr.leads {
                *how.entry(l.how.key()).or_default() += 1;
            }
            println!(
                "{} to {} in {:.0}s: {} news ({} delivered), {} leads {:?}, {} finds, {} records",
                start.pretty(),
                w.today.pretty(),
                t.elapsed().as_secs_f64(),
                gr.news.len(),
                gr.news.iter().filter(|n| n.delivered).count(),
                gr.leads.len(),
                how,
                gr.finds.len(),
                gr.records.len()
            );
            for n in gr.news.iter().take(6) {
                println!("  {} | {}", n.line, afm_engine::world::ground::news::order_words(w, n));
            }
        }
        let list = g.call("ground_leads", &json!({})).unwrap();
        let rows = list["leads"].as_array().unwrap();
        assert!(!rows.is_empty(), "a spring of work brings leads");
        for r in rows.iter().take(25) {
            let id = r["kid"]["id"].as_u64().unwrap();
            let c = g.call("ground_lead", &json!({ "id": id })).unwrap();
            let text = c.to_string();
            assert!(!text.contains("NaN") && !text.contains("{}"), "{text}");
            assert_eq!(c["blanks"].as_array().unwrap().len(), 5);
            // What board 4b draws: the services' next update, the race's marks, your days.
            assert!(c["services"]["next"]["date"].is_string());
            assert!(c["race"]["you"].as_array().is_some_and(|y| !y.is_empty()), "you heard on a day");
            assert!(c["race"]["rivals"].is_array() && c["today"].is_string());
        }
        let first = rows[0]["kid"]["id"].as_u64().unwrap();
        println!("{}", serde_json::to_string_pretty(&g.call("ground_lead", &json!({ "id": first })).unwrap()).unwrap());
        let open = g.call("ground_lead", &json!({ "id": first })).unwrap()["blanks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["filled"].is_null() && !b["asked"].as_bool().unwrap_or(false))
            .map(|b| b["key"].as_str().unwrap().to_string());
        if let Some(k) = open {
            let r = g.call("ground_fill", &json!({ "id": first, "blank": k })).unwrap();
            assert!(r["text"].as_str().is_some_and(|s| !s.is_empty()));
        }
        // The card's offer button sends the desk's offer; once it is out the word drops the option.
        let id = first as u32;
        let had = |c: &serde_json::Value| {
            c["word"]["options"].as_array().is_some_and(|o| o.iter().any(|x| x["key"] == "offer"))
        };
        let before = g.call("ground_lead", &json!({ "id": id })).unwrap();
        if had(&before) && g.call("recruit_offer", &json!({ "id": id })).is_ok() {
            let after = g.call("ground_lead", &json!({ "id": id })).unwrap();
            assert!(after["word"]["offered"].as_bool().unwrap_or(false), "offered shows on the card");
            assert!(!had(&after), "an offer you've made isn't offered twice");
        }
        assert!(g.call("ground_finds", &json!({})).unwrap()["finder"].is_string());
        assert!(g.call("ground_lead_drop", &json!({ "id": first })).is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }
}
