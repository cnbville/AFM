//! Ground's offseason campaign (lane G6): the commands the ground report
//! (board 8b, lane S8) and the campaign's own screens read
//! (`afm_engine::world::ground::campaign`). Words only: every rating is
//! your staff's read, chances are said in words, potential stays hidden.
//!
//! - `ground_campaign`: this spring's goals by slot (each with its reading,
//!   its verdict on the scale so far, how likely it looked when you picked
//!   it, bold, who knows), your AD's wishes, the guide (his three schools,
//!   the tools open), the spring's dates and whether today is a Monday
//!   meeting or media days.
//! - `ground_goals`: the list of 63 by group, each with who raises it, its
//!   classes and how likely it looks for your class today.
//! - `ground_goal_pick` {goal (number or key), n, of}: your own goal (the
//!   hunt is goal 44); `ground_goal_drop` {slot: yours, hunt};
//!   `ground_hunt` {on}; `ground_goal_tell` {slot} (your AD);
//!   `ground_goal_name` {slot} (media days).
//! - `ground_guide_go`, `ground_guide_off`.
//! - `ground_report` {year}: the verdict's report (the newest by default).

use afm_engine::world::World;
use afm_engine::world::ground::campaign::{self, Held, Slot, goals};
use serde_json::{Value, json};

use crate::views::date_text;
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "ground_campaign" => game.w().and_then(view),
        "ground_goals" => game.w().and_then(catalogue),
        "ground_goal_pick" => game.wm().and_then(|w| {
            let g = goal_arg(args.get("goal")).ok_or("Missing the goal.")?;
            let d = goals::def(g);
            let n = args.get("n").and_then(Value::as_u64).map_or(d.n, |x| x as u32);
            let of = args.get("of").and_then(Value::as_u64).map_or(0, |x| x as u32);
            let class = w.ground.campaign.class;
            let said = campaign::pick(w, goals::Aim { goal: g, n, of, class })?;
            said_view(w, said)
        }),
        "ground_goal_drop" => game.wm().and_then(|w| {
            let slot = slot_arg(args.get("slot")).ok_or("Missing the slot.")?;
            let said = campaign::drop(w, slot)?;
            said_view(w, said)
        }),
        "ground_hunt" => game.wm().and_then(|w| {
            let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
            let said = campaign::hunt(w, on)?;
            said_view(w, said)
        }),
        "ground_goal_tell" => game.wm().and_then(|w| {
            let slot = slot_arg(args.get("slot")).ok_or("Missing the slot.")?;
            let said = campaign::tell_ad(w, slot)?;
            said_view(w, said)
        }),
        "ground_goal_name" => game.wm().and_then(|w| {
            let slot = slot_arg(args.get("slot")).ok_or("Missing the slot.")?;
            let said = campaign::name_at_media_days(w, slot)?;
            said_view(w, said)
        }),
        "ground_guide_go" => game.wm().and_then(|w| {
            let said = campaign::guide_go(w)?;
            said_view(w, said)
        }),
        "ground_guide_off" => game.wm().and_then(|w| {
            let said = campaign::guide_off(w)?;
            said_view(w, said)
        }),
        "ground_report" => game.w().and_then(|w| {
            let year = args.get("year").and_then(Value::as_i64).map(|x| x as i32);
            report(w, year)
        }),
        _ => return None,
    })
}

fn goal_arg(v: Option<&Value>) -> Option<u8> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return (1..=63).contains(&n).then_some(n as u8);
    }
    let s = v.as_str()?;
    s.parse::<u8>().ok().filter(|n| (1..=63).contains(n)).or_else(|| goals::by_key(s).map(|d| d.id))
}

fn slot_arg(v: Option<&Value>) -> Option<Slot> {
    match v?.as_str()? {
        "needs" => Some(Slot::Needs),
        "ad" => Some(Slot::Ad),
        "yours" => Some(Slot::Yours),
        "hunt" => Some(Slot::Hunt),
        _ => None,
    }
}

fn said_view(w: &World, said: String) -> Reply {
    let mut v = view(w)?;
    v["said"] = json!(said);
    Ok(v)
}

