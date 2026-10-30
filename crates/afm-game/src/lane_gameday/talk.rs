//! Game day talks (dialogue D4-D6, lane 14): the pregame room, the halftime
//! talk beside your coordinators' adjustments, a word on the sideline after
//! a costly mistake, and what each did on the game page. The talks are the
//! engine's (`game::talk`), the same for every team; these screens show
//! your staff's read of them in words and bars, never the numbers.

use afm_engine::calendar::{Date, Level as CalLevel};
use afm_engine::game::GameTeam;
use afm_engine::game::live::{Ask, Input, WordOffer};
use afm_engine::game::talk::{self, Ctx, Landing, Mind, Moment, Move, Pick, Record, Subject, Tone, Unit, Why};
use afm_engine::player::Trait;
use afm_engine::world::World;
use afm_engine::world::folk::{Team, gametalk};
use serde_json::{Value, json};

use super::film::Names;
use crate::Reply;
use crate::views;

/// The team as your seat names it, for the world's helpers.
fn seat_team(w: &World) -> Option<Team> {
    let (level, t) = w.seat_team()?;
    Some(match level {
        CalLevel::Nfl => Team::Nfl(t),
        CalLevel::HighSchool => Team::High(t),
        CalLevel::Other => Team::Other(t),
        _ => Team::College(t),
    })
}

fn pick_of(a: &Value, base: Pick) -> Pick {
    let s = |k: &str| a.get(k).and_then(Value::as_str);
    Pick {
        mv: s("move").and_then(Move::from_key).unwrap_or(base.mv),
        tone: s("tone").and_then(Tone::from_key).unwrap_or(base.tone),
        subject: s("subject").and_then(Subject::from_key).unwrap_or(base.subject),
        who: a.get("who").and_then(Value::as_u64).map_or(base.who, |x| x as u32),
    }
}

fn pick_json(p: &Pick) -> Value {
    json!({ "move": p.mv.key(), "move_name": p.mv.name(), "tone": p.tone.key(), "tone_name": p.tone.name(),
            "subject": p.subject.key(), "subject_name": subject_name(p.subject), "who": p.who })
}

fn subject_name(s: Subject) -> &'static str {
    match s {
        Subject::Stakes => "The stakes",
        Subject::Us => "Us",
        Subject::Them => "Them",
        Subject::Seniors => "The seniors",
        Subject::Plan => "The plan",
        Subject::One => "One player",
    }
}

fn seed_of(d: Date, p: &Pick, half: bool) -> u64 {
    (d.0 as u64) << 24
        ^ (p.mv as u64) << 16
        ^ (p.tone as u64) << 12
        ^ (p.subject as u64) << 8
        ^ p.who as u64
        ^ if half { 0x4A1F_0000_0000 } else { 0 }
}

fn first(name: &str) -> String {
    name.split_whitespace().next().unwrap_or(name).to_string()
}

/// Your staff's read of one value: off by a little, by your own read of a
/// room (motivation), the same each time you look at the same talk.
fn read(v: f32, motivation: f64, seed: u64, k: u64) -> f32 {
    let sd = 0.06 * (1.7 - 0.07 * motivation).max(0.25);
    (v as f64 + sd * talk::normal(seed, k)).clamp(0.02, 0.98) as f32
}

fn mind_json(m: &Mind) -> Value {
    json!({ "fire": m.fire, "poise": m.poise, "focus": m.focus })
}

/// What the staff calls a player: name, spot, year, and the few words
/// they'd use about how he takes a talk (only the strong ones).
fn tag_line(p: &afm_engine::game::GamePlayer, level: afm_engine::player::Level) -> String {
    let t = &p.personality.traits;
    let mut v: Vec<&str> = Vec::new();
    if t.get(Trait::Temper) >= 14.0 {
        v.push("short fuse");
    } else if t.get(Trait::Temper) <= 7.0 {
        v.push("even temper");
    }
    if t.get(Trait::Ego) >= 14.0 {
        v.push("big ego");
    }
    if t.get(Trait::Leadership) >= 14.0 {
        v.push("leads");
    }
    let c = p.composure(level);
    if c <= 7.5 {
        v.push("nervy");
    } else if c >= 14.0 {
        v.push("unflappable");
    }
    if p.experience < 1.0 {
        v.push("first season playing");
    } else if p.experience >= 6.0 {
        v.push("seen it all");
    }
    let s = v.join(", ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => "Nothing stands out".into(),
    }
}

