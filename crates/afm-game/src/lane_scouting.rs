//! Lane commands: scouting and the draft (section 7's own side, final build
//! lane 7). Every screen here shows your staff's reads (ranges, words and
//! where each one came from), never a true number: the scouting file, your
//! department, the public's boards, the draft board, the war room, the
//! undrafted rush, the pre-draft season, pro scouting, recruit and portal
//! scouting at a college, and the high school coach's side.

use std::collections::{HashMap, HashSet};

use afm_engine::attributes::{Attr, Category};
use afm_engine::calendar::{Date, Level as CalLevel};
use afm_engine::positions::Position;
use afm_engine::world::coach::Group;
use afm_engine::world::scouting::{
    self as sc, Kind, board, dept, file, hs, lines, live, lookback, predraft, public, readers, social,
};
use afm_engine::world::{Person, Stage, World};
use serde_json::{Value, json};

use crate::views::{TeamRef, date_text, team_brief, team_name};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let num = |k: &str| args.get(k).and_then(Value::as_u64);
    let id = || num("id").map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
    let text = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let r = match cmd {
        "scout_file" => (|| scout_file(game.w()?, id()?))(),
        "scouting" => (|| Ok(department(game.w()?)))(),
        "scout_assign" => (|| {
            let w = game.wm()?;
            assign(w, args)?;
            Ok(department(w))
        })(),
        "scout_instruct" => (|| {
            let w = game.wm()?;
            instruct(w, args)?;
            Ok(department(w))
        })(),
        "scout_ask" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            let msg = ask(w, pid, &text("kind"))?;
            Ok(json!({ "said": msg, "file": scout_file(w, pid)? }))
        })(),
        "follow" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            follow(w, pid, &text("why"))?;
            scout_file(w, pid)
        })(),
        "hs_pitch" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            Ok(json!({ "text": afm_engine::world::scouting::pitch::pitch(w, pid)? }))
        })(),
        "stake_name" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            let public = args.get("public").and_then(Value::as_bool).unwrap_or(false);
            Ok(json!({ "text": afm_engine::world::scouting::stake::stake(w, pid, public)? }))
        })(),
        "unfollow" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            w.scouting.dept.my_players.retain(|f| f.id != pid);
            crate::lane_core::desk::list_remove(w, pid);
            scout_file(w, pid)
        })(),
        "public_boards" => (|| Ok(public_boards(game.w()?)))(),
        "draft_board" => (|| draft_board(game.w()?))(),
        "board_move" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            board_move(w, pid, args)?;
            draft_board(w)
        })(),
        "board_dnd" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            board_dnd(w, pid, args.get("on").and_then(Value::as_bool).unwrap_or(true), &text("why"))?;
            draft_board(w)
        })(),
        "board_pfa" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            board_pfa(w, pid, args)?;
            draft_board(w)
        })(),
        "board_list" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            board_list(
                w,
                pid,
                num("list").unwrap_or(0) as u8,
                args.get("on").and_then(Value::as_bool).unwrap_or(true),
            )?;
            draft_board(w)
        })(),
        "draft_live" => (|| war_room(game.w()?))(),
        "draft_go" => (|| {
            let w = game.wm()?;
            let until = match text("until").as_str() {
                "next" => live::Until::Next,
                "round" => live::Until::Round,
                _ => live::Until::Yours,
            };
            if w.scouting.live.is_none() {
                return Err("The draft isn't live.".into());
            }
            live::advance(w, until);
            war_room(w)
        })(),
        "draft_pick" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            live::pick(w, pid)?;
            war_room(w)
        })(),
        "draft_auto" => (|| {
            let w = game.wm()?;
            live::auto_pick(w)?;
            war_room(w)
        })(),
        "draft_timeout" => (|| {
            let w = game.wm()?;
            live::timeout(w)?;
            war_room(w)
        })(),
        "draft_accept" => (|| {
            let w = game.wm()?;
            live::accept(w, num("offer").unwrap_or(0) as usize)?;
            war_room(w)
        })(),
        "draft_quote" => (|| {
            let w = game.w()?;
            let l = w.scouting.live.as_ref().ok_or("The draft isn't live.")?;
            let slot = num("slot").ok_or("Missing slot.")? as usize;
            Ok(live::quote(w, l, slot).map_or(Value::Null, |o| offer_json(w, l, &o, 0)))
        })(),
        "draft_trade_up" => (|| {
            let w = game.wm()?;
            live::trade_up(w, num("slot").ok_or("Missing slot.")? as usize)?;
            war_room(w)
        })(),
        "draft_rush" => (|| {
            let w = game.wm()?;
            live::rush(w)?;
            Ok(draft_record(w))
        })(),
        "draft_record" => (|| Ok(draft_record(game.w()?)))(),
        "predraft" => (|| predraft_view(game.w()?))(),
        "leak" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            let said = predraft::plant(w, pid, args.get("real").and_then(Value::as_bool).unwrap_or(false))?;
            Ok(json!({ "said": said, "predraft": predraft_view(w)? }))
        })(),
        "pro_scouting" => (|| pro_scouting(game.w()?, args))(),
        "tryout" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            sc::pro::tryout(w, pid)?;
            pro_scouting(w, &json!({}))
        })(),
        "supp_bid" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            let round = num("round").unwrap_or(0) as u32;
            w.scouting.pro.supp_bids.retain(|b| b.0 != pid);
            if round > 0 {
                w.scouting.pro.supp_bids.push((pid, round.clamp(1, 7)));
            }
            pro_scouting(w, &json!({}))
        })(),
        "recruit_scouting" => (|| recruit_scouting(game.w()?, args))(),
        "hs_side" => (|| hs_side(game.w()?))(),
        "promote" => (|| {
            let pid = id()?;
            let w = game.wm()?;
            let program = num("program").map(|p| p as usize).filter(|&p| p < w.programs.len());
            let said = hs::promote(w, pid, program, num("strength").unwrap_or(0) as u8)?;
            Ok(json!({ "said": said, "hs": hs_side(w)? }))
        })(),
        "lookback" => (|| Ok(lookback_view(game.w()?)))(),
        _ => return None,
    };
    Some(r)
}

// --- Shared pieces -------------------------------------------------------------------------

pub(crate) const REGION_NAMES: [&str; 8] = [
    "Northeast",
    "Mid-Atlantic",
    "Deep South",
    "Midwest",
    "Plains",
    "Texas and Louisiana",
    "Mountain West",
    "West Coast",
];

fn staff_of(w: &World) -> Result<readers::Staff, String> {
    readers::seat_staff(w).ok_or_else(|| "Start a career first.".to_string())
}

fn person(w: &World, id: u32) -> Result<(usize, &Person), String> {
    let i = sc::place(w, id).ok_or("That player is no longer in football.")?;
    Ok((i, &w.people[i]))
}

/// Where a player is now, in a few words.
fn school_of(w: &World, p: &Person) -> String {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            w.high_schools.get(p.hs).map_or(String::new(), |s| w.school_name(s.id).to_string())
        }
        Stage::College | Stage::Declared => p.college.map_or(String::new(), |c| w.program_name(c).to_string()),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map_or("Free agent".into(), |t| w.nfl_team_name(t)),
        _ => p.college.map_or(String::new(), |c| w.program_name(c).to_string()),
    }
}

fn team_ref_of(w: &World, p: &Person) -> Option<TeamRef> {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => Some(TeamRef::HighSchool(p.hs)),
        Stage::College | Stage::Declared => p.college.map(TeamRef::College),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map(TeamRef::Nfl),
        _ => None,
    }
    .filter(|t| match *t {
        TeamRef::HighSchool(h) => h < w.high_schools.len(),
        TeamRef::College(c) => c < w.programs.len(),
        TeamRef::Nfl(t) => t < w.nfl.len(),
        TeamRef::Other(f) => f < afm_engine::world::pro::other::FRANCHISES,
    })
}

fn class_words(w: &World, p: &Person) -> String {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            let left = p.class_of - w.dates.season;
            match left {
                ..=1 => "Senior".into(),
                2 => "Junior".into(),
                3 => "Sophomore".into(),
                _ => "Freshman".into(),
            }
        }
        Stage::College => {
            let n = p.college_seasons;
            let y = ["Freshman", "Sophomore", "Junior", "Senior", "Fifth-year"][(n as usize).min(4)];
            if p.redshirt_used && n > 0 { format!("{y} (RS)") } else { y.into() }
        }
        Stage::Declared => "Draft prospect".into(),
        Stage::Pro => {
            let acc = p.pro.as_ref().map_or(0, |c| c.accrued);
            if acc == 0 { "Rookie".into() } else { format!("{} accrued seasons", acc) }
        }
        _ => String::new(),
    }
}

fn ft_in(inches: f64) -> String {
    let i = inches.round() as i64;
    format!("{}'{}\"", i / 12, i % 12)
}

/// A player's row for any list: id, name, position, school, class, the
/// public's word.
pub(crate) fn row(w: &World, i: usize) -> Value {
    let p = &w.people[i];
    json!({
        "id": p.player.id,
        "name": p.name,
        "pos": p.player.position.name(),
        "school": school_of(w, p),
        "team": team_ref_of(w, p).map(|t| team_brief(w, t)),
        "class": class_words(w, p),
        "age": p.player.age,
        "ht": ft_in(file::listed(&p.player).0),
        "wt": file::listed(&p.player).1.round(),
        "stars": p.stars,
    })
}