fn held_row(w: &World, prog: usize, h: &Held, class: i32) -> Value {
    let r = goals::read(w, prog, &h.aim, class);
    let d = h.aim.def();
    let parts: Vec<f32> = h.parts.iter().filter_map(|p| p.result.or(p.pace)).collect();
    let share = if h.dropped.is_some() {
        0.0
    } else if d.classes > 1 {
        goals::share_over(&parts, d.classes).max(r.share / d.classes as f32)
    } else {
        r.share
    };
    json!({
        "slot": h.slot.key(),
        "slot_words": h.slot.words(),
        "goal": d.id,
        "key": d.key,
        "group": d.group.words(),
        "words": goals::words(w, &h.aim),
        "judged": d.judged,
        "reading": r.words,
        "waiting": r.waiting,
        "verdict": if r.waiting { "Waiting".to_string() } else { goals::Verdict::of(share).words().to_string() },
        "classes": d.classes,
        "later": d.later,
        "parts": h.parts.iter().map(|p| json!({
            "class": p.class,
            "pace": p.pace.map(|x| goals::Verdict::of(x).words()),
            "result": p.result.map(|x| goals::Verdict::of(x).words()),
            "words": p.words,
        })).collect::<Vec<_>>(),
        "likely": campaign::chance_words(h.likely),
        "bold": h.bold > 0.0,
        "by": h.by,
        "why": h.why,
        "told": { "staff": true, "ad": h.told.ad, "press": h.told.press },
        "set_on": date_text(h.set_on),
        "dropped": h.dropped.map(date_text),
        "settled": h.settled,
    })
}

fn view(w: &World) -> Reply {
    let prog = campaign::seat(w).ok_or("The campaign is for a college head coach.")?;
    let c = &w.ground.campaign;
    let y = w.today.year();
    let s = campaign::Spring::of(y);
    let verdict = campaign::verdict_day(w, prog, y);
    let class = c.class.max(campaign::class_now(w));
    let goals_now: Vec<Value> = c
        .goals
        .iter()
        .filter(|h| h.year == c.year || (h.open() && h.aim.classes().any(|x| x == class)))
        .map(|h| held_row(w, prog, h, if h.aim.classes().any(|x| x == class) { class } else { h.aim.class }))
        .collect();
    let past: Vec<Value> =
        c.goals.iter().filter(|h| h.year < c.year && !h.open()).map(|h| held_row(w, prog, h, h.aim.last())).collect();
    let wishes: Vec<Value> = c
        .wishes
        .iter()
        .map(|m| {
            json!({
                "words": goals::words(w, &m.aim),
                "why": m.why,
                "who": m.who,
                "reading": goals::read(w, prog, &m.aim, m.aim.class).words,
            })
        })
        .collect();
    let g = &c.guide;
    let schools: Vec<Value> =
        g.schools.iter().map(|&h| json!({ "id": h, "name": w.school_name(h as usize) })).collect();
    Ok(json!({
        "year": c.year,
        "class": class,
        "started": c.year == y,
        "goals": goals_now,
        "past": past,
        "wishes": wishes,
        "guide": {
            "on": g.on,
            "off": g.off,
            "door": g.door.map(date_text),
            "area": g.area.map(|a| afm_engine::world::ground::areas::area(a).name),
            "schools": schools,
            "went": g.went,
            "tools": campaign::TOOLS.iter().map(|t| json!({ "tool": t, "open": campaign::tool_open(w, t) })).collect::<Vec<_>>(),
        },
        "dates": {
            "signing_day": date_text(s.signing),
            "ad_talk": date_text(s.talk),
            "meeting": date_text(s.meeting),
            "media_days": [date_text(s.media.0), date_text(s.media.1)],
            "verdict": date_text(verdict),
        },
        "monday": w.today.weekday() == afm_engine::calendar::Weekday::Mon || w.today == s.meeting,
        "media_days": w.today >= s.media.0 && w.today <= s.media.1,
        "reports": c.reports.iter().map(|r| json!({ "year": r.year, "title": r.title })).collect::<Vec<_>>(),
    }))
}

fn catalogue(w: &World) -> Reply {
    campaign::seat(w).ok_or("The campaign is for a college head coach.")?;
    let rows: Vec<Value> = campaign::catalogue(w)
        .into_iter()
        .map(|(d, p)| {
            json!({
                "goal": d.id,
                "key": d.key,
                "group": d.group.words(),
                "name": d.name.replace("{n}", "N"),
                "n": d.n,
                "classes": d.classes,
                "later": d.later,
                "judged": d.judged,
                "raised_by": d.raised.iter().map(|r| r.words()).collect::<Vec<_>>(),
                "about": format!("{:?}", d.subj).to_lowercase(),
                "single": d.single,
                "likely": if p > 0.0 { Some(campaign::chance_words(p)) } else { None },
            })
        })
        .collect();
    Ok(json!({ "goals": rows }))
}