/// One sentence on what it does to him on the field.
fn effect_line(l: &Landing) -> String {
    let mut v: Vec<&str> = Vec::new();
    if l.ear < 0.25 {
        v.push("It barely reaches him.");
    }
    if l.lift > 0.25 && l.focus >= 0.0 {
        v.push("Sharper off the ball.");
    } else if l.lift > 0.25 {
        v.push("Up for it, a touch loose.");
    } else if l.lift < -0.25 {
        v.push("Less of him early on.");
    }
    if l.focus < -0.4 {
        v.push("Likelier to jump the count or lose his man.");
    } else if l.focus > 0.4 {
        v.push("Locked in on his assignment.");
    }
    if l.poise < -0.5 {
        v.push("A mistake will bite harder.");
    } else if l.poise > 0.5 {
        v.push("A bad play won't rattle him.");
    }
    if l.heat > 0.06 {
        v.push("More likely to hit late.");
    }
    if v.is_empty() {
        v.push("No change, and that is fine for him.");
    }
    v.join(" ")
}

/// A unit's read in words, from the players who will play.
fn unit_words(team: &GameTeam, ls: &[Landing], w: &[f64], unit: Unit) -> (String, bool) {
    let mut count: Vec<(Why, f64)> = Vec::new();
    for (k, l) in ls.iter().enumerate() {
        if Unit::of(team.players[k].position) != unit || w[k] < 1.0 {
            continue;
        }
        match count.iter_mut().find(|c| c.0 == l.why) {
            Some(c) => c.1 += 1.0,
            None => count.push((l.why, 1.0)),
        }
    }
    count.sort_by(|a, b| b.1.total_cmp(&a.1));
    let Some(&(top, _)) = count.first() else { return ("No one dressed.".into(), false) };
    let worry = count.iter().filter(|c| c.0.worry() && c.0 != top).map(|c| c.1).sum::<f64>();
    let mut s = top.words().to_string();
    if !top.worry()
        && worry >= 2.0
        && let Some(c) = count.iter().find(|c| c.0.worry())
    {
        s = format!("{} {} of them: {}", s.trim_end_matches('.'), worry as u32, c.0.words().to_lowercase());
        return (s, true);
    }
    (s, top.worry())
}

/// The risk your staff names: who could go over the top or tighten up.
fn risk(team: &GameTeam, ls: &[Landing], w: &[f64], names: &Names) -> (String, &'static str) {
    let mut amped: Vec<&str> = Vec::new();
    let mut tight: Vec<&str> = Vec::new();
    let mut heat = 0.0;
    for (k, l) in ls.iter().enumerate() {
        if w[k] < 1.0 {
            continue;
        }
        let name = names.get(&team.players[k].id).map_or("", |x| x.0.as_str());
        match l.why {
            Why::OverAmped => amped.push(name),
            Why::Tight | Why::Shrank => tight.push(name),
            _ => {}
        }
        heat += l.heat.max(0.0) as f64;
    }
    let n = amped.len() + tight.len();
    let level = if n >= 4 || heat > 0.6 {
        "high"
    } else if n >= 2 || heat > 0.3 {
        "some"
    } else {
        "low"
    };
    let mut parts: Vec<String> = Vec::new();
    if !amped.is_empty() {
        parts.push(format!(
            "{} over the top: flags on the first series{}",
            list(&amped),
            if heat > 0.3 { " and a late hit or two" } else { "" }
        ));
    }
    if !tight.is_empty() {
        parts.push(format!("{} wound tight: a mistake will bite", list(&tight)));
    }
    if parts.is_empty() {
        parts.push("Little: nobody goes over the top and nobody tightens up.".into());
    }
    (parts.join(". ") + ".", level)
}

fn list(v: &[&str]) -> String {
    let n: Vec<String> = v.iter().take(3).map(|s| s.split_whitespace().last().unwrap_or(s).to_string()).collect();
    let more = v.len().saturating_sub(3);
    let s = match n.len() {
        0 => String::new(),
        1 => n[0].clone(),
        2 => format!("{} and {}", n[0], n[1]),
        _ => format!("{}, {} and {}", n[0], n[1], n[2]),
    };
    if more > 0 { format!("{s} and {more} more") } else { s }
}

