//! The hunt (next update, lane 11, SK67): one target chased over seasons,
//! at every seat. The page shows your hunt (his games watched, what you
//! banked, who knows, his say, your boss), your past hunts and their
//! look-backs, who to hunt when you have none, the world's chases and the
//! generational prospects, and who is hunting your own players. Every
//! rating is your staff's read; potential stays in words.

use afm_engine::calendar::Level;
use afm_engine::world::hunt::{self, End, Hunt, Seat, ai, generational};
use afm_engine::world::scouting::{self as sc, readers};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::reads;
use crate::views::{TeamRef, date_text, person_index, team_brief};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let id = || args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
    Some(match cmd {
        "hunt_page" => game.w().and_then(page),
        "hunt_name" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            Ok(json!({ "text": hunt::name(w, pid)? }))
        })(),
        "hunt_drop" => game.wm().and_then(|w| Ok(json!({ "text": hunt::drop(w)? }))),
        "hunt_loud" => game.wm().and_then(|w| Ok(json!({ "text": hunt::loud(w)? }))),
        "hunt_bank" => (|| {
            let nil = args.get("nil").and_then(Value::as_f64);
            let hold = args.get("hold").and_then(Value::as_bool);
            let w = game.wm()?;
            Ok(json!({ "text": hunt::bank(w, nil, hold)? }))
        })(),
        _ => return None,
    })
}

fn seat_key(s: Seat) -> &'static str {
    s.key()
}

fn team_of_seat(s: Seat, t: usize) -> TeamRef {
    match s {
        Seat::Club => TeamRef::Nfl(t),
        Seat::College => TeamRef::College(t),
        Seat::School => TeamRef::HighSchool(t),
    }
}

fn ai_team(level: u8, t: u32) -> TeamRef {
    match level {
        ai::CLUB => TeamRef::Nfl(t as usize),
        ai::COLLEGE => TeamRef::College(t as usize),
        _ => TeamRef::HighSchool(t as usize),
    }
}

fn why_key(why: u8) -> &'static str {
    match why {
        ai::PILED => "piled",
        ai::GEN => "generational",
        _ => "own",
    }
}

/// A player's class in words.
fn class_words(w: &World, k: usize) -> String {
    let p = &w.people[k];
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => match p.class_of - w.dates.season {
            ..=1 => format!("Senior, class of {}", p.class_of),
            2 => format!("Junior, class of {}", p.class_of),
            3 => format!("Sophomore, class of {}", p.class_of),
            _ => format!("Freshman, class of {}", p.class_of),
        },
        Stage::College => {
            let y = ["Freshman", "Sophomore", "Junior", "Senior", "Fifth-year"][(p.college_seasons as usize).min(4)];
            format!("{y} in college")
        }
        Stage::Declared => "Declared for the draft".into(),
        Stage::Pro => "In the NFL".into(),
        _ => String::new(),
    }
}

/// Where he plays now.
fn where_of(w: &World, k: usize) -> Value {
    crate::views::team_of(w, &w.people[k]).map_or(Value::Null, |t| team_brief(w, t))
}

/// A player row: who, where, your staff's read and yours, potential words.
fn who(w: &World, k: usize) -> Value {
    let p = &w.people[k];
    json!({
        "id": p.player.id,
        "name": p.name,
        "pos": p.player.position.name(),
        "class": class_words(w, k),
        "age": p.player.age,
        "stars": p.stars,
        "team": where_of(w, k),
        "staff": reads::overall(w, k).json(),
        "mine": reads::mine(w, k).json(),
        "potential": reads::potential_words(w, k),
        "generational": gen_json(w, p.player.id),
    })
}

fn gen_json(w: &World, id: u32) -> Value {
    generational::of(w, id).map_or(Value::Null, |g| {
        json!({
            "year": g.year,
            "draft": g.draft,
            "date": date_text(g.date),
            "since": g.since.as_ref().map(|(y, n)| json!({ "year": y, "name": n })),
        })
    })
}

/// The deadline in words.
fn deadline(w: &World, h: &Hunt, k: usize) -> String {
    let p = &w.people[k];
    match h.seat {
        Seat::College => match afm_engine::calendar::anchors::early_signing(p.class_of - 1) {
            Some(d) => format!("He can sign on {} or in February {}.", d.pretty(), p.class_of),
            None => format!("He signs in February {}.", p.class_of),
        },
        Seat::Club => {
            let dy = sc::public::draft_year(w);
            if p.stage == Stage::Declared {
                format!("The {dy} draft.")
            } else if dy - p.class_of >= 3 {
                format!("Eligible for the {dy} draft, if he declares.")
            } else {
                format!("Eligible from the {} draft.", p.class_of + 3)
            }
        }
        Seat::School => {
            let next = if w.today.parts().1 >= 7 { w.today.year() + 1 } else { w.today.year() };
            format!("Families move in the June transfer season ({next}).")
        }
    }
}