/// Who a reader is, as a link the screens can follow.
fn link_json(r: &file::Reader, code: u32) -> Value {
    match r.link {
        file::Link::Coach(c) => json!({ "kind": "coach", "id": c }),
        file::Link::Scout(_) => json!({ "kind": "scout", "id": code }),
        file::Link::Gm => json!({ "kind": "gm" }),
        file::Link::You => json!({ "kind": "you" }),
        file::Link::Staff => json!({ "kind": "staff", "id": code }),
        file::Link::Event => json!({ "kind": "event" }),
    }
}

fn chips(c: u8) -> Vec<&'static str> {
    let mut v = Vec::new();
    if c & sc::CHIP_FILM != 0 {
        v.push("film");
    }
    if c & sc::CHIP_LIVE != 0 {
        v.push("live");
    }
    if c & sc::CHIP_CAMP != 0 {
        v.push("camp");
    }
    if c & sc::CHIP_TALK != 0 {
        v.push("talk");
    }
    v
}

fn speak(p: &Person) -> file::Speak {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => file::Speak::Recruit,
        Stage::Pro => file::Speak::Pro,
        _ => file::Speak::Prospect,
    }
}

/// The overall read as a range, or None.
fn now_range(f: &file::FileRead) -> Option<[u8; 2]> {
    if f.now.1 <= 0.0 && f.now.0 <= 0.0 {
        return None;
    }
    let half = f.now.1.max(1.0);
    Some([(f.now.0 - half).floor().clamp(1.0, 99.0) as u8, (f.now.0 + half).ceil().clamp(1.0, 99.0) as u8])
}

fn group_name(cat: Category) -> &'static str {
    match cat {
        Category::Physical => "Physical",
        Category::Passing => "Passing",
        Category::Running => "Running",
        Category::Receiving => "Receiving",
        Category::LinePlay => "Line play",
        Category::Defense => "Defense",
        Category::KickingAndSnapping => "Kicking and snapping",
        Category::Mental => "Mental",
    }
}

const CATS: [Category; 8] = [
    Category::Physical,
    Category::Passing,
    Category::Running,
    Category::Receiving,
    Category::LinePlay,
    Category::Defense,
    Category::KickingAndSnapping,
    Category::Mental,
];

/// One "My players" list (the core desk's): following from a scouting file
/// puts him on it, with why and your staff's grade that day kept here.
fn is_followed(w: &World, id: u32) -> bool {
    w.career.as_ref().is_some_and(|c| c.desk.is_listed(id))
}

/// A short read of a player for a list row: the now range, the role words
/// and how many looks.
pub(crate) fn brief_read(w: &World, staff: readers::Staff, id: u32) -> Value {
    let Some(i) = sc::place(w, id) else { return Value::Null };
    let p = &w.people[i];
    match readers::read(w, staff, id) {
        Some((_, looks, f)) => json!({
            "now": now_range(&f),
            "role": file::tier_words(speak(p), f.ceiling.0),
            "sure": file::sure_words(f.ceiling.1),
            "looks": looks.len(),
            "medical": file::medical_words(f.medical.grade),
            "examined": f.medical.examined,
        }),
        None => json!({ "now": Value::Null, "role": "Not seen", "looks": 0 }),
    }
}

// --- The scouting file (screens topic 4, scouting topics 1, 5-7) -------------------------