/// The room for one pick: units, key players, the risk, the other tones,
/// your coordinators' reads.
#[allow(clippy::too_many_arguments)]
fn room_json(
    w: &World,
    team: &GameTeam,
    cx: &Ctx,
    pick: &Pick,
    names: &Names,
    date: Date,
    half: bool,
    lcx: &gametalk::LineCx,
) -> Value {
    let ls = talk::room(team, pick, cx);
    let wt = talk::weights(team);
    let mot = team.staff.motivation;
    let seed = seed_of(date, pick, half);
    let units: Vec<Value> = Unit::ALL
        .iter()
        .filter_map(|&u| {
            let (b, a) = talk::by_unit(team, &ls, u)?;
            let (words, worry) = unit_words(team, &ls, &wt, u);
            let k = u.index() as u64 * 8;
            let a = Mind {
                fire: read(a.fire, mot, seed, k),
                poise: read(a.poise, mot, seed, k + 1),
                focus: read(a.focus, mot, seed, k + 2),
            };
            let b = Mind {
                fire: read(b.fire, mot, seed ^ 0xB, k),
                poise: read(b.poise, mot, seed ^ 0xB, k + 1),
                focus: read(b.focus, mot, seed ^ 0xB, k + 2),
            };
            Some(json!({ "unit": u.name(), "words": words, "worry": worry, "before": mind_json(&b), "after": mind_json(&a) }))
        })
        .collect();
    // The men it moves most, with why.
    let mut moved: Vec<usize> = (0..ls.len()).filter(|&i| wt[i] >= 1.0).collect();
    let size = |i: usize| (ls[i].lift + 0.3 * ls[i].poise).abs() + if ls[i].why.worry() { 0.2 } else { 0.0 };
    moved.sort_by(|&a, &b| size(b).total_cmp(&size(a)));
    let captains = talk::captains(team, cx.day);
    let key: Vec<Value> = moved
        .iter()
        .take(5)
        .map(|&i| {
            let p = &team.players[i];
            let (name, pos) = names.get(&p.id).cloned().unwrap_or((format!("No. {}", p.id), p.position.name().into()));
            let class = views::person_index(w, p.id).map(|k| views::class_text(w, &w.people[k])).unwrap_or_default();
            json!({
                "id": p.id, "name": name, "pos": pos, "class": class,
                "captain": captains.contains(&i),
                "tag": tag_line(p, cx.level),
                "why": ls[i].why.words(), "worry": ls[i].why.worry(),
                "detail": effect_line(&ls[i]),
                "before": mind_json(&ls[i].before),
                "after": mind_json(&Mind {
                    fire: read(ls[i].after.fire, mot, seed ^ p.id as u64, 1),
                    poise: read(ls[i].after.poise, mot, seed ^ p.id as u64, 2),
                    focus: read(ls[i].after.focus, mot, seed ^ p.id as u64, 3),
                }),
            })
        })
        .collect();
    let (risk_text, risk_level) = risk(team, &ls, &wt, names);
    // The other tones for the same move and subject, roughly.
    let here = talk::worth(&ls, &wt) + 0.05 * talk::normal(seed, 99);
    let others: Vec<Value> = if matches!(pick.mv, Move::Nothing | Move::Captain | Move::Listen) {
        Vec::new()
    } else {
        Tone::ALL
            .iter()
            .filter(|&&t| t != pick.tone)
            .map(|&t| {
                let p2 = Pick { tone: t, ..*pick };
                let l2 = talk::room(team, &p2, cx);
                let v = talk::worth(&l2, &wt) + 0.05 * talk::normal(seed, 100 + t.index() as u64);
                let worries = l2.iter().zip(&wt).filter(|(l, k)| **k >= 1.0 && l.why.worry()).count();
                let mine = ls.iter().zip(&wt).filter(|(l, k)| **k >= 1.0 && l.why.worry()).count();
                let d = v - here;
                let text = if d > 0.08 {
                    "More of an edge than this."
                } else if d > 0.02 {
                    "A little more of an edge."
                } else if d > -0.02 {
                    "About the same."
                } else if d > -0.08 {
                    "A little less of an edge."
                } else {
                    "Much less of an edge."
                };
                let calm = if worries + 1 < mine {
                    " Fewer go over the top or tighten up."
                } else if worries > mine + 1 {
                    " More go over the top or tighten up."
                } else {
                    ""
                };
                json!({ "tone": t.name(), "key": t.key(), "text": format!("{text}{calm}") })
            })
            .collect()
    };
    // Your coordinators' reads, in their own voices.
    let me = seat_team(w);
    let coords = me.map(|t| gametalk::coordinators(w, t)).unwrap_or([None; 2]);
    let worst = moved.iter().find(|&&i| ls[i].why.worry()).map(|&i| {
        let n = names.get(&team.players[i].id).map_or(String::new(), |x| x.0.clone());
        (first(&n), ls[i].why)
    });
    let side_has = |off: bool| {
        moved.iter().take(8).any(|&i| {
            ls[i].why.worry()
                && matches!(Unit::of(team.players[i].position), Unit::Qb | Unit::Line | Unit::Skill) == off
        })
    };
    let worn = cx.day.worn[pick.tone.index()] >= 0.6 && pick.mv != Move::Captain;
    let staff: Vec<Value> = coords
        .iter()
        .enumerate()
        .filter_map(|(k, c)| {
            let c = (*c)?;
            let off = k == 0;
            let kind = if pick.mv == Move::Captain {
                "captain"
            } else if worn {
                "hollow"
            } else if side_has(off) {
                match worst {
                    Some((_, Why::OverAmped)) => "amped",
                    Some(_) => "tight",
                    None => "good",
                }
            } else if here.abs() < 0.04 {
                "flat"
            } else {
                "good"
            };
            let player = worst.as_ref().map(|x| x.0.as_str());
            let text = gametalk::staff_line(w, c, kind, player, seed).or_else(|| {
                gametalk::staff_line(w, c, if kind == "tight" || kind == "amped" { "good" } else { kind }, None, seed)
            })?;
            Some(json!({
                "coach": c, "name": w.coaches[c].name.clone(),
                "role": if off { "OC" } else { "DC" }, "text": text,
            }))
        })
        .collect();
    json!({
        "pick": pick_json(pick),
        "line": gametalk::talk_line(w, pick, lcx),
        "units": units,
        "key": key,
        "risk": { "text": risk_text, "level": risk_level },
        "others": others,
        "staff": staff,
        "worn": worn,
    })
}