fn heat_words(x: f64) -> &'static str {
    match x {
        x if x < 0.15 => "Nobody knows",
        x if x < 0.35 => "Whispers",
        x if x < 0.6 => "People are talking",
        x if x < 0.85 => "Known around the game",
        _ => "Everyone knows",
    }
}

/// His say, in your staff's words.
fn say(w: &World, h: &Hunt, k: usize) -> Value {
    let p = &w.people[k];
    match h.seat {
        Seat::College => {
            let offered =
                w.front_office.college.board.iter().any(|t| t.id == h.id && t.offered.is_some() && t.pulled.is_none());
            let last = hunt::signing_class(w) == p.class_of;
            let state = if !offered {
                "Your staff is building a bond: he's not counting you until you offer."
            } else if !last {
                "He's listening: he won't cut his list until his senior year."
            } else if h.cold == 0 {
                "You're near the top of his list."
            } else if h.cold < 4 {
                "Your staff hears you're slipping on his list."
            } else {
                "He's cutting his list, and you may be off it soon."
            };
            let committed = p.cal.commit.map(|c| team_brief(w, TeamRef::College(c.0)));
            json!({
                "words": state,
                "offers": p.cal.offers.len() + p.cal.early_offers.iter().filter(|o| !p.cal.offers.contains(&o.0)).count(),
                "committed": committed,
                "visits": p.cal.visits.iter().filter(|v| v.0 == h.team).count(),
            })
        }
        Seat::Club => {
            let refuses = sc::predraft::refuses(w, h.id, h.team);
            json!({
                "words": if refuses {
                    "His camp says he won't play for you. Take him anyway and his rights go to the club that wants him most, for its next pick and a first."
                } else if p.stage == Stage::Declared {
                    "He has declared. His agent says he'd be happy anywhere, as agents do."
                } else {
                    "He's in school: his say comes when he declares."
                },
                "refuses": refuses,
                "declared": p.stage == Stage::Declared,
            })
        }
        Seat::School => {
            let pitched = w.scouting.pitches.list.iter().rev().find(|x| x.id == h.id && x.date >= h.since);
            let warm = match h.warmth {
                x if x < 0.15 => "His family hardly knows you.",
                x if x < 0.45 => "His family is getting to know you.",
                x if x < 0.75 => "His family likes you.",
                _ => "His family trusts you.",
            };
            json!({
                "words": warm,
                "warmth": (h.warmth * 100.0).round(),
                "pitched": pitched.map(|x| date_text(x.date)),
                "moved": pitched.is_some_and(|x| x.moved),
            })
        }
    }
}

/// What you banked.
fn bank(w: &World, h: &Hunt) -> Value {
    match h.seat {
        Seat::College => {
            let b = afm_engine::world::front_office::college::budget(w, h.team);
            let promise = w.front_office.college.board.iter().find(|t| t.id == h.id).and_then(|t| t.nil);
            json!({
                "nil": h.nil,
                "hold": h.hold,
                "left": b.left.max(0.0),
                "total": b.total,
                "promise": promise,
                "nil_era": afm_engine::world::ncaa::nil_era(w.today.year()),
            })
        }
        Seat::Club => {
            let c = hunt::capital(w, h.team, h.id);
            json!({
                "draft": c.draft,
                "rank": c.rank,
                "first": c.first,
                "gap": c.gap.map(|x| x.round()),
                "this_year": c.this_year.round(),
                "next_year": c.next_year.round(),
            })
        }
        Seat::School => {
            let season = w.dates.season;
            let used = w
                .scouting
                .pitches
                .list
                .iter()
                .filter(|x| x.date >= afm_engine::calendar::Date::ymd(season, 1, 1).add(-150))
                .count();
            json!({ "pitches_left": sc::pitch::PER_SEASON.saturating_sub(used), "chance": sc::pitch::chance_now(w, h.id).map(|c| (c * 100.0).round()) })
        }
    }
}

fn boss_json(w: &World) -> Value {
    let Some(t) = afm_engine::world::folk::Team::seat(w) else { return Value::Null };
    let b = afm_engine::world::folk::boss::boss(w, t);
    let trust = afm_engine::world::folk::boss::trust(w, t);
    json!({ "name": b.name, "title": b.title, "words": afm_engine::world::folk::boss::trust_words(trust) })
}