pub fn scout_file(w: &World, id: u32) -> Reply {
    let staff = staff_of(w)?;
    let (i, p) = person(w, id)?;
    let pos = p.player.position;
    let read = readers::read(w, staff, id);
    let key = readers::staff_key(staff);
    let seen = read.is_some();
    let (rds, looks, f) = match read {
        Some((r, l, f)) => (r, l, Some(f)),
        None => (Vec::new(), Vec::new(), None),
    };
    let (_, _, codes) = readers::evidence(w, staff, id);
    // Summary.
    let summary = f.as_ref().map(|f| {
        let physical = Attr::ALL
            .iter()
            .filter(|a| a.category() == Category::Physical && f.attrs[a.index()].seen)
            .map(|a| f.attrs[a.index()].centre)
            .fold(0.0f64, f64::max);
        let hints: Vec<Value> = file::hints(w, i, &rds, &looks)
            .into_iter()
            .map(|(t, r)| json!({ "text": t, "by": rds.get(r).map(|x| x.name.clone()) }))
            .collect();
        let eye = rds.iter().map(|r| r.potential).fold(8.0f64, f64::max);
        let comp = file::comparison(w, i, eye, f).map(|(n, at, pid)| json!({ "name": n, "where": at, "id": pid }));
        // The best and worst of him, from the attributes that matter at his position.
        let mut strong: Vec<(f64, Attr)> = Attr::ALL
            .iter()
            .filter(|a| f.attrs[a.index()].seen && file::matters(pos, **a) > 0.4)
            .map(|&a| (f.attrs[a.index()].centre, a))
            .collect();
        strong.sort_by(|a, b| b.0.total_cmp(&a.0));
        let best: Vec<&str> = strong.iter().take(3).map(|x| lines::attr_phrase(x.1, true)).collect();
        let worst: Vec<&str> = strong.iter().rev().take(2).map(|x| lines::attr_phrase(x.1, false)).collect();
        json!({
            "now": now_range(f),
            "role": file::tier_words(speak(p), f.ceiling.0),
            "floor": file::floor_words(f.now.0, f.ceiling.0, f.ceiling.1, p.player.age, physical),
            "sure": file::sure_words(f.ceiling.1),
            "hints": hints,
            "comparison": comp,
            "strengths": best,
            "weaknesses": worst,
            "medical": file::medical_words(f.medical.grade),
            "medical_grade": f.medical.grade,
            "examined": f.medical.examined,
        })
    });
    // Ratings by group.
    let groups: Vec<Value> = CATS
        .iter()
        .map(|&cat| {
            let attrs: Vec<Value> = Attr::ALL
                .iter()
                .filter(|a| a.category() == cat)
                .map(|&a| {
                    let r = f.as_ref().map(|f| f.attrs[a.index()]);
                    let per: Vec<Value> = f
                        .as_ref()
                        .map(|f| {
                            f.per_reader
                                .iter()
                                .enumerate()
                                .filter_map(|(k, v)| {
                                    let (c, sd) = v.get(a.index()).copied().flatten()?;
                                    Some(json!({
                                        "who": rds.get(k).map(|x| x.name.clone()),
                                        "read": [(c - sd).floor().clamp(1.0, 99.0) as u8, (c + sd).ceil().clamp(1.0, 99.0) as u8],
                                    }))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    json!({
                        "key": a.key(),
                        "label": a.label(),
                        "read": r.and_then(|r| r.range()),
                        "chips": r.map_or(Vec::new(), |r| chips(r.chips)),
                        "readers": r.map_or(0, |r| r.readers),
                        "matters": file::matters(pos, a) > 0.4,
                        "per": per,
                    })
                })
                .collect();
            json!({ "name": group_name(cat), "attrs": attrs })
        })
        .collect();
    // Looks, newest first.
    let mut look_rows: Vec<Value> = looks
        .iter()
        .map(|l| {
            let r = &rds[l.reader];
            json!({
                "date": date_text(l.date),
                "who": r.name,
                "job": r.job,
                "link": link_json(r, codes.get(l.reader).copied().unwrap_or(0)),
                "kind": l.kind.name(),
                "family": l.kind.family(),
            })
        })
        .collect();
    look_rows.reverse();
    // Tendencies: a staff's read, narrower with film.
    let film = looks.iter().filter(|l| matches!(l.kind.family(), "Film" | "Live")).count() as f64;
    let acc = rds.iter().map(|r| r.judge).fold(0.0f64, f64::max);
    let tendencies: Vec<Value> = if seen {
        afm_engine::tendencies::Tend::ALL
            .iter()
            .filter(|t| t.owned_by(pos))
            .map(|&t| {
                let mut rng = afm_engine::rng::Rng::new(id ^ (t.index() as u32) << 24 ^ key as u32 ^ w.year as u32);
                let shown = afm_engine::tendencies::shown(t, p.player.tend.get(t), pos);
                let r = afm_engine::tendencies::scout(shown, film * 1.5, acc.max(6.0), &mut rng);
                let (lo, hi) = t.ends();
                json!({ "label": t.label(), "read": [r.low, r.high], "word": r.word.to_string(), "ends": [lo, hi] })
            })
            .collect()
    } else {
        Vec::new()
    };
    // Character: the 13 reads with their main source.
    let character: Vec<Value> = f
        .as_ref()
        .map(|f| {
            f.traits
                .iter()
                .enumerate()
                .map(|(k, t)| {
                    if !t.seen {
                        return json!({ "name": file::TRAIT_NAMES[k], "word": "Not read yet" });
                    }
                    let main = looks.get(t.main);
                    let (src, shade) = main.map_or(("our staff", file::Shade::True), |l| {
                        lines::note_source(l.kind, key, id as u64 ^ k as u64)
                    });
                    json!({
                        "name": file::TRAIT_NAMES[k],
                        "word": lines::trait_word(k, t.centre),
                        "good": if file::trait_good_high(k) { t.centre >= 11.0 } else { t.centre <= 9.0 },
                        "sure": file::sure_words(t.sd * 1.6),
                        "note": lines::trait_note(k, t.centre, shade, key, id as u64),
                        "source": src,
                        "shade": shade.words(),
                        "who": main.and_then(|l| rds.get(l.reader)).map(|r| r.name.clone()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let family: Vec<String> = if seen { lines::family_notes(p, key) } else { Vec::new() };
    // Medical and academics.
    let medical = f.as_ref().map(|f| {
        json!({ "grade": f.medical.grade, "words": file::medical_words(f.medical.grade), "examined": f.medical.examined, "notes": f.medical.notes })
    });
    let transcript = looks.iter().any(|l| l.kind == Kind::Transcript);
    let gpa_half = if transcript { 0.05 } else { 0.25 };
    let academics = json!({
        "gpa": [((file::gpa(p) - gpa_half) * 10.0).round() / 10.0, ((file::gpa(p) + gpa_half).min(4.0) * 10.0).round() / 10.0],
        "transcript": transcript,
        "test": p.cal.test_score,
        "eligible": if p.cal.fall_ineligible { "Ineligible this fall" } else if p.cal.academic_sit { "Sitting out for grades" } else { "Eligible" },
    });
    // Testing: the combine, a pro day or a camp, when one was in the file.
    let tests: Vec<Value> = [Kind::Combine, Kind::ProDay, Kind::Camp, Kind::Showcase]
        .iter()
        .filter_map(|&k| {
            let l = looks.iter().rev().find(|l| l.kind == k)?;
            let c = file::tests(&p.player, k, l.date.year());
            Some(json!({
                "where": k.name(), "date": date_text(l.date),
                "forty": c.forty, "bench": c.bench.round(), "vertical": c.vertical, "broad": c.broad.round(), "cone": c.cone, "shuttle": c.shuttle,
            }))
        })
        .collect();
    // History: his seasons as the record shows them.
    let mut history: Vec<Value> = p
        .player
        .history
        .iter()
        .map(|h| json!({ "age": h.age, "level": format!("{:?}", h.level), "pos": h.position.name(), "missed": h.games_missed, "role": format!("{:?}", h.playing_time) }))
        .collect();
    history.reverse();
    // The room's voices (each reader with three or more looks).
    let mut counts: HashMap<usize, usize> = HashMap::new();
    for l in &looks {
        *counts.entry(l.reader).or_default() += 1;
    }
    let mut voices: Vec<(usize, usize)> = counts.into_iter().filter(|x| x.1 >= 3).collect();
    voices.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let voices: Vec<Value> = voices
        .into_iter()
        .take(4)
        .filter_map(|(r, n)| {
            // Bug 18: each voice comes from his own read.
            let rd = &rds[r];
            let says = afm_engine::world::scouting::own::voice(w, i, f.as_ref()?, &rds, &looks, r, speak(p))?;
            Some(json!({ "who": rd.name, "job": rd.job, "looks": n, "says": says }))
        })
        .collect();
    // Board flags.
    let b = &w.scouting.board;
    let our_rank = b.order.iter().position(|&x| x == id);
    let public_rank = if matches!(p.stage, Stage::Declared | Stage::College) {
        board::public_ranks(w).get(&id).copied()
    } else {
        None
    };
    let lists: Vec<&str> = w
        .scouting
        .predraft
        .lists
        .iter()
        .filter(|x| x.0 == id)
        .map(|x| predraft::LIST_NAMES[(x.1 as usize).min(3)])
        .collect();
    let followers = Some(p.followers).filter(|&f| f > 0);
    let choices = w.scouting.predraft.choices.get(&id).map_or(Vec::new(), |&c| predraft::choice_words(c));
    let (lh, lw) = file::listed(&p.player);
    // Measured is his body now (a boy still growing is smaller than the man
    // he becomes).
    let now = afm_engine::body::body_now(&p.player);
    let measured = looks.iter().any(|l| matches!(l.kind, Kind::Combine | Kind::ProDay | Kind::Camp | Kind::Showcase));
    let film_now = sc::own::film_cost(w, p.player.id)
        .ok()
        .map(|(_, h, what)| json!({ "hours": h, "what": what, "left": sc::own::film_left(w), "week": sc::own::film_budget(w) }));
    Ok(json!({
        "id": id,
        "name": p.name,
        "pos": pos.name(),
        "group": Group::of(pos).name(),
        "age": p.player.age,
        "class": class_words(w, p),
        "school": school_of(w, p),
        "team": team_ref_of(w, p).map(|t| team_brief(w, t)),
        "hometown": w.states.get(p.home_state).map(|s| s.code.to_string()),
        "high_school": w.high_schools.get(p.hs).map(|s| w.school_name(s.id).to_string()),
        "ht": ft_in(if measured { now.height } else { lh }),
        "wt": (if measured { now.weight } else { lw }).round(),
        "measured": measured,
        "arm": measured.then_some(now.arm),
        "hand": measured.then_some(now.hand),
        "stars": p.stars,
        "mine": matches!(staff, readers::Staff::Club(t) if p.pro.as_ref().is_some_and(|c| c.team == Some(t)))
            || matches!(staff, readers::Staff::Program(c) if p.college == Some(c) && p.stage == Stage::College)
            || matches!(staff, readers::Staff::School(h) if p.hs == h && p.stage == Stage::HighSchool),
        "followed": is_followed(w, id),
        "seen": seen,
        "looks_n": looks.len(),
        "readers_n": rds.len(),
        "summary": summary,
        "groups": groups,
        "looks": look_rows,
        "tendencies": tendencies,
        "character": character,
        "family": family,
        "medical": medical,
        "academics": academics,
        "tests": tests,
        "history": history,
        "stories": p.notes.iter().rev().take(12).collect::<Vec<_>>(),
        "voices": voices,
        "board": {
            "rank": our_rank.map(|r| r + 1),
            "tier": b.tier_of(id),
            "dnd": b.dnd.iter().find(|x| x.0 == id).map(|x| x.1.clone()),
            "pfa": b.pfa.iter().any(|x| x.0 == id),
            "lists": lists,
            "public": public_rank.map(|r| r + 1),
        },
        "choices": choices,
        "followers": followers.map(social::words),
        "aura": seen.then(|| afm_engine::world::fame::aura_read(p, rds.iter().map(|r| r.character).fold(6.0f64, f64::max), key)),
        "brand": if p.brand > 0.0 { Some(p.brand.round()) } else { None },
        "followers_n": followers,
        "nil": if p.nil_income > 0.0 { Some(p.nil_income.round()) } else { None },
        "can_ask": ask_kinds(w, p).into_iter().filter(|k| *k != "Watch his film now").collect::<Vec<_>>(),
    }))
    .map(|mut v: Value| {
        v["film_now"] = film_now.unwrap_or(Value::Null);
        v
    })
}

/// The looks you can ask your staff for on this player, by your seat.
fn ask_kinds(w: &World, p: &Person) -> Vec<&'static str> {
    let Some(s) = w.seat else { return Vec::new() };
    let mut v = match s.level {
        CalLevel::Nfl if matches!(p.stage, Stage::College | Stage::Declared) => {
            vec!["See him again", "Cross-check", "Private workout", "Background check", "Medical recheck"]
        }
        CalLevel::Nfl => vec!["Pro film", "Tryout"],
        CalLevel::College if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) => {
            vec!["Evaluate in person", "Film study", "Call his coach", "Transcript", "Social media"]
        }
        CalLevel::College => vec!["Film study", "Call his old staff", "Transcript"],
        _ => Vec::new(),
    };
    // Bug 82: you watch his film yourself, today, on your own hours.
    if sc::own::film_cost(w, p.player.id).is_ok() {
        v.insert(0, "Watch his film now");
    }
    v
}

fn ask(w: &mut World, id: u32, what: &str) -> Result<String, String> {
    let (_, p) = person(w, id)?;
    let name = p.name.clone();
    let d = w.today;
    let kind = match what {
        "Watch his film now" => return sc::own::watch_film(w, id),
        "See him again" => Kind::School,
        "Cross-check" => Kind::CrossCheck,
        "Private workout" => Kind::Workout,
        "Background check" => Kind::Background,
        "Medical recheck" => Kind::Medical,
        "Pro film" => Kind::ProFilm,
        "Tryout" => {
            sc::pro::tryout(w, id)?;
            return Ok(format!("{name} is in for a tryout on Tuesday."));
        }
        "Evaluate in person" => Kind::Practice,
        "Film study" => Kind::Film,
        "Call his coach" | "Call his old staff" => Kind::Call,
        "Transcript" => Kind::Transcript,
        "Social media" => Kind::Social,
        _ => return Err("Unknown look.".into()),
    };
    if w.scouting.dept.asks.iter().any(|a| a.0 == id && a.1 == kind) {
        return Err(format!("Your staff already has that on its list for {name}."));
    }
    // A private workout or a recheck is the spring's (topic 10); an
    // evaluation waits for a day the rules allow.
    w.scouting.dept.asks.push((id, kind, d));
    Ok(format!("Done: {} for {name} is on your staff's list.", kind.name().to_lowercase()))
}

fn follow(w: &mut World, id: u32, why: &str) -> Result<(), String> {
    if is_followed(w, id) {
        return Ok(());
    }
    let staff = staff_of(w)?;
    let (_, p) = person(w, id)?;
    let read = brief_read(w, staff, id);
    let first = read["role"].as_str().unwrap_or("Not seen").to_string();
    let public = if p.stars > 0 { format!("{}-star", p.stars) } else { String::from("Unranked") };
    let d = w.today;
    w.scouting.dept.my_players.retain(|f| f.id != id);
    w.scouting.dept.my_players.push(dept::Followed {
        id,
        since: d,
        why: if why.is_empty() { "Followed".into() } else { why.to_string() },
        first_read: first,
        public_then: public,
    });
    crate::lane_core::desk::list_add(w, id, 0);
    Ok(())
}

/// The department's view of My players: the core list, with why you
/// followed him and your staff's grade then where the scouting file kept it.
fn my_players(w: &World) -> Value {
    let Ok(staff) = staff_of(w) else { return json!({ "rows": [] }) };
    let Some(c) = &w.career else { return json!({ "rows": [] }) };
    let rows: Vec<Value> = c
        .desk
        .listed
        .iter()
        .filter_map(|l| {
            let i = sc::place(w, l.id)?;
            let f = w.scouting.dept.my_players.iter().find(|f| f.id == l.id);
            let mut r = row(w, i);
            r["since"] = json!(date_text(l.since));
            r["why"] = json!(f.map_or("Added to My players", |f| f.why.as_str()));
            r["first"] = json!(f.map_or_else(
                || if l.read.1 > 0 { format!("{}-{}", l.read.0, l.read.1) } else { "Not seen".into() },
                |f| f.first_read.clone()
            ));
            r["public_then"] = json!(f.map_or_else(
                || if l.stars > 0 { format!("{}-star", l.stars) } else { "Unranked".into() },
                |f| f.public_then.clone()
            ));
            r["read"] = brief_read(w, staff, l.id);
            r["where"] = json!(school_of(w, &w.people[i]));
            Some(r)
        })
        .collect();
    json!({ "rows": rows })
}

// --- Your department (screens topic 6, scouting topics 2-3, 14-17) -----------------------

/// The people who read players for you, by code.
fn staff_people(w: &World, staff: readers::Staff) -> Vec<(u32, String, String, Value)> {
    let mut v: Vec<(u32, String, String, Value)> = Vec::new();
    match staff {
        readers::Staff::Club(t) => {
            if let Some(c) = w.clubs.get(t) {
                let areas =
                    c.scouts.iter().filter(|s| s.role == afm_engine::world::pro::scouts::Role::Area).count().max(1);
                for (k, s) in c.scouts.iter().enumerate() {
                    let (job, area) = match s.role {
                        afm_engine::world::pro::scouts::Role::Director => {
                            ("Director of college scouting".to_string(), "Top of the board".to_string())
                        }
                        afm_engine::world::pro::scouts::Role::National => {
                            ("National scout".to_string(), "Cross-checks".to_string())
                        }
                        afm_engine::world::pro::scouts::Role::Area => {
                            let regions: Vec<&str> = afm_engine::world::pro::scouts::covers(s.region, areas)
                                .into_iter()
                                .map(|r| REGION_NAMES[r as usize])
                                .collect();
                            ("Area scout".to_string(), regions.join(", "))
                        }
                    };
                    v.push((
                        k as u32,
                        s.name.clone(),
                        job,
                        json!({ "area": area, "since": s.since, "age": w.year - s.born }),
                    ));
                }
                let tm = &w.nfl[t];
                for (c, job) in
                    [(tm.head_coach, "Head coach"), (tm.oc, "Offensive coordinator"), (tm.dc, "Defensive coordinator")]
                {
                    if let Some(c) = c {
                        v.push((
                            readers::COACH + c as u32,
                            w.coaches[c].name.clone(),
                            job.into(),
                            json!({ "coach": c }),
                        ));
                    }
                }
                for (k, s) in w.scouting.pro.scouts.iter().enumerate() {
                    let job = if k == 0 { "Director of pro personnel" } else { "Pro scout" };
                    v.push((readers::PRO_SCOUT + k as u32, s.name.clone(), job.into(), json!({ "pro": true })));
                }
            }
        }
        readers::Staff::Program(prog) => {
            let pg = &w.programs[prog];
            let mut coaches: Vec<(usize, String)> = Vec::new();
            if let Some(c) = pg.head_coach {
                coaches.push((c, "Head coach".into()));
            }
            if let Some(c) = pg.oc {
                coaches.push((c, "Offensive coordinator".into()));
            }
            if let Some(c) = pg.dc {
                coaches.push((c, "Defensive coordinator".into()));
            }
            for (g, c) in pg.position_coaches.iter().enumerate() {
                if let Some(c) = c
                    && !coaches.iter().any(|x| x.0 == *c)
                {
                    coaches.push((*c, format!("{} coach", Group::ALL[g].name())));
                }
            }
            for (c, job) in coaches {
                let area: Vec<String> = w
                    .scouting
                    .dept
                    .areas
                    .iter()
                    .find(|a| a.0 == c)
                    .map(|a| a.1.iter().filter_map(|&s| w.states.get(s as usize).map(|x| x.code.to_string())).collect())
                    .unwrap_or_default();
                v.push((
                    readers::COACH + c as u32,
                    w.coaches[c].name.clone(),
                    job,
                    json!({ "coach": c, "area": area.join(", ") }),
                ));
            }
            for (k, p) in readers::personnel(w, prog).into_iter().enumerate() {
                v.push((readers::PERSONNEL + k as u32, p.name, p.job.to_string(), json!({ "since": p.since })));
            }
        }
        readers::Staff::School(h) => {
            if let Some(c) = w.high_schools.get(h).and_then(|s| s.coach) {
                v.push((
                    readers::COACH + c as u32,
                    w.coaches[c].name.clone(),
                    "Head coach".into(),
                    json!({ "coach": c }),
                ));
            }
            if let Some(car) = &w.career {
                for (k, a) in car.hs_staff.iter().enumerate() {
                    v.push((readers::HS_ASSISTANT + k as u32, a.name.clone(), "Assistant coach".into(), json!({})));
                }
            }
        }
    }
    v
}

pub fn department(w: &World) -> Value {
    let Ok(staff) = staff_of(w) else { return json!({ "seat": Value::Null }) };
    let level = w.seat.map(|s| s.level);
    let season = w.dates.season;
    let d = &w.scouting.dept;
    // Looks each reader took this football year.
    let start = w.dates.start;
    let mut by_who: HashMap<u32, usize> = HashMap::new();
    for v in d.looks.values() {
        for l in v.iter().filter(|l| l.date >= start) {
            *by_who.entry(l.who).or_default() += 1;
        }
    }
    if let readers::Staff::Club(t) = staff
        && let Some(c) = w.clubs.get(t)
    {
        for v in c.files.values() {
            for l in v.iter().filter(|l| l.date >= start) {
                *by_who.entry(l.who as u32).or_default() += 1;
            }
        }
    }
    let people: Vec<Value> = staff_people(w, staff)
        .into_iter()
        .map(|(code, name, job, extra)| {
            let assignment = d.assignments.iter().find(|a| a.who == code).map(|a| target_words(w, a.target));
            let (held, high, low) = lookback::record_of(&w.scouting.records, code);
            json!({
                "code": code, "name": name, "job": job, "extra": extra,
                "looks": by_who.get(&code).copied().unwrap_or(0),
                "assignment": assignment,
                "record": { "held": held, "high": high, "low": low },
            })
        })
        .collect();
    let instructions: Vec<Value> = instruction_choices()
        .into_iter()
        .map(|(k, i)| json!({ "key": k, "words": i.words(), "on": d.instructions.contains(&i) }))
        .collect();
    let (fall, spring) = dept::eval_limits(season);
    let days: Vec<Value> = d
        .days_log
        .iter()
        .rev()
        .take(60)
        .map(|&(date, who, pid)| {
            let name = staff_people_name(w, staff, who);
            let player = sc::place(w, pid).map(|i| json!({ "id": pid, "name": w.people[i].name }));
            json!({ "date": date_text(date), "who": name, "player": player })
        })
        .collect();
    let asks: Vec<Value> = d
        .asks
        .iter()
        .filter_map(|&(pid, kind, date)| {
            let i = sc::place(w, pid)?;
            Some(json!({ "id": pid, "name": w.people[i].name, "kind": kind.name(), "asked": date_text(date) }))
        })
        .collect();
    // The board of players your staff is on (top of the reads).
    let mut on_file: Vec<(usize, u32)> = d.looks.iter().map(|(&id, v)| (v.len(), id)).collect();
    if let readers::Staff::Club(t) = staff
        && let Some(c) = w.clubs.get(t)
    {
        on_file.extend(c.files.iter().map(|(&id, v)| (v.len(), id)));
    }
    on_file.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut seen_ids = HashSet::new();
    let most_seen: Vec<Value> = on_file
        .into_iter()
        .filter(|x| seen_ids.insert(x.1))
        .take(40)
        .filter_map(|(n, id)| {
            let i = sc::place(w, id)?;
            let mut r = row(w, i);
            r["looks"] = json!(n);
            Some(r)
        })
        .collect();
    json!({
        "seat": format!("{:?}", level.unwrap_or(CalLevel::College)),
        "season": season,
        "people": people,
        "instructions": instructions,
        "assign_targets": assign_targets(w, staff),
        "eval": { "fall": [d.eval_days.1, fall], "spring": [d.eval_days.2, spring], "season": d.eval_days.0 },
        "days": days,
        "asks": asks,
        "most_seen": most_seen,
        "my_players": my_players(w)["rows"],
        "public": public_boards(w),
        "lookback": lookback_view(w),
    })
}

fn staff_people_name(w: &World, staff: readers::Staff, who: u32) -> String {
    readers::reader(w, staff, who, 0).map_or_else(|| "Staff".into(), |r| r.name)
}

fn target_words(w: &World, t: dept::Target) -> String {
    match t {
        dept::Target::Player(id) => {
            sc::place(w, id).map_or("A player".into(), |i| format!("See {} again", w.people[i].name))
        }
        dept::Target::Region(r) => match w.seat.map(|s| s.level) {
            Some(CalLevel::Nfl) => REGION_NAMES.get(r as usize).map_or("A region".into(), |s| format!("The {s}")),
            _ => w.states.get(r as usize).map_or("A state".into(), |s| s.code.to_string()),
        },
        dept::Target::Group(g) => {
            format!("Every {} in the class", Group::ALL[(g as usize).min(7)].name().to_lowercase())
        }
        dept::Target::CrossCheck => "Cross-check the top of the board".into(),
    }
}

fn instruction_choices() -> Vec<(&'static str, dept::Instruction)> {
    let mut v: Vec<(&'static str, dept::Instruction)> = vec![
        ("small", dept::Instruction::SmallSchools),
        ("fit", dept::Instruction::SchemeFit),
        ("ask_dnd", dept::Instruction::AskBeforeDnd),
        ("analytics", dept::Instruction::Analytics),
    ];
    const FOCUS: [&str; 8] = ["focus0", "focus1", "focus2", "focus3", "focus4", "focus5", "focus6", "focus7"];
    for (g, k) in FOCUS.iter().enumerate() {
        v.push((k, dept::Instruction::Focus(g as u8)));
    }
    v
}

fn assign_targets(w: &World, staff: readers::Staff) -> Vec<Value> {
    let mut v = vec![json!({ "key": "cross", "words": "Cross-check the top of the board" })];
    for (g, gr) in Group::ALL.iter().enumerate() {
        v.push(
            json!({ "key": format!("group{g}"), "words": format!("Every {} in the class", gr.name().to_lowercase()) }),
        );
    }
    match staff {
        readers::Staff::Club(_) => {
            for (r, n) in REGION_NAMES.iter().enumerate() {
                v.push(json!({ "key": format!("region{r}"), "words": format!("The {n}") }));
            }
        }
        _ => {
            for (k, s) in w.states.iter().enumerate() {
                v.push(json!({ "key": format!("region{k}"), "words": s.code }));
            }
        }
    }
    v
}

fn assign(w: &mut World, args: &Value) -> Result<(), String> {
    let who = args.get("who").and_then(Value::as_u64).ok_or("Missing who.")? as u32;
    let key = args.get("target").and_then(Value::as_str).unwrap_or("");
    let d = w.today;
    w.scouting.dept.assignments.retain(|a| a.who != who);
    let target = if key.is_empty() || key == "none" {
        return Ok(());
    } else if key == "cross" {
        dept::Target::CrossCheck
    } else if let Some(g) = key.strip_prefix("group") {
        dept::Target::Group(g.parse::<u8>().map_err(|_| "Bad group.")?.min(7))
    } else if let Some(r) = key.strip_prefix("region") {
        dept::Target::Region(r.parse::<u8>().map_err(|_| "Bad region.")?)
    } else if let Some(p) = key.strip_prefix("player") {
        dept::Target::Player(p.parse::<u32>().map_err(|_| "Bad player.")?)
    } else {
        return Err("Unknown assignment.".into());
    };
    w.scouting.dept.assignments.push(dept::Assignment { who, target, since: d });
    Ok(())
}

fn instruct(w: &mut World, args: &Value) -> Result<(), String> {
    let key = args.get("key").and_then(Value::as_str).ok_or("Missing key.")?;
    let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
    let (_, i) = instruction_choices().into_iter().find(|x| x.0 == key).ok_or("Unknown instruction.")?;
    let v = &mut w.scouting.dept.instructions;
    v.retain(|x| *x != i);
    if on {
        // One focus at a time.
        if matches!(i, dept::Instruction::Focus(_)) {
            v.retain(|x| !matches!(x, dept::Instruction::Focus(_)));
        }
        v.push(i);
    }
    Ok(())
}

// --- The public's word (topic 8): services, analysts, boards and mocks -------------------

pub fn public_boards(w: &World) -> Value {
    let year = w.year;
    let s = &w.scouting.public;
    let name = |id: u32| sc::place(w, id).map(|i| row(w, i));
    let desks: Vec<Value> = w
        .scouting
        .desks
        .iter()
        .enumerate()
        .filter(|(_, d)| d.active(year))
        .map(|(k, d)| {
            let board: Vec<Value> = s
                .boards
                .iter()
                .find(|b| b.0 as usize == k)
                .map_or(Vec::new(), |b| b.2.iter().take(32).filter_map(|&id| name(id)).collect());
            let mock: Vec<Value> = s
                .mocks
                .iter()
                .find(|m| m.0 as usize == k)
                .map_or(Vec::new(), |m| m.2.iter().take(32).filter_map(|&id| name(id)).collect());
            json!({
                "k": k,
                "name": d.name,
                "outlet": d.outlet,
                "kind": if d.kind == public::DeskKind::Service { "Recruiting service" } else { "Draft analyst" },
                "lean": d.lean.words(),
                "record": { "drafts": d.record.0, "exact": d.record.1, "within3": d.record.2 },
                "since": d.from,
                "board": board,
                "mock": mock,
                "updated": s.boards.iter().find(|b| b.0 as usize == k).map(|b| date_text(b.1)),
            })
        })
        .collect();
    let consensus: Vec<Value> = s.weekly.last().map_or(Vec::new(), |c| c.1.iter().filter_map(|&id| name(id)).collect());
    let prev: HashMap<u32, usize> = s
        .weekly
        .iter()
        .rev()
        .nth(1)
        .map_or(HashMap::new(), |c| c.1.iter().enumerate().map(|(k, &id)| (id, k)).collect());
    let moves: Vec<Value> = s.weekly.last().map_or(Vec::new(), |c| {
        c.1.iter()
            .enumerate()
            .map(|(k, id)| prev.get(id).map_or(json!(null), |&p| json!(p as i64 - k as i64)))
            .collect()
    });
    json!({
        "draft_year": s.year,
        "desks": desks,
        "consensus": consensus,
        "moves": moves,
        "consensus_date": s.weekly.last().map(|c| date_text(c.0)),
        "scored": s.scored.iter().filter_map(|&(k, exact, within, of)| {
            let d = w.scouting.desks.get(k as usize)?;
            Some(json!({ "name": d.name, "outlet": d.outlet, "exact": exact, "within3": within, "of": of }))
        }).collect::<Vec<_>>(),
    })
}

// --- The draft board (screens topic 6, scouting topic 9) ---------------------------------

fn my_club(w: &World) -> Result<usize, String> {
    w.seat
        .filter(|s| s.level == CalLevel::Nfl)
        .and_then(|s| s.nfl_team)
        .ok_or_else(|| "Only an NFL club keeps a draft board.".into())
}

pub fn draft_board(w: &World) -> Reply {
    let t = my_club(w)?;
    let grades = board::seat_grades(w);
    let order = board::ordered(w, &grades);
    let public = board::public_ranks(w);
    let teams = afm_engine::world::franchise::active_teams_in(w, w.year).len().max(1);
    let b = &w.scouting.board;
    let taken: HashSet<u32> = w
        .scouting
        .live
        .as_ref()
        .map_or(HashSet::new(), |l| l.run.picked.iter().map(|x| w.people[x.0].player.id).collect());
    let best_mock = &w.scouting.public.best_mock;
    let rows: Vec<Value> = order
        .iter()
        .take(400)
        .enumerate()
        .filter_map(|(k, &id)| {
            let i = sc::place(w, id)?;
            let p = &w.people[i];
            let tier = b.tier_of(id).unwrap_or_else(|| board::tier_of_rank(k, teams));
            let pub_rank = public.get(&id).copied();
            let mut r = row(w, i);
            r["rank"] = json!(k + 1);
            r["tier"] = json!(tier);
            r["grade_words"] = json!(board::round_words(tier));
            r["public"] = json!(pub_rank.map(|x| x + 1));
            r["limb"] = json!(board::limb(k, pub_rank));
            r["dnd"] = json!(b.dnd.iter().find(|x| x.0 == id).map(|x| x.1.clone()));
            r["pfa"] = json!(b.pfa.iter().any(|x| x.0 == id));
            r["lists"] = json!(w.scouting.predraft.lists.iter().filter(|x| x.0 == id).map(|x| x.1).collect::<Vec<_>>());
            r["taken"] = json!(taken.contains(&id));
            r["mock"] = json!(best_mock.get(&id));
            r["group"] = json!(Group::of(p.player.position).name());
            r["choices"] =
                json!(w.scouting.predraft.choices.get(&id).map_or(Vec::new(), |&c| predraft::choice_words(c)));
            Some(r)
        })
        .collect();
    // Your picks this draft.
    let year = public::draft_year(w);
    let picks: Vec<Value> = board::coming_slots(w)
        .iter()
        .enumerate()
        .filter(|(_, s)| s.0 == t)
        .map(|(k, s)| json!({ "pick": k + 1, "round": s.1 }))
        .collect();
    let tiers: Vec<Value> = board::TIERS.iter().enumerate().map(|(k, n)| json!({ "tier": k + 1, "name": n })).collect();
    let groups: Vec<&str> = Group::ALL.iter().map(|g| g.name()).collect();
    Ok(json!({
        "year": year,
        "team": team_brief(w, TeamRef::Nfl(t)),
        "rows": rows,
        "tiers": tiers,
        "groups": groups,
        "picks": picks,
        "live": w.scouting.live.is_some(),
        "lists": predraft::LIST_NAMES.iter().enumerate().map(|(k, n)| json!({
            "k": k, "name": n, "cap": predraft::list_cap(k as u8, year),
            "n": w.scouting.predraft.lists.iter().filter(|x| x.1 == k as u8).count(),
        })).collect::<Vec<_>>(),
        "pfa_pool": undrafted_pool(w),
        "director": director_name(w, t),
    }))
}

fn director_name(w: &World, t: usize) -> Option<String> {
    w.clubs
        .get(t)
        .and_then(|c| c.scouts.iter().find(|s| s.role == afm_engine::world::pro::scouts::Role::Director))
        .map(|s| s.name.clone())
}

fn undrafted_pool(w: &World) -> f64 {
    afm_engine::world::pro::scouts::udfa_pool(w.year)
}

fn board_move(w: &mut World, id: u32, args: &Value) -> Result<(), String> {
    my_club(w)?;
    let year = public::draft_year(w);
    let grades = board::seat_grades(w);
    if !grades.contains_key(&id) {
        return Err("He isn't on your board.".into());
    }
    let mut order = board::ordered(w, &grades);
    w.scouting.board.for_year(year);
    if let Some(tier) = args.get("tier").and_then(Value::as_u64) {
        let b = &mut w.scouting.board;
        b.tiers.retain(|x| x.0 != id);
        b.tiers.push((id, (tier as u8).clamp(1, 8)));
    }
    if let Some(to) = args.get("to").and_then(Value::as_u64) {
        order.retain(|&x| x != id);
        let to = (to as usize).saturating_sub(1).min(order.len());
        order.insert(to, id);
        // Your order is kept for the top 400.
        order.truncate(400);
        w.scouting.board.order = order;
    }
    Ok(())
}

fn board_dnd(w: &mut World, id: u32, on: bool, why: &str) -> Result<(), String> {
    my_club(w)?;
    let year = public::draft_year(w);
    w.scouting.board.for_year(year);
    let b = &mut w.scouting.board;
    b.dnd.retain(|x| x.0 != id);
    if on {
        b.dnd.push((id, if why.is_empty() { "Your call".into() } else { why.to_string() }));
    } else if !b.restored.contains(&id) {
        b.restored.push(id);
    }
    Ok(())
}

fn board_pfa(w: &mut World, id: u32, args: &Value) -> Result<(), String> {
    my_club(w)?;
    let year = public::draft_year(w);
    w.scouting.board.for_year(year);
    let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
    let bonus = args.get("bonus").and_then(Value::as_u64).unwrap_or(0) as u32;
    let guarantee = args.get("guarantee").and_then(Value::as_u64).unwrap_or(0) as u32;
    let pool = undrafted_pool(w);
    let b = &mut w.scouting.board;
    b.pfa.retain(|x| x.0 != id);
    if on {
        let spent: u32 = b.pfa.iter().map(|x| x.1).sum();
        if (spent + bonus) as f64 > pool * 1.0001 {
            return Err(format!("That's over your undrafted pool (${:.0}).", pool));
        }
        b.pfa.push((id, bonus, guarantee));
    }
    Ok(())
}

fn board_list(w: &mut World, id: u32, list: u8, on: bool) -> Result<(), String> {
    my_club(w)?;
    let list = list.min(3);
    let cap = predraft::list_cap(list, w.year);
    let pd = &mut w.scouting.predraft;
    pd.lists.retain(|x| !(x.0 == id && x.1 == list));
    if on {
        if pd.lists.iter().filter(|x| x.1 == list).count() >= cap {
            return Err(format!("Your {} list is full ({cap}).", predraft::LIST_NAMES[list as usize].to_lowercase()));
        }
        pd.lists.push((id, list));
        // A private workout or a recheck happens when you put him on the list
        // (your staff travels to him in the spring).
        let d = w.today;
        if list == predraft::LIST_WORKOUT || list == predraft::LIST_MEDICAL {
            let who = match list {
                predraft::LIST_WORKOUT => w
                    .seat
                    .and_then(|s| s.nfl_team)
                    .and_then(|t| w.nfl[t].head_coach)
                    .map_or(afm_engine::world::pro::scouts::HC as u32, |c| readers::COACH + c as u32),
                _ => readers::MEDICAL,
            };
            let kind = if list == predraft::LIST_WORKOUT { Kind::Workout } else { Kind::Medical };
            w.scouting.dept.asks.push((id, kind, d));
            let _ = who;
        }
    }
    Ok(())
}

// --- The war room (screens topic 8, scouting topic 11) -----------------------------------

fn offer_json(w: &World, l: &live::LiveDraft, o: &live::TradeOffer, k: usize) -> Value {
    let slot_text = |s: usize| l.run.board.get(s).map_or(String::new(), |x| format!("pick {} (round {})", s + 1, x.1));
    let mut gets = vec![slot_text(o.their_slot)];
    if let Some(s) = o.slot_extra {
        gets.push(slot_text(s));
    }
    if let Some(r) = o.future {
        gets.push(format!("their {} round {r} pick", w.year + 1));
    }
    json!({
        "k": k,
        "from": team_brief(w, TeamRef::Nfl(o.from)),
        "gets": gets,
        "after": o.after.and_then(|id| sc::place(w, id)).map(|i| row(w, i)),
    })
}

pub fn war_room(w: &World) -> Reply {
    let Some(l) = &w.scouting.live else {
        return Ok(json!({ "live": false, "record": draft_record(w) }));
    };
    let t = l.team;
    let run = &l.run;
    let next = run.next as usize;
    let round = run.board.get(next.saturating_sub(1)).map_or(7, |s| s.1);
    let on_clock = live::on_clock(l);
    let clock = live::clock_minutes(run.year, round);
    let grades = live::grades(w, l);
    let order = board::ordered(w, &grades);
    let left: HashSet<u32> = run.pool.iter().map(|x| w.people[x.0].player.id).collect();
    let public: HashMap<u32, usize> = run.public_rank.iter().map(|(&i, &r)| (w.people[i].player.id, r)).collect();
    let teams = afm_engine::world::franchise::active_teams_in(w, run.year).len().max(1);
    let b = &w.scouting.board;
    // Your board, still there, best first.
    let board_rows: Vec<Value> = order
        .iter()
        .enumerate()
        .filter(|(_, id)| left.contains(id))
        .take(60)
        .filter_map(|(k, &id)| {
            let i = sc::place(w, id)?;
            let tier = b.tier_of(id).unwrap_or_else(|| board::tier_of_rank(k, teams));
            let mut r = row(w, i);
            r["rank"] = json!(k + 1);
            r["tier"] = json!(tier);
            r["public"] = json!(public.get(&id).map(|x| x + 1));
            r["limb"] = json!(board::limb(k, public.get(&id).copied()));
            r["dnd"] = json!(b.is_dnd(id));
            // The last pick he'd last to: the next club with a need at his
            // position and him high on its board (your staff's guess).
            r["last_shot"] = json!(last_shot(w, l, i));
            Some(r)
        })
        .collect();
    // Tier counts left.
    let mut tier_left = [0u32; 8];
    for (k, id) in order.iter().enumerate().filter(|(_, id)| left.contains(id)) {
        let tier = b.tier_of(*id).unwrap_or_else(|| board::tier_of_rank(k, teams));
        tier_left[(tier as usize - 1).min(7)] += 1;
    }
    let feed: Vec<Value> = l
        .feed
        .iter()
        .rev()
        .take(80)
        .map(|f| {
            json!({
                "pick": f.pick, "round": f.round,
                "team": team_brief(w, TeamRef::Nfl(f.team)),
                "player": f.player.and_then(|id| sc::place(w, id)).map(|i| row(w, i)),
                "text": f.text,
                "mine": f.team == t,
            })
        })
        .collect();
    let my_slots: Vec<Value> = run
        .board
        .iter()
        .enumerate()
        .filter(|(_, s)| s.0 == t)
        .map(|(k, s)| json!({ "pick": k + 1, "round": s.1, "used": k + 1 < next }))
        .collect();
    let upcoming: Vec<Value> = run
        .board
        .iter()
        .enumerate()
        .skip(next.saturating_sub(1))
        .take(12)
        .map(
            |(k, s)| json!({ "pick": k + 1, "round": s.1, "team": team_brief(w, TeamRef::Nfl(s.0)), "mine": s.0 == t }),
        )
        .collect();
    let staff_pick = live::staff_choice(w, l).and_then(|id| sc::place(w, id)).map(|i| row(w, i));
    let offers: Vec<Value> = l.offers.iter().enumerate().map(|(k, o)| offer_json(w, l, o, k)).collect();
    // The room's voices for the top three still there.
    let voices: Vec<Value> = order
        .iter()
        .filter(|id| left.contains(id) && !b.is_dnd(**id))
        .take(3)
        .enumerate()
        .filter_map(|(k, &id)| {
            // Bug 18: the director, the head coach and the analytics head
            // each speak from their own read of him.
            let i = sc::place(w, id)?;
            let p = &w.people[i];
            let (rds, looks, f) = readers::read(w, readers::Staff::Club(t), id)?;
            let job = ["Director of college scouting", "Head coach", "analytics"][k.min(2)];
            let r = rds.iter().position(|x| x.job.contains(job))?;
            let says = afm_engine::world::scouting::own::voice(w, i, &f, &rds, &looks, r, speak(p))?;
            Some(json!({ "who": rds[r].name, "says": says, "id": id }))
        })
        .collect();
    let revealed = l.revealed.and_then(|(pk, id)| {
        let i = sc::place(w, id)?;
        let mut r = row(w, i);
        r["pick"] = json!(pk);
        // What the analysts say about it.
        let ours = order.iter().position(|&x| x == id).unwrap_or(pk as usize);
        let pub_rank = public.get(&id).copied().unwrap_or(ours) + 1;
        let reach = pub_rank as i64 - pk as i64 > (pk as i64 / 3).max(10);
        let good = (pk as i64 - pub_rank as i64) > 5;
        let desk = w.scouting.desks.iter().find(|d| d.active(run.year) && d.kind == public::DeskKind::Analyst);
        r["take"] = json!(desk.map(
            |d| json!({ "who": d.name, "outlet": d.outlet, "says": lines::pick_take(good, reach, d.key, id as u64) })
        ));
        Some(r)
    });
    Ok(json!({
        "live": true,
        "year": run.year,
        "team": team_brief(w, TeamRef::Nfl(t)),
        "next": next,
        "round": round,
        "on_clock": on_clock,
        "rush": l.rush,
        "clock_minutes": clock,
        "on_the_clock": run.board.get(next.saturating_sub(1)).map(|s| team_brief(w, TeamRef::Nfl(s.0))),
        "board": board_rows,
        "tiers": board::TIERS,
        "tier_left": tier_left,
        "feed": feed,
        "my_slots": my_slots,
        "upcoming": upcoming,
        "staff_pick": staff_pick,
        "offers": offers,
        "voices": voices,
        "revealed": revealed,
        "picks_made": run.picked.len(),
        "slots": run.board.len(),
        "pfa": b.pfa.len(),
        "pool": undrafted_pool(w),
        "final_say": w.career.as_ref().map(|_| "You"),
    }))
}

/// Your staff's guess at the last pick a player lasts to: the first club
/// picking after you with him in its top three of what's left.
fn last_shot(_w: &World, l: &live::LiveDraft, i: usize) -> Option<usize> {
    let run = &l.run;
    let start = run.next as usize;
    let mut tops: Vec<(f64, usize)> = Vec::new();
    for (k, s) in run.board.iter().enumerate().skip(start).take(40) {
        if s.0 == l.team {
            continue;
        }
        let b = run.boards.get(s.0)?;
        let g = *b.grade.get(&i)?;
        tops.clear();
        for &(j, _) in run.pool.iter().take(60) {
            if let Some(&gj) = b.grade.get(&j) {
                tops.push((gj, j));
            }
        }
        tops.sort_by(|a, b| b.0.total_cmp(&a.0));
        if tops.iter().take(3).any(|x| x.1 == i) || tops.get(2).is_some_and(|x| g >= x.0) {
            return Some(k + 1);
        }
    }
    None
}

pub fn draft_record(w: &World) -> Value {
    let Some(r) = &w.scouting.last_draft else { return Value::Null };
    let mine: Vec<Value> = r
        .picks
        .iter()
        .filter(|p| p.2 == r.team)
        .map(|p| json!({ "pick": p.0, "round": p.1, "id": p.3, "name": p.4, "pos": p.5, "school": p.6, "why": p.7 }))
        .collect();
    let all: Vec<Value> = r
        .picks
        .iter()
        .map(|p| json!({ "pick": p.0, "round": p.1, "team": team_brief(w, TeamRef::Nfl(p.2)), "id": p.3, "name": p.4, "pos": p.5, "school": p.6, "mine": p.2 == r.team }))
        .collect();
    let rush: Vec<Value> = r
        .rush
        .iter()
        .map(|x| json!({ "id": x.0, "name": x.1, "team": team_brief(w, TeamRef::Nfl(x.2)), "bonus": x.3.round(), "called": x.4, "mine": x.2 == r.team }))
        .collect();
    let minicamp: Vec<Value> = w
        .scouting
        .pro
        .minicamp
        .iter()
        .filter_map(|(id, clubs, signed)| {
            let i = sc::place(w, *id)?;
            Some(json!({
                "id": id, "name": w.people[i].name, "pos": w.people[i].player.position.name(),
                "clubs": clubs.iter().map(|&c| team_name(w, TeamRef::Nfl(c))).collect::<Vec<_>>(),
                "signed": signed.map(|c| team_name(w, TeamRef::Nfl(c))),
            }))
        })
        .collect();
    // Your class's grades from the analysts (one letter a desk).
    let grade = lines::class_grade(
        mine.iter()
            .filter_map(|m| {
                let pk = m["pick"].as_u64()? as f64;
                let rank = w
                    .scouting
                    .records
                    .drafts
                    .iter()
                    .find(|g| g.year == r.year && Some(g.id) == m["id"].as_u64().map(|x| x as u32))?
                    .public? as f64;
                Some(((pk - rank) / pk.max(8.0)).clamp(-2.0, 2.0))
            })
            .sum::<f64>(),
    );
    json!({ "year": r.year, "team": team_brief(w, TeamRef::Nfl(r.team)), "mine": mine, "all": all, "trades": r.trades, "rush": rush, "minicamp": minicamp, "grade": grade })
}

// --- The pre-draft season (scouting topic 10) --------------------------------------------

pub fn predraft_view(w: &World) -> Reply {
    let t = my_club(w)?;
    let pd = &w.scouting.predraft;
    let choices: Vec<Value> = {
        let mut v: Vec<(&u32, &u8)> = pd.choices.iter().collect();
        v.sort();
        v.into_iter()
            .filter_map(|(&id, &c)| {
                let i = sc::place(w, id)?;
                let mut r = row(w, i);
                r["choices"] = json!(predraft::choice_words(c));
                Some(r)
            })
            .collect()
    };
    let leaks: Vec<Value> = pd
        .leaks
        .iter()
        .rev()
        .take(60)
        .filter_map(|l| {
            let i = sc::place(w, l.player)?;
            Some(json!({
                "date": date_text(l.date),
                "team": team_brief(w, TeamRef::Nfl(l.team)),
                "player": row(w, i),
                // Your staff's read on whether it's real (the GM's habits).
                "read": if l.yours { "Yours" } else if l.real { "Likely real" } else { "Smells like a smokescreen" },
                "yours": l.yours,
            }))
        })
        .collect();
    let refuses: Vec<Value> = pd
        .refuses
        .iter()
        .filter_map(|&(id, club)| {
            let i = sc::place(w, id)?;
            Some(json!({ "player": row(w, i), "team": team_brief(w, TeamRef::Nfl(club)) }))
        })
        .collect();
    let lists: Vec<Value> = (0..4u8)
        .map(|k| {
            let ids: Vec<Value> = pd
                .lists
                .iter()
                .filter(|x| x.1 == k)
                .filter_map(|x| {
                    let i = sc::place(w, x.0)?;
                    let mut r = row(w, i);
                    r["declined"] = json!(k == predraft::LIST_VISIT && predraft::declines(w, i, t));
                    Some(r)
                })
                .collect();
            json!({ "k": k, "name": predraft::LIST_NAMES[k as usize], "cap": predraft::list_cap(k, w.year), "rows": ids })
        })
        .collect();
    let dates = &w.dates;
    // This draft's January: the deadline and the Senior Bowl after last
    // season when the draft is still ahead this spring.
    let jan = (sc::public::draft_year(w) == w.year).then(|| afm_engine::calendar::YearDates::new(w.year - 1));
    let jan = jan.as_ref().unwrap_or(dates);
    let cal = json!([
        ["Declaration deadline", date_text(jan.declare_deadline)],
        ["Senior Bowl", date_text(jan.senior_bowl)],
        ["Combine", date_text(dates.combine_start)],
        ["Pro days", "March to mid-April"],
        ["Top-30 visits deadline", date_text(dates.draft[0].date.add(-10))],
        ["The draft", date_text(dates.draft[0].date)],
    ]);
    Ok(
        json!({ "choices": choices, "leaks": leaks, "refuses": refuses, "lists": lists, "calendar": cal, "board": draft_board(w)? }),
    )
}

// --- Pro scouting (scouting topic 13) ----------------------------------------------------

pub fn pro_scouting(w: &World, args: &Value) -> Reply {
    let t = my_club(w)?;
    let staff = readers::Staff::Club(t);
    let ps = &w.scouting.pro;
    let scouts: Vec<Value> = ps
        .scouts
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let clubs: Vec<Value> =
                ps.clubs.iter().find(|c| c.0 as usize == k).map_or(Vec::new(), |c| c.1.iter().map(|&x| team_brief(w, TeamRef::Nfl(x))).collect());
            json!({ "code": readers::PRO_SCOUT + k as u32, "name": s.name, "job": if k == 0 { "Director of pro personnel" } else { "Pro scout" }, "clubs": clubs })
        })
        .collect();
    // One club's roster as your pro staff reads it.
    let club = args.get("club").and_then(Value::as_u64).map(|c| c as usize).filter(|&c| c < w.clubs.len() && c != t);
    let roster: Vec<Value> = club.map_or(Vec::new(), |c| {
        w.clubs[c]
            .roster
            .iter()
            .map(|&i| {
                let mut r = row(w, i);
                r["read"] = brief_read(w, staff, w.people[i].player.id);
                r["contract"] = json!(
                    w.people[i]
                        .pro
                        .as_ref()
                        .and_then(|c| c.contract.as_ref())
                        .map(|k| json!({ "apy": k.apy().round(), "through": k.last_year() }))
                );
                r
            })
            .collect()
    });
    // The street: unsigned pros worth a call, your pro staff's reads
    // first, then the ones nobody has looked at yet by what the league paid
    // them (public: their contracts).
    let mut street: Vec<(bool, f64, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| c.team.is_none()) && p.player.age <= 33
        })
        .map(|(i, p)| match readers::read(w, staff, p.player.id) {
            Some((_, _, f)) => (true, f.now.0, i),
            None => (false, p.pro.as_ref().map_or(0.0, |c| c.earnings), i),
        })
        .collect();
    street.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.total_cmp(&a.1)));
    let street: Vec<Value> = street
        .into_iter()
        .map(|x| x.2)
        .take(60)
        .map(|i| {
            let mut r = row(w, i);
            r["read"] = brief_read(w, staff, w.people[i].player.id);
            r["tried"] = json!(ps.tryouts.iter().any(|x| x.0 == w.people[i].player.id));
            r
        })
        .collect();
    let tryouts: Vec<Value> = ps
        .tryouts
        .iter()
        .rev()
        .take(30)
        .filter_map(|&(id, d)| {
            let i = sc::place(w, id)?;
            let mut r = row(w, i);
            r["date"] = json!(date_text(d));
            Some(r)
        })
        .collect();
    // The supplemental draft (through 2019).
    let supp_open = w.year <= 2019;
    let supp: Vec<Value> = if supp_open {
        sc::supp::eligible(w)
            .into_iter()
            .map(|i| {
                let mut r = row(w, i);
                let id = w.people[i].player.id;
                r["read"] = brief_read(w, staff, id);
                r["bid"] = json!(ps.supp_bids.iter().find(|b| b.0 == id).map(|b| b.1));
                r
            })
            .collect()
    } else {
        Vec::new()
    };
    let past: Vec<Value> = w
        .scouting
        .supplemental
        .iter()
        .rev()
        .take(20)
        .map(|s| json!({ "year": s.year, "id": s.player, "name": s.name, "team": s.team.map(|t| team_brief(w, TeamRef::Nfl(t))), "round": s.round }))
        .collect();
    let clubs: Vec<Value> = afm_engine::world::franchise::active_teams_in(w, w.year)
        .into_iter()
        .filter(|&c| c != t)
        .map(|c| team_brief(w, TeamRef::Nfl(c)))
        .collect();
    Ok(json!({
        "scouts": scouts,
        "clubs": clubs,
        "club": club.map(|c| team_brief(w, TeamRef::Nfl(c))),
        "roster": roster,
        "street": street,
        "tryouts": tryouts,
        "supp_open": supp_open,
        "supp_date": supp_open.then(|| date_text(sc::supp::date(w.year))),
        "supp": supp,
        "supp_past": past,
    }))
}

// --- College: recruit and portal scouting (scouting topics 15, 17) -----------------------

pub fn recruit_scouting(w: &World, args: &Value) -> Reply {
    let prog = w
        .seat
        .filter(|s| s.level == CalLevel::College)
        .and_then(|s| s.program)
        .ok_or("Only a college staff scouts recruits.")?;
    let staff = readers::Staff::Program(prog);
    let what = args.get("list").and_then(Value::as_str).unwrap_or("board");
    let season = w.dates.season;
    let rows: Vec<Value> = match what {
        "portal" => w
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| p.cal.portal.is_some() && p.stage == Stage::College && p.college != Some(prog))
            .map(|(i, _)| i)
            .collect::<Vec<_>>(),
        "juco" => w
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.juco
                    && p.stage == Stage::College
                    && p.college.is_some_and(|c| w.programs[c].division == afm_engine::world::data::Division::JUCO)
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>(),
        _ => dept::watch(w, prog),
    }
    .into_iter()
    .take(300)
    .map(|i| {
        let p = &w.people[i];
        let mut r = row(w, i);
        let id = p.player.id;
        r["read"] = brief_read(w, staff, id);
        r["offered"] = json!(p.cal.offers.contains(&prog));
        r["committed"] = json!(p.cal.commit.is_some_and(|c| c.0 == prog));
        r["visits"] = json!(p.cal.visits.iter().filter(|v| v.0 == prog).count());
        r["offers"] = json!(p.cal.offers.len());
        r["class_of"] = json!(p.class_of);
        r["followed"] = json!(is_followed(w, id));
        r["state"] = json!(w.states.get(p.home_state).map(|s| s.code.to_string()));
        r
    })
    .collect();
    let d = &w.scouting.dept;
    let (fall, spring) = dept::eval_limits(season);
    Ok(json!({
        "list": what,
        "rows": rows,
        "eval": { "fall": [d.eval_days.1, fall], "spring": [d.eval_days.2, spring] },
        "period": format!("{:?}", afm_engine::calendar::recruiting_period(&w.dates, w.today)),
    }))
}

// --- The high school coach's side (scouting topic 16) ------------------------------------

pub fn hs_side(w: &World) -> Reply {
    let h = hs::school(w).ok_or("Only a high school coach has this page.")?;
    let staff = readers::Staff::School(h);
    let players: Vec<Value> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.hs == h && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool))
        .map(|(i, p)| {
            let mut r = row(w, i);
            let id = p.player.id;
            r["read"] = brief_read(w, staff, id);
            r["offers"] =
                json!(p.cal.offers.iter().map(|&o| json!({ "id": o, "name": w.program_name(o) })).collect::<Vec<_>>());
            r["visits"] = json!(
                p.cal
                    .visits
                    .iter()
                    .map(|v| json!({ "id": v.0, "name": w.program_name(v.0), "date": date_text(v.1) }))
                    .collect::<Vec<_>>()
            );
            r["committed"] = json!(p.cal.commit.map(|c| w.program_name(c.0).to_string()));
            r["signed"] = json!(p.cal.signed.as_ref().map(|s| w.program_name(s.program).to_string()));
            r["promoted"] = json!(w.scouting.hs.promoted.iter().filter(|x| x.id == id).count());
            r["class_of"] = json!(p.class_of);
            r
        })
        .collect();
    // The college staffs that came to you: who offered or visited your
    // players, with your credit at each.
    let mut staffs: HashMap<usize, u32> = HashMap::new();
    for p in w.people.iter().filter(|p| p.hs == h && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)) {
        for &o in &p.cal.offers {
            *staffs.entry(o).or_default() += 1;
        }
    }
    for &c in w.scouting.hs.credibility.keys() {
        staffs.entry(c).or_default();
    }
    let mut staffs: Vec<(usize, u32)> = staffs.into_iter().collect();
    staffs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let colleges: Vec<Value> = staffs
        .into_iter()
        .take(40)
        .map(|(c, n)| {
            let cred = hs::credibility(w, c);
            json!({
                "team": team_brief(w, TeamRef::College(c)),
                "offers": n,
                "credit": cred,
                "credit_words": if cred >= 0.7 { "Takes your word" } else if cred >= 0.45 { "Listens" } else if cred >= 0.25 { "Checks for itself" } else { "Doesn't trust your word" },
            })
        })
        .collect();
    let calls: Vec<Value> = w
        .scouting
        .hs
        .promoted
        .iter()
        .rev()
        .take(40)
        .filter_map(|x| {
            let i = sc::place(w, x.id)?;
            Some(json!({
                "date": date_text(x.date),
                "player": row(w, i),
                "to": x.program.map_or("Every staff in your area".to_string(), |c| w.program_name(c).to_string()),
                "said": hs::STRENGTHS[x.strength as usize],
                "settled": x.settled.map(|ok| if ok { "He proved you right" } else { "It cost you" }),
            }))
        })
        .collect();
    Ok(json!({ "players": players, "colleges": colleges, "calls": calls, "strengths": hs::STRENGTHS }))
}