/// Who the room listens to: each captain's voice and yours, how far each
/// carries, and what the captain you'd hand it to would say.
fn captains_json(w: &World, team: &GameTeam, cx: &Ctx, names: &Names, lcx: &gametalk::LineCx) -> Value {
    let wt = talk::weights(team);
    let n: f64 = wt.iter().sum::<f64>().max(1.0);
    let caps = talk::captains(team, cx.day);
    let mut v: Vec<Value> = caps
        .iter()
        .map(|&i| {
            let c = &team.players[i];
            let reach =
                team.players.iter().zip(&wt).map(|(p, k)| k * talk::reach(c, p)).sum::<f64>() / n;
            let (name, pos) = names.get(&c.id).cloned().unwrap_or((format!("No. {}", c.id), c.position.name().into()));
            let lead = c.personality.traits.get(Trait::Leadership);
            let words = if reach > 0.7 {
                "The whole room"
            } else if reach > 0.5 {
                match Unit::of(c.position) {
                    Unit::Qb | Unit::Line | Unit::Skill => "The offense",
                    Unit::Specialists => "The kickers",
                    _ => "The defense",
                }
            } else {
                "His own unit"
            };
            let says = views::person_index(w, c.id)
                .map(|k| gametalk::captain_lines(w, k, lcx))
                .unwrap_or_default();
            json!({
                "id": c.id, "name": name, "pos": pos, "reach": reach, "reach_words": words,
                "leadership": if lead >= 15.0 { "Very high" } else if lead >= 12.0 { "High" } else if lead >= 9.0 { "Fair" } else { "Low" },
                "years": c.experience.round(),
                "says": says,
            })
        })
        .collect();
    // Your own voice: how much of a talk reaches the room today.
    let pick = Pick::new(Move::Straight, Tone::Firm, Subject::Us);
    let ls = talk::room(team, &pick, cx);
    let ear = ls.iter().zip(&wt).map(|(l, k)| k * l.ear as f64).sum::<f64>() / n;
    v.push(json!({
        "you": true, "name": "You", "reach": ear.min(1.0),
        "reach_words": if ear > 0.75 { "The whole room" } else if ear > 0.55 { "Most of them" } else { "Mixed" },
    }));
    json!({ "voices": v, "handed": cx.day.handed })
}

/// The day's stakes in words: the headline and why.
fn stakes_json(team: &GameTeam, stakes: f64, playoff: bool, why: &[String]) -> Value {
    let head = if stakes >= 0.85 {
        "Everything is on this one."
    } else if stakes >= 0.6 {
        "A big one."
    } else if stakes >= 0.4 {
        "It matters more than most."
    } else {
        "A regular week."
    };
    let mut lines: Vec<String> = why.to_vec();
    if team.day.rival {
        lines.push("A rivalry game.".into());
    }
    if playoff {
        lines.push("Lose and the season is over.".into());
    }
    if team.day.seniors {
        lines.push("The seniors' last home game.".into());
    }
    json!({ "head": head, "level": stakes, "why": lines })
}