fn report(w: &World, year: Option<i32>) -> Reply {
    campaign::seat(w).ok_or("The campaign is for a college head coach.")?;
    let c = &w.ground.campaign;
    let r = match year {
        Some(y) => c.reports.iter().find(|r| r.year == y),
        None => c.reports.last(),
    };
    let Some(r) = r else {
        let v = campaign::verdict_day(w, campaign::seat(w).unwrap_or(0), w.today.year());
        return Ok(json!({ "ready": false, "verdict": date_text(v) }));
    };
    let mut v = serde_json::to_value(r).map_err(|e| e.to_string())?;
    v["ready"] = json!(true);
    v["date"] = json!(date_text(r.date));
    if let Some(gs) = v["goals"].as_array_mut() {
        for (g, line) in gs.iter_mut().zip(r.goals.iter()) {
            g["verdict"] = json!(line.verdict.words());
            g["slot"] = json!(line.slot.key());
        }
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Game;

    /// A small world's college head coach from his first January to the
    /// verdict: the AD's talk, the February meeting, a Monday pick, and a
    /// report that reads whole.
    #[test]
    fn a_first_spring_runs_from_the_meeting_to_the_report() {
        let dir = std::env::temp_dir().join(format!("afm-campaign-{}", std::process::id()));
        let mut g = Game::new(dir.clone());
        g.call("build_world", &json!({ "seed": 191, "history_years": 1, "scale": 0.03, "engine_from": 2026 })).unwrap();
        let job = g.call("jobs", &json!({ "level": "college" })).unwrap()["jobs"][0]["ref"].clone();
        g.call(
            "start_career",
            &json!({ "name": "Test Coach", "age": 46, "background": "college_assistant", "job": job, "seed": 9 }),
        )
        .unwrap();
        let w = g.wm().unwrap();
        if afm_engine::world::ground::campaign::seat(w).is_none() {
            println!("the first job isn't a head coach's: nothing to check");
            return;
        }
        let year = w.today.year();
        let s = afm_engine::world::ground::campaign::Spring::of(year);
        println!("start {}, door {}, meeting {}", w.today.pretty(), s.door.pretty(), s.meeting.pretty());
        // Through the meeting's day, then to the next Monday's meeting.
        while w.today <= s.meeting || w.today.weekday() != afm_engine::calendar::Weekday::Mon {
            w.advance_day();
        }
        println!("guide door {:?}", w.ground.campaign.guide.door.map(|d| d.pretty()));
        let v = g.call("ground_campaign", &json!({})).unwrap();
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        assert_eq!(v["year"], json!(year));
        let slots: Vec<&str> = v["goals"].as_array().unwrap().iter().filter_map(|h| h["slot"].as_str()).collect();
        assert!(slots.contains(&"needs") && slots.contains(&"ad") && slots.contains(&"yours"), "{slots:?}");
        let list = g.call("ground_goals", &json!({})).unwrap();
        assert_eq!(list["goals"].as_array().unwrap().len(), 63);
        // A Monday pick: five blue-chips is goal 5.
        let p = g.call("ground_goal_pick", &json!({ "goal": 5, "n": 2 })).unwrap();
        assert!(p["said"].as_str().unwrap().contains("blue"), "{}", p["said"]);
        g.call("ground_hunt", &json!({ "on": true })).unwrap();
        assert!(g.call("ground_goal_drop", &json!({ "slot": "ad" })).is_err());
        g.call("ground_goal_tell", &json!({ "slot": "yours" })).unwrap();
        let w = g.wm().unwrap();
        let prog = afm_engine::world::ground::campaign::seat(w).unwrap();
        let verdict = afm_engine::world::ground::campaign::verdict_day(w, prog, year);
        while w.today <= verdict {
            w.advance_day();
        }
        let r = g.call("ground_report", &json!({})).unwrap();
        let text = r.to_string();
        println!("{}", serde_json::to_string_pretty(&r).unwrap());
        assert_eq!(r["ready"], json!(true), "{text}");
        assert!(!text.contains("NaN") && !text.contains("{}"), "{text}");
        assert!(!r["goals"].as_array().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