fn yours_json(w: &World, h: &Hunt) -> Value {
    let Some(k) = person_index(w, h.id) else { return Value::Null };
    let (heat, signals) = hunt::heat(w, h);
    let rivals: Vec<Value> = ai::on(w, h.id)
        .into_iter()
        .filter(|x| ai::known(w, x) && !(x.team as usize == h.team && x.level == level_of(h.seat)))
        .map(|x| json!({ "team": team_brief(w, ai_team(x.level, x.team)), "why": why_key(x.why), "since": date_text(x.since) }))
        .collect();
    let seen = h.watched.iter().filter(|x| x.seen).count();
    let mut log: Vec<Value> =
        h.log.iter().rev().take(20).map(|(d, s)| json!({ "date": date_text(*d), "text": s })).collect();
    if log.is_empty() {
        log.push(json!({ "date": date_text(h.since), "text": "You named him your hunt." }));
    }
    json!({
        "player": who(w, k),
        "since": date_text(h.since),
        "deadline": deadline(w, h, k),
        "loud": h.loud.map(date_text),
        "heat": (heat * 100.0).round(),
        "heat_words": heat_words(heat),
        "signals": signals.iter().map(|s| json!({ "date": date_text(s.date), "what": s.what, "weight": (s.weight * 100.0).round() })).collect::<Vec<_>>(),
        "watched": h.watched.iter().rev().take(16).map(|x| json!({ "date": date_text(x.date), "against": x.against, "seen": x.seen })).collect::<Vec<_>>(),
        "games_seen": seen,
        "games_missed": h.watched.len() - seen,
        "film_left": (sc::own::film_left(w) * 10.0).round() / 10.0,
        "film_week": (sc::own::film_budget(w) * 10.0).round() / 10.0,
        "rivals": rivals,
        "say": say(w, h, k),
        "bank": bank(w, h),
        "boss": boss_json(w),
        "log": log,
    })
}

fn level_of(s: Seat) -> u8 {
    match s {
        Seat::Club => ai::CLUB,
        Seat::College => ai::COLLEGE,
        Seat::School => ai::SCHOOL,
    }
}

fn past_json(w: &World) -> Vec<Value> {
    w.hunt
        .past
        .iter()
        .rev()
        .map(|h| {
            let (d, end, why) = h.end.clone().unwrap_or((h.since, End::Gone, String::new()));
            json!({
                "id": h.id,
                "name": h.name,
                "pos": h.pos.name(),
                "since": date_text(h.since),
                "date": date_text(d),
                "end": end.key(),
                "why": why,
                "loud": h.loud.is_some(),
                "games": h.watched.iter().filter(|x| x.seen).count(),
                "look_back": h.look_back.as_ref().map(|(d, hit, words)| json!({ "date": date_text(*d), "hit": hit, "words": words })),
            })
        })
        .collect()
}