/// The week as your staff tells it: last week, practice, how your talks
/// have worn.
fn week_json(team: &GameTeam) -> Value {
    let pr = talk::practice(team);
    let practice = if pr > 0.5 {
        "Sharp. A good week of practice."
    } else if pr > 0.1 {
        "Prepared."
    } else if pr > -0.3 {
        "Loose in spots."
    } else {
        "Underprepared. Too many missed reps."
    };
    let worn: Vec<Value> = Tone::ALL
        .iter()
        .filter(|t| team.day.worn[t.index()] > 0.0)
        .map(|t| json!({ "tone": t.name(), "worn": team.day.worn[t.index()] }))
        .collect();
    json!({
        "last": match team.day.last { 1 => "A win last week.", -1 => "A loss last week.", _ => "No game last week." },
        "practice": practice,
        "worn": worn,
        "handed": team.day.handed,
    })
}

/// Who isn't dressing among the starters (the room plays without them).
fn hurt_json(w: &World, team: &GameTeam) -> Vec<Value> {
    let Some(t) = seat_team(w) else { return Vec::new() };
    let here: std::collections::HashSet<u32> = team.players.iter().map(|p| p.id).collect();
    t.roster(w)
        .into_iter()
        .filter(|&i| {
            let p = &w.people[i];
            p.last_role == Some(afm_engine::player::PlayingTime::Starter) && !here.contains(&p.player.id)
        })
        .take(5)
        .map(|i| {
            let p = &w.people[i];
            json!({ "id": p.player.id, "name": p.name.clone(), "pos": p.player.position.name(),
                    "room": team.day.hurt == Some(p.player.id) })
        })
        .collect()
}

/// The pregame clock: in the room, out for the tunnel, kickoff.
fn clock_json(kick: Option<u32>, level: CalLevel) -> Value {
    let Some(k) = kick else { return Value::Null };
    // Real pregame: back in after warm-ups about 20 minutes out (high
    // school about 22), out for the anthem and the coin toss 8-12 minutes
    // before (proposed, not confirmed).
    let (inn, out) = match level {
        CalLevel::Nfl => (20, 10),
        CalLevel::HighSchool => (22, 8),
        _ => (20, 12),
    };
    let t = |m: u32| {
        let h = (m / 60) % 24;
        format!("{}:{:02}", if h.is_multiple_of(12) { 12 } else { h % 12 }, m % 60)
    };
    json!({ "in": t(k.saturating_sub(inn)), "out": t(k.saturating_sub(out)), "kick": t(k), "minutes": inn - out })
}