// --- Look-backs (scouting topic 9) -------------------------------------------------------

pub fn lookback_view(w: &World) -> Value {
    let r = &w.scouting.records;
    let rows = |v: &[lookback::Graded]| -> Vec<Value> {
        v.iter()
            .rev()
            .take(300)
            .map(|g| {
                let v = lookback::verdict(g);
                json!({
                    "year": g.year, "id": g.id, "name": g.name, "pos": g.pos.name(), "ours": g.ours, "public": g.public,
                    "pick": g.pick, "team": g.team, "looks": g.looks, "outcome": g.outcome,
                    "verdict": v.words(), "v": v.key(),
                })
            })
            .collect()
    };
    let sum = |v: &[lookback::Graded]| {
        let mut c = [0u32; 4];
        for g in v {
            match lookback::verdict(g) {
                lookback::Verdict::Held => c[0] += 1,
                lookback::Verdict::TooHigh => c[1] += 1,
                lookback::Verdict::TooLow => c[2] += 1,
                lookback::Verdict::NoFairRead => c[3] += 1,
                _ => {}
            }
        }
        json!({ "held": c[0], "high": c[1], "low": c[2], "unfair": c[3] })
    };
    json!({ "drafts": rows(&r.drafts), "classes": rows(&r.classes), "draft_sum": sum(&r.drafts), "class_sum": sum(&r.classes) })
}

#[allow(dead_code)]
fn _uses(_: Date, _: Position) {}