/// Who you could hunt: your board or the public's best, read by your
/// staff, best first.
fn suggest(w: &World, s: Seat, team: usize) -> Vec<Value> {
    let season = w.dates.season;
    let mut ids: Vec<(u32, &'static str)> = Vec::new();
    match s {
        Seat::College => {
            for t in &w.front_office.college.board {
                ids.push((t.id, "On your board"));
            }
            let next = hunt::signing_class(w);
            let mut top: Vec<(f64, u32)> = w
                .people
                .iter()
                .filter(|p| {
                    matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
                        && (next..=next + 2).contains(&p.class_of)
                        && p.stars >= 4
                        && p.cal.signed.is_none()
                })
                .map(|p| (p.cal.service_read.unwrap_or(0.0), p.player.id))
                .collect();
            top.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            ids.extend(top.into_iter().take(40).map(|x| (x.1, "The services' best")));
        }
        Seat::Club => {
            if let Some(v) = sc::public::consensus(w) {
                ids.extend(v.into_iter().take(40).map(|id| (id, "The public's top 50")));
            }
            let dy = sc::public::draft_year(w);
            let mut top: Vec<(f64, u32)> = w
                .people
                .iter()
                .filter(|p| p.stage == Stage::College && p.pro.is_none() && dy - p.class_of >= 2)
                .filter_map(|p| p.cal.draft_read.map(|r| (r.score, p.player.id)))
                .collect();
            top.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            ids.extend(top.into_iter().take(40).map(|x| (x.1, "The scouts' early word")));
        }
        Seat::School => {
            let state = w.high_schools[team].state;
            let mut top: Vec<(u8, u8, u32)> = w
                .people
                .iter()
                .filter(|p| {
                    p.stage == Stage::HighSchool
                        && p.class_of > season
                        && p.hs != team
                        && w.high_schools.get(p.hs).is_some_and(|x| x.state == state)
                        && (p.stars >= 3 || p.cal.all_state > 0)
                })
                .map(|p| (p.stars, p.cal.all_state, p.player.id))
                .collect();
            top.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)).then(a.2.cmp(&b.2)));
            ids.extend(top.into_iter().take(40).map(|x| (x.2, "Rated in your state")));
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut rows: Vec<(f64, Value)> = Vec::new();
    for (id, why) in ids {
        if !seen.insert(id) || hunt::can_hunt(w, id).is_err() {
            continue;
        }
        let Some(k) = person_index(w, id) else { continue };
        let c = reads::centre(w, k).unwrap_or(0.0);
        let mut v = who(w, k);
        v["why"] = json!(why);
        rows.push((c, v));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    rows.into_iter().take(14).map(|x| x.1).collect()
}

/// The chases the public knows about: the generational prospects and the
/// most-hunted players at your level, with the teams known to be on them.
fn world_json(w: &World, s: Seat) -> Value {
    let gens: Vec<Value> = w
        .hunt
        .generational
        .iter()
        .rev()
        .take(6)
        .map(|g| {
            json!({
                "id": g.id,
                "name": g.name,
                "pos": g.pos.name(),
                "year": g.year,
                "draft": g.draft,
                "date": date_text(g.date),
                "since": g.since.as_ref().map(|(y, n)| json!({ "year": y, "name": n })),
                "hunters": ai::on(w, g.id).len(),
            })
        })
        .collect();
    let level = level_of(s);
    let mut by: std::collections::HashMap<u32, Vec<&ai::AiHunt>> = std::collections::HashMap::new();
    for h in w.hunt.ai.iter().filter(|h| h.level == level && h.open() && ai::known(w, h)) {
        by.entry(h.id).or_default().push(h);
    }
    let mut rows: Vec<(usize, u32, Vec<&ai::AiHunt>)> = by.into_iter().map(|(id, v)| (v.len(), id, v)).collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let chases: Vec<Value> = rows
        .into_iter()
        .take(10)
        .filter_map(|(_, id, v)| {
            let k = person_index(w, id)?;
            let mut x = who(w, k);
            x["hunters"] = json!(v.iter().map(|h| team_brief(w, ai_team(h.level, h.team))).collect::<Vec<_>>());
            Some(x)
        })
        .collect();
    json!({ "generational": gens, "chases": chases })
}

/// Your own players others hunt: colleges and private powers after your
/// high schoolers, NFL clubs after your college players.
fn hunted(w: &World, s: Seat, team: usize) -> Vec<Value> {
    let mine = |k: usize| -> bool {
        let p = &w.people[k];
        match s {
            Seat::School => p.stage == Stage::HighSchool && p.hs == team,
            Seat::College => p.stage == Stage::College && p.college == Some(team),
            Seat::Club => false,
        }
    };
    let mut by: std::collections::HashMap<u32, Vec<&ai::AiHunt>> = std::collections::HashMap::new();
    for h in w.hunt.ai.iter().filter(|h| h.open() && ai::known(w, h)) {
        if let Some(k) = person_index(w, h.id)
            && mine(k)
        {
            by.entry(h.id).or_default().push(h);
        }
    }
    let mut rows: Vec<(usize, u32, Vec<&ai::AiHunt>)> = by.into_iter().map(|(id, v)| (v.len(), id, v)).collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    rows.into_iter()
        .take(12)
        .filter_map(|(_, id, v)| {
            let k = person_index(w, id)?;
            let mut x = who(w, k);
            x["hunters"] = json!(v.iter().map(|h| team_brief(w, ai_team(h.level, h.team))).collect::<Vec<_>>());
            Some(x)
        })
        .collect()
}

pub fn page(w: &World) -> Reply {
    let (s, team) =
        hunt::seat(w).ok_or("The hunt is for a coach with a job at a high school, a college or an NFL club.")?;
    let yours = hunt::yours(w).map(|h| yours_json(w, h)).unwrap_or(Value::Null);
    Ok(json!({
        "seat": seat_key(s),
        "team": team_brief(w, team_of_seat(s, team)),
        "today": date_text(w.today),
        "yours": yours,
        "past": past_json(w),
        "suggest": if hunt::yours(w).is_none() { json!(suggest(w, s, team)) } else { json!([]) },
        "world": world_json(w, s),
        "hunted": hunted(w, s, team),
        "level": match s { Seat::Club => "nfl", Seat::College => "college", Seat::School => "hs" },
    }))
}

/// The player page's part: whether you can make him your hunt, and if he
/// is yours.
pub fn player_json(w: &World, k: usize) -> Value {
    let Some(p) = w.people.get(k) else { return Value::Null };
    if w.seat.is_none_or(|s| !matches!(s.level, Level::Nfl | Level::College | Level::HighSchool))
        || readers::seat_staff(w).is_none()
    {
        return Value::Null;
    }
    let id = p.player.id;
    let on = hunt::is_yours(w, id);
    let can = !on && hunt::can_hunt(w, id).is_ok();
    let busy = hunt::yours(w).filter(|h| h.id != id).map(|h| h.name.clone());
    json!({ "on": on, "can": can, "busy": busy, "generational": gen_json(w, id) })
}