/// The pregame room (D4), or the halftime room (D5) while your live game
/// waits at halftime.
pub(super) fn room(w: &World, a: &Value) -> Reply {
    let me = seat_team(w).ok_or("You have no team.")?;
    let (level, t) = w.seat_team().ok_or("You have no team.")?;
    if let Some(l) = w.live.as_ref().filter(|l| l.date == w.today) {
        let mut g = l.game.clone();
        let step = g.run();
        return match step.pending {
            Some(p) if p.ask == Ask::Halftime => halftime_room(w, l, &p, a),
            _ => Ok(
                json!({ "status": "played", "pregame": step.result.talks.pregame[l.game.ctl.team].as_ref().map(pick_json) }),
            ),
        };
    }
    let s = w.seat_setup_today().ok_or("The room opens on the day of the game.")?;
    let side = if s.teams[0] == t { 0 } else { 1 };
    let team = if side == 0 { &s.home } else { &s.away };
    let cx = talk::ctx(team, s.params.rules.level, s.params.playoff, &s.params.context[side], Moment::Pregame);
    let refs = [super::team_ref(level, s.teams[0]), super::team_ref(level, s.teams[1])];
    let names = super::names(w, &refs);
    let said = w.gameday.pregame.filter(|p| p.0 == w.today).map(|p| p.1);
    let staff_pick = talk::coach_pick(team, &cx, s.params.seed ^ 0x7A1C_0000 ^ ((side as u64) << 40));
    let base = said.unwrap_or(staff_pick);
    let mut pick = pick_of(a, base);
    if pick.mv == Move::Captain && pick.who == 0 {
        pick.who = talk::captains(team, cx.day).first().map_or(0, |&i| team.players[i].id);
    }
    if pick.subject == Subject::One && pick.who == 0 {
        pick.who = team.day.hurt.unwrap_or(0);
    }
    let opp = s.teams[1 - side];
    let nm = |id: u32| names.get(&id).map(|x| first(&x.0));
    let lcx = gametalk::LineCx {
        them: s.names[1 - side].clone(),
        halftime: None,
        last: team.day.last,
        player: nm(pick.who).filter(|_| pick.subject == Subject::One),
        captain: nm(pick.who).filter(|_| pick.mv == Move::Captain),
        rival: team.day.rival,
        seed: seed_of(w.today, &pick, false),
    };
    let next = super::next_game(w);
    let quote = team.day.quote.and_then(|(id, u)| {
        let unit = Unit::ALL.get(u as usize).copied()?;
        let text = gametalk::quote_line(w, id, unit, &s.names[side], w.today)?;
        let k = views::person_index(w, id)?;
        let p = &w.people[k];
        Some(json!({ "id": id, "name": p.name.clone(), "pos": p.player.position.name(),
                     "class": views::class_text(w, p), "unit": unit.name(), "text": text }))
    });
    let mut v = room_json(w, team, &cx, &pick, &names, w.today, false, &lcx);
    let ones: Vec<Value> = {
        let wt = talk::weights(team);
        let mut v: Vec<Value> = Vec::new();
        if let Some(h) = team.day.hurt {
            let n = views::person_index(w, h).map(|k| w.people[k].name.clone()).unwrap_or_default();
            v.push(json!({ "id": h, "name": n, "note": "out today" }));
        }
        for (k, p) in team.players.iter().enumerate() {
            if wt[k] >= 1.0 && v.len() < 26 {
                let (n, pos) = names.get(&p.id).cloned().unwrap_or_default();
                v.push(json!({ "id": p.id, "name": n, "note": pos }));
            }
        }
        v
    };
    v["status"] = json!(if said.is_some() { "said" } else { "choose" });
    v["moment"] = json!("pregame");
    v["staff_pick"] = pick_json(&staff_pick);
    v["me"] = views::team_brief(w, refs[side]);
    v["opp"] = views::team_brief(w, refs[1 - side]);
    v["opp_index"] = json!(opp);
    v["home"] = json!(side == 0);
    v["records"] = json!([views::record_text(w, refs[side]), views::record_text(w, refs[1 - side])]);
    v["date"] = json!(views::date_text(w.today));
    v["level"] = json!(level_name(level));
    v["stakes"] = stakes_json(team, cx.stakes, s.params.playoff, next.as_ref().map_or(&[][..], |n| &n.why[..]));
    v["name"] = json!(next.as_ref().and_then(|n| n.name.clone()));
    v["quote"] = quote.unwrap_or(Value::Null);
    v["week"] = week_json(team);
    v["hurt"] = json!(hurt_json(w, team));
    v["clock"] = clock_json(next.as_ref().and_then(|n| n.kick_min), level);
    v["captains"] = captains_json(w, team, &cx, &names, &lcx);
    v["ones"] = json!(ones);
    v["dressed"] = json!(team.players.len());
    v["options"] = options_json(team, &cx);
    let _ = me;
    Ok(v)
}

fn level_name(l: CalLevel) -> &'static str {
    match l {
        CalLevel::Nfl => "NFL",
        CalLevel::HighSchool => "High school",
        CalLevel::Other => "Pro",
        _ => "College",
    }
}

/// The moves, tones and subjects you can pick today.
fn options_json(team: &GameTeam, cx: &Ctx) -> Value {
    let moves: Vec<Value> = Move::ALL.iter().map(|m| json!({ "key": m.key(), "name": m.name() })).collect();
    let tones: Vec<Value> =
        Tone::ALL.iter().map(|t| json!({ "key": t.key(), "name": t.name(), "worn": cx.day.worn[t.index()] })).collect();
    let subjects: Vec<Value> = Subject::ALL
        .iter()
        .filter(|s| {
            **s != Subject::Seniors
                || cx.day.seniors
                || !team.players.is_empty() && cx.level != afm_engine::player::Level::NFL
        })
        .map(|s| json!({ "key": s.key(), "name": subject_name(*s) }))
        .collect();
    json!({ "moves": moves, "tones": tones, "subjects": subjects })
}

/// The halftime room (D5): the same reads at the break, with the score.
fn halftime_room(
    w: &World,
    l: &afm_engine::world::live_seat::LiveSeat,
    p: &afm_engine::game::live::Pending,
    a: &Value,
) -> Reply {
    let mine = l.game.ctl.team;
    let team = if mine == 0 { &l.game.home } else { &l.game.away };
    let margin = p.view.score[mine] - p.view.score[1 - mine];
    let cx = talk::ctx(
        team,
        l.game.params.rules.level,
        l.game.params.playoff,
        &l.game.params.context[mine],
        Moment::Halftime { margin },
    );
    let refs = [super::team_ref(l.level, l.teams[0]), super::team_ref(l.level, l.teams[1])];
    let names = super::names(w, &refs);
    let staff_pick = talk::coach_pick(team, &cx, l.game.params.seed ^ 0x7A1C_0000 ^ ((mine as u64) << 40) ^ (1 << 44));
    let mut pick = pick_of(a, p.talked.unwrap_or(staff_pick));
    if pick.mv == Move::Captain && pick.who == 0 {
        pick.who = talk::captains(team, cx.day).first().map_or(0, |&i| team.players[i].id);
    }
    let nm = |id: u32| names.get(&id).map(|x| first(&x.0));
    let lcx = gametalk::LineCx {
        them: l.names[1 - mine].clone(),
        halftime: Some(margin),
        last: team.day.last,
        player: nm(pick.who).filter(|_| pick.subject == Subject::One),
        captain: nm(pick.who).filter(|_| pick.mv == Move::Captain),
        rival: team.day.rival,
        seed: seed_of(l.date, &pick, true),
    };
    let mut v = room_json(w, team, &cx, &pick, &names, l.date, true, &lcx);
    v["status"] = json!(if p.talked.is_some() { "said" } else { "choose" });
    v["moment"] = json!("halftime");
    v["margin"] = json!(margin);
    v["score"] = json!([p.view.score[mine], p.view.score[1 - mine]]);
    v["staff_pick"] = pick_json(&staff_pick);
    v["me"] = views::team_brief(w, refs[mine]);
    v["opp"] = views::team_brief(w, refs[1 - mine]);
    v["home"] = json!(mine == 0);
    v["level"] = json!(level_name(l.level));
    v["break"] = json!(match l.level {
        CalLevel::Nfl => 13,
        CalLevel::Other => 15,
        _ => 20,
    });
    v["captains"] = captains_json(w, team, &cx, &names, &lcx);
    v["options"] = options_json(team, &cx);
    v["week"] = week_json(team);
    Ok(v)
}

/// Your pregame talk said (D4): kept for kickoff.
pub(super) fn say(w: &mut World, a: &Value) -> Reply {
    if w.live.as_ref().is_some_and(|l| l.date == w.today) {
        return Err("The team is already on the field.".into());
    }
    let s = w.seat_setup_today().ok_or("You have no game today.")?;
    let (_, t) = w.seat_team().ok_or("You have no team.")?;
    let side = if s.teams[0] == t { 0 } else { 1 };
    let team = if side == 0 { &s.home } else { &s.away };
    let nothing = a.get("nothing").and_then(Value::as_bool).unwrap_or(false);
    let mut pick = if nothing { Pick { mv: Move::Nothing, ..Pick::default() } } else { pick_of(a, Pick::default()) };
    if pick.mv == Move::Captain && pick.who == 0 {
        let cx = talk::ctx(team, s.params.rules.level, s.params.playoff, &s.params.context[side], Moment::Pregame);
        pick.who = talk::captains(team, cx.day).first().map_or(0, |&i| team.players[i].id);
    }
    if a.get("undo").and_then(Value::as_bool).unwrap_or(false) {
        w.gameday.pregame = None;
    } else {
        w.gameday.pregame = Some((w.today, pick));
    }
    room(w, &json!({}))
}

/// Your halftime talk (D5): an answer to the halftime stop.
pub(super) fn halftime_input(a: &Value) -> Input {
    let nothing = a.get("nothing").and_then(Value::as_bool).unwrap_or(false);
    Input::Talk(if nothing { Pick { mv: Move::Nothing, ..Pick::default() } } else { pick_of(a, Pick::default()) })
}

/// A word on the sideline (D6): the answer that gives it.
pub(super) fn word_input(a: &Value) -> Result<Input, String> {
    let id = a.get("id").and_then(Value::as_u64).ok_or("Missing player.")? as u32;
    let pick = pick_of(a, Pick { mv: Move::Straight, tone: Tone::Calm, subject: Subject::One, who: id });
    Ok(Input::Word { id, pick: Pick { subject: Subject::One, who: id, ..pick } })
}

/// The players waiting for a word, for the live screen: the mistake, the
/// word your staff would give and how each word you could give would land.
pub(super) fn words_json(w: &World, offers: &[WordOffer], names: &Names) -> Vec<Value> {
    let Some(l) = w.live.as_ref() else { return Vec::new() };
    let mine = l.game.ctl.team;
    let team = if mine == 0 { &l.game.home } else { &l.game.away };
    offers
        .iter()
        .filter_map(|o| {
            let p = team.players.iter().find(|p| p.id == o.id)?;
            let cx = talk::ctx(
                team,
                l.game.params.rules.level,
                l.game.params.playoff,
                &l.game.params.context[mine],
                Moment::Sideline { mistake: o.mistake },
            );
            let (name, pos) = names.get(&o.id).cloned().unwrap_or((format!("No. {}", o.id), p.position.name().into()));
            let f = first(&name);
            let seed = (l.date.0 as u64) << 20 ^ o.play as u64 ^ (o.id as u64) << 32;
            let opts: Vec<Value> = talk::word_options(o.id)
                .iter()
                .enumerate()
                .map(|(k, pk)| {
                    let land = talk::land(p, pk, &cx, None);
                    let good = land.lift + 0.3 * land.poise;
                    let noisy = good as f64
                        + 0.25 * (1.7 - 0.07 * team.staff.motivation).max(0.25) * talk::normal(seed, k as u64);
                    let read = if noisy > 0.5 {
                        "Should settle him."
                    } else if noisy > 0.1 {
                        "Should help a little."
                    } else if noisy > -0.2 {
                        "Hard to say."
                    } else {
                        "Could rattle him more."
                    };
                    json!({
                        "pick": pick_json(pk),
                        "label": format!("{} · {}", pk.mv.name(), pk.tone.name()),
                        "line": gametalk::word_line(w, pk, o.mistake, &f, seed ^ k as u64),
                        "read": read,
                        "staff": *pk == Pick { subject: Subject::One, who: o.id, ..o.staff },
                    })
                })
                .collect();
            Some(json!({
                "id": o.id, "name": name, "pos": pos, "tag": tag_line(p, cx.level),
                "mistake": o.mistake.words(), "play": o.play, "options": opts,
            }))
        })
        .collect()
}

/// A game's talks for its page: each talk, what it did to your room by
/// unit, who stood out, the first quarter, and every sideline word.
pub(crate) fn record_json(w: &World, rec: &Record, mine: Option<usize>, names: &Names) -> Value {
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    let rooms: Vec<Value> = rec
        .rooms
        .iter()
        .filter(|r| mine.is_none_or(|m| r.side as usize == m))
        .map(|r| {
            let pick = if r.halftime { rec.halftime[r.side as usize] } else { rec.pregame[r.side as usize] };
            let units: Vec<Value> = r
                .units
                .iter()
                .filter_map(|(u, b, a)| {
                    let u = Unit::ALL.get(*u as usize)?;
                    let dp = a.poise - b.poise;
                    let df = a.focus - b.focus;
                    let words = if dp < -0.06 {
                        "Tighter."
                    } else if a.fire > 0.8 {
                        "Amped, a bit loose."
                    } else if df > 0.05 && dp >= 0.0 {
                        "Locked in."
                    } else if a.fire - b.fire > 0.06 {
                        "Fired up."
                    } else if dp > 0.05 {
                        "Settled."
                    } else {
                        "Unmoved."
                    };
                    Some(json!({ "unit": u.name(), "words": words, "worry": dp < -0.06 || a.fire > 0.8,
                                 "before": mind_json(b), "after": mind_json(a) }))
                })
                .collect();
            let stood: Vec<Value> = r
                .stood
                .iter()
                .map(|(id, why, lift)| {
                    json!({ "id": id, "name": name(*id), "pos": names.get(id).map_or(String::new(), |x| x.1.clone()),
                            "why": why.words(), "worry": why.worry(), "up": *lift > 0.0 })
                })
                .collect();
            let first_q = if r.quarter > 0.4 {
                "A real edge through the first quarter."
            } else if r.quarter > 0.15 {
                "A small edge early, gone by the second quarter."
            } else if r.quarter > -0.15 {
                "About even: the room played as it came in."
            } else {
                "It cost a little early on."
            };
            json!({
                "halftime": r.halftime, "side": r.side,
                "pick": pick.as_ref().map(pick_json),
                "units": units, "stood": stood,
                "first": first_q, "worries": r.worries,
                "poise": if r.poise < -0.4 { "Composure down: mistakes bit harder." } else if r.poise > 0.4 { "Composure up: mistakes bit less." } else { "Composure about the same." },
            })
        })
        .collect();
    let words: Vec<Value> = rec
        .words
        .iter()
        .filter(|n| mine.is_none_or(|m| n.side as usize == m))
        .map(|n| {
            json!({
                "id": n.id, "name": name(n.id), "mistake": n.mistake.words(), "play": n.play,
                "pick": pick_json(&n.pick), "yours": n.yours, "why": n.why.words(), "worry": n.why.worry(),
                "trust": if !n.yours { "" } else if n.trust > 0.005 { "Trusts you a little more." } else if n.trust < -0.005 { "Trusts you a little less." } else { "" },
            })
        })
        .collect();
    let _ = w;
    json!({ "rooms": rooms, "words": words })
}
