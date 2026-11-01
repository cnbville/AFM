//! Lane commands: stories (docs/final-build/plan.md). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! Lane 8 of the final build is the stories around the football on your
//! side (the world side lives in `afm_engine::world::stories`):
//! - your office (C6): the shelves the years fill, moving with you;
//! - your career story (A14 item 2): a profile's timeline, every season and
//!   title, your own nicknames;
//! - the media desk: the morning-after grades (P5), the schedule release and
//!   games abroad (P6, P7), the preseason annuals (C7), the video game's
//!   ratings (C10), fantasy (C11), jersey sales (C12), the documentary crews
//!   and the pregame show (C3, C9), and the nicknames (A14 items 4-20);
//! - the traditions: rivalry trophies (C1), uniforms picked each week (C4),
//!   retired numbers, rings of honor and statues (C5), stormings and their
//!   fines (C2), parades and rings (C13, C14), farewells and one-day
//!   contracts (C15, C16) and signature celebrations (C17);
//! - a "Story" tab on the player page, and naming your own players.
//!
//! Two asks need your answer (a one-day contract, a documentary): each
//! comes as a letter with its answers (`after_days`), and the media desk
//! answers them too.

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::desk::Head;
use afm_engine::world::stories::nick::{self, NickKind, Nickname, Who};
use afm_engine::world::stories::story::{EntryKind, MementoKind};
use afm_engine::world::stories::traditions::{self, HonorKind};
use afm_engine::world::stories::{self as st, TeamKey, media, nfl, numbers};
use serde_json::{Value, json};

use crate::lane_core::links::{self, Linker};
use crate::lane_core::mail;
use crate::views::{self, TeamRef, date_text};
use crate::{Game, Reply};

const COMMANDS: [&str; 9] = [
    "stories_office",
    "stories_story",
    "stories_media",
    "stories_traditions",
    "stories_answer",
    "stories_uniform",
    "stories_player",
    "stories_nickname",
    "stories_suggest",
];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_mut() else { return Some(Err("No world is loaded.".into())) };
    // Answers given on the letters count at once.
    st::desk_answers(w);
    let tab = args.get("tab").and_then(Value::as_str).unwrap_or("");
    Some(match cmd {
        "stories_office" => Ok(office(w)),
        "stories_story" => Ok(story(w)),
        "stories_media" => Ok(media_desk(w, tab, args)),
        "stories_traditions" => Ok(traditions_page(w, tab)),
        "stories_answer" => answer(w, args),
        "stories_uniform" => (|| {
            let t = st::seat_team(w).ok_or("You need a team first.")?;
            let n = traditions::sets(w, t).len();
            w.stories.traditions.uniforms.next = match args.get("set").and_then(Value::as_i64) {
                Some(k) if k >= 0 && (k as usize) < n => Some(k as u8),
                Some(k) if k >= 0 => return Err("No such set.".into()),
                _ => None,
            };
            Ok(traditions_page(w, "Uniforms"))
        })(),
        "stories_player" => num(args, "id").and_then(|id| player(w, id as u32)),
        "stories_nickname" => (|| {
            let id = num(args, "id")? as u32;
            let name = args.get("name").and_then(Value::as_str).ok_or("Missing name.")?;
            let text = nick::give(w, id, name)?;
            let mut v = player(w, id)?;
            v["said"] = json!(link_one(w, &text));
            Ok(v)
        })(),
        "stories_suggest" => num(args, "id").map(|id| json!(nick::suggestions(w, id as u32))),
        _ => unreachable!(),
    })
}

fn num(args: &Value, k: &str) -> Result<u64, String> {
    args.get(k)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .ok_or_else(|| format!("Missing {k}."))
}

// --- Small helpers -------------------------------------------------------------------------------

fn tref(t: TeamKey) -> TeamRef {
    match t {
        TeamKey::Hs(h) => TeamRef::HighSchool(h as usize),
        TeamKey::College(p) => TeamRef::College(p as usize),
        TeamKey::Nfl(n) => TeamRef::Nfl(n as usize),
        TeamKey::Other(f) => TeamRef::Other(f as usize),
    }
}

fn brief(w: &World, t: TeamKey) -> Value {
    views::team_brief(w, tref(t))
}

fn seat_kind(w: &World) -> &'static str {
    match st::seat_team(w) {
        Some(TeamKey::Nfl(_)) => "nfl",
        Some(TeamKey::College(_)) => "college",
        Some(TeamKey::Hs(_)) => "hs",
        Some(TeamKey::Other(_)) => "other",
        None => "",
    }
}

/// A player as a link with his position, or his plain id's words when gone.
fn who(w: &World, id: u32) -> Value {
    match views::person_index(w, id) {
        Some(i) => {
            let p = &w.people[i];
            let team = numbers::team_of(w, i);
            json!({
                "id": id,
                "link": links::player(id, &p.name),
                "name": p.name,
                "pos": p.player.position.name(),
                "team": team.map(|t| brief(w, t)),
                "mine": team.is_some() && team == st::seat_team(w),
                "nick": nick::current(&w.stories, Who::Player(id)).map(|n| n.name.clone()),
            })
        }
        None => json!({ "id": id, "link": "A former player", "name": "A former player", "pos": "" }),
    }
}

/// Lines with their teams and people linked, read once.
fn link_all(w: &World, texts: &[String], ctx: &[TeamRef]) -> Vec<String> {
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let l = Linker::new(w, &refs, true);
    texts.iter().map(|t| l.link(t, ctx)).collect()
}

fn link_one(w: &World, text: &str) -> String {
    link_all(w, &[text.to_string()], &views::mine(w).into_iter().collect::<Vec<_>>()).pop().unwrap_or_default()
}

/// Many lines linked with one reading of the world's names (building the
/// name index is the slow part, so a list never builds it per line).
fn link_many<'a>(w: &World, texts: impl IntoIterator<Item = &'a String>) -> std::vec::IntoIter<String> {
    let texts: Vec<String> = texts.into_iter().cloned().collect();
    link_all(w, &texts, &views::mine(w).into_iter().collect::<Vec<_>>()).into_iter()
}

fn stage_words(n: &Nickname) -> &'static str {
    if n.retired {
        return if n.replaced.is_some() { "Replaced" } else { "Faded" };
    }
    match n.stage {
        0 => "Just coined",
        1 => "Local",
        2 => "National",
        _ => "The booth uses it",
    }
}

fn kind_words(k: NickKind) -> &'static str {
    match k {
        NickKind::Earned => "Earned",
        NickKind::Hype => "Hype",
        NickKind::Family => "From home",
        NickKind::Mocking => "Mocking",
        NickKind::Heir => "Heir",
        NickKind::Given => "Yours",
    }
}

fn nick_json(w: &World, n: &Nickname) -> Value {
    let whom = match n.who {
        Who::Player(id) => who(w, id),
        Who::Coach(c) => {
            let name = w.coaches.get(c as usize).map_or("A coach", |x| x.name.as_str());
            json!({ "id": c, "link": links::coach(c as usize, name), "name": name, "coach": true })
        }
    };
    json!({
        "who": whom,
        "name": n.name,
        "kind": kind_words(n.kind),
        "coined": date_text(n.coined),
        "year": n.coined.year(),
        "by": n.by.name(),
        "outlet": n.outlet,
        "moment": n.moment,
        "team": n.team.map(|t| brief(w, t)),
        "stage": stage_words(n),
        "spread": n.stage,
        "stick": (n.stick * 100.0).round(),
        "reaction": n.reaction.name(),
        "merch": n.merch,
        "retired": n.retired,
        "replaced": n.replaced,
        "heir": n.heir_of.map(|id| who(w, id)),
    })
}

/// Your players: everyone on your team now.
fn my_players(w: &World) -> Vec<usize> {
    let Some(me) = st::seat_team(w) else { return Vec::new() };
    (0..w.people.len()).filter(|&i| numbers::team_of(w, i) == Some(me)).collect()
}

fn clock_text(sec: u16) -> String {
    format!("{}:{:02}", sec / 60, sec % 60)
}

fn spot_text(yardline: u8) -> String {
    match yardline {
        50 => "midfield".into(),
        y if y < 50 => format!("the opponent's {y}"),
        y => format!("your own {}", 100 - y),
    }
}

// --- C6: the office -------------------------------------------------------------------------------

fn office(w: &World) -> Value {
    let Some(c) = w.career.as_ref() else { return json!({ "none": true }) };
    let s = &w.stories.story;
    let texts: Vec<String> = s.office.iter().map(|m| m.text.clone()).collect();
    let linked = link_all(w, &texts, &[]);
    let order = [
        MementoKind::Ring,
        MementoKind::Trophy,
        MementoKind::GameBall,
        MementoKind::Cover,
        MementoKind::Plaque,
        MementoKind::Photo,
        MementoKind::Gift,
    ];
    let shelves: Vec<Value> = order
        .iter()
        .map(|&k| {
            let mut items: Vec<Value> = s
                .office
                .iter()
                .zip(&linked)
                .filter(|(m, _)| m.kind == k)
                .map(|(m, t)| {
                    json!({ "date": date_text(m.date), "year": m.date.year(), "text": t, "team": m.team.map(|t| brief(w, t)) })
                })
                .collect();
            items.reverse();
            json!({ "kind": k.word(), "key": format!("{k:?}").to_lowercase(), "items": items })
        })
        .collect();
    // Where the office has been: each team's years.
    let mut stints: Vec<(TeamKey, i16, i16)> = Vec::new();
    for x in &s.seasons {
        match stints.last_mut() {
            Some(l) if l.0 == x.team => l.2 = x.year,
            _ => stints.push((x.team, x.year, x.year)),
        }
    }
    let moved: Vec<_> = s.entries.iter().filter(|e| matches!(e.kind, EntryKind::Hired | EntryKind::Left)).collect();
    let mut texts = link_many(w, moved.iter().map(|e| &e.text));
    let moves: Vec<Value> = moved
        .iter()
        .map(|e| json!({ "date": date_text(e.date), "kind": format!("{:?}", e.kind), "text": texts.next(), "team": e.team.map(|t| brief(w, t)) }))
        .collect();
    let here = st::seat_team(w);
    // The job you hold now runs on from the year you were hired.
    if let Some(h) = here
        && stints.last().is_none_or(|l| l.0 != h)
    {
        let hired = s.entries.iter().rev().find(|e| e.kind == EntryKind::Hired && e.team == Some(h));
        let from = hired.map_or(w.today.year(), |e| e.date.year()) as i16;
        stints.push((h, from, from));
    }
    json!({
        "you": c.you.name,
        "team": here.map(|t| brief(w, t)),
        "level": seat_kind(w),
        "since": s.office.first().map(|m| date_text(m.date)),
        "count": s.office.len(),
        "shelves": shelves,
        "stints": stints.iter().map(|&(t, a, b)| json!({ "team": brief(w, t), "from": a, "to": b, "now": Some(t) == here && stints.last().is_some_and(|l| l.0 == t) })).collect::<Vec<_>>(),
        "moves": moves,
        "titles": s.titles.iter().rev().map(|(y, t, what)| json!({ "year": y, "team": brief(w, *t), "what": what })).collect::<Vec<_>>(),
        "wins": s.wins,
    })
}

// --- A14 item 2: your career story ---------------------------------------------------------------

fn entry_kind(k: EntryKind) -> &'static str {
    match k {
        EntryKind::Hired => "Hired",
        EntryKind::FirstWin => "First win",
        EntryKind::Milestone => "Milestone",
        EntryKind::Title => "Title",
        EntryKind::Upset => "Upset",
        EntryKind::Rivalry => "Rivalry",
        EntryKind::Left => "Moved on",
        EntryKind::Draftee => "Drafted",
        EntryKind::Honor => "Honor",
        EntryKind::Nickname => "Nickname",
        EntryKind::LosingSeason => "Hard year",
        EntryKind::Season => "Season",
        EntryKind::BigWin => "Big win",
        EntryKind::Chapter => "Chapter",
    }
}

fn story(w: &World) -> Value {
    let Some(c) = w.career.as_ref() else { return json!({ "none": true }) };
    let s = &w.stories.story;
    let texts: Vec<String> = s.entries.iter().map(|e| e.text.clone()).collect();
    let linked = link_all(w, &texts, &[]);
    let entries: Vec<Value> = s
        .entries
        .iter()
        .zip(linked)
        .map(|(e, t)| {
            json!({
                "date": date_text(e.date),
                "iso": e.date.0,
                "year": e.date.year(),
                "kind": entry_kind(e.kind),
                "text": t,
                "team": e.team.map(|t| brief(w, t)),
            })
        })
        .collect();
    let (mut wins, mut losses, mut ties) = (0u32, 0u32, 0u32);
    let seasons: Vec<Value> = s
        .seasons
        .iter()
        .rev()
        .map(|x| {
            wins += x.wins as u32;
            losses += x.losses as u32;
            ties += x.ties as u32;
            let games = (x.wins + x.losses + x.ties).max(1) as f64;
            json!({
                "year": x.year,
                "team": brief(w, x.team),
                "level": match x.team { TeamKey::Nfl(_) => "NFL", TeamKey::College(_) => "College", TeamKey::Hs(_) => "High school", TeamKey::Other(f) => afm_engine::world::pro::other::league_short(afm_engine::world::pro::other::league_of(f as usize)) },
                "w": x.wins, "l": x.losses, "t": x.ties,
                "pct": ((x.wins as f64 + 0.5 * x.ties as f64) / games * 1000.0).round() / 1000.0,
                "title": s.titles.iter().find(|t| t.0 == x.year as i32 && t.1 == x.team).map(|t| t.2.clone()),
            })
        })
        .collect();
    let drafted = s.entries.iter().filter(|e| e.kind == EntryKind::Draftee).count();
    let teams = {
        let mut v: Vec<TeamKey> = Vec::new();
        for t in s.seasons.iter().map(|x| x.team).chain(st::seat_team(w)) {
            if !v.contains(&t) {
                v.push(t);
            }
        }
        v
    };
    let names: Vec<Value> =
        nick::all_of(&w.stories, Who::Coach(c.coach as u32)).into_iter().map(|n| nick_json(w, n)).collect();
    // Your fourth downs as the bots read them.
    let reports = &w.stories.nfl.reports;
    let calls: Vec<&nfl::Call> = reports.iter().flat_map(|r| &r.calls).collect();
    let agreed = calls.iter().filter(|x| x.chose == x.bot).count();
    // A fold from 0.0: an empty f32 sum is -0.0, which a page prints as "-0".
    let given = calls.iter().fold(0.0f32, |a, x| a + x.cost);
    let start = w.stories.story.entries.first().map(|e| e.date).unwrap_or(c.started);
    json!({
        "you": c.you.name,
        "coach": c.coach,
        "nick": w.coach_nickname(c.coach).map(str::to_string),
        "team": st::seat_team(w).map(|t| brief(w, t)),
        "since": date_text(start),
        "years": (w.today.year() - start.year()).max(0),
        "record": { "w": wins, "l": losses, "t": ties },
        "wins": s.wins,
        "titles": s.titles.iter().rev().map(|(y, t, what)| json!({ "year": y, "team": brief(w, *t), "what": what })).collect::<Vec<_>>(),
        "teams": teams.iter().map(|&t| brief(w, t)).collect::<Vec<_>>(),
        "players": s.players.len(),
        "drafted": drafted,
        "seasons": seasons,
        "entries": entries,
        "names": names,
        "fourth": { "calls": calls.len(), "agreed": agreed, "given": (given * 10.0).round() / 10.0, "go_rate": w.stories.nfl.go_rate(w.year).map(|x| (x * 100.0).round()) },
        "office": w.stories.story.office.len(),
    })
}

// --- The media desk ------------------------------------------------------------------------------

fn media_tabs(w: &World) -> Vec<&'static str> {
    match seat_kind(w) {
        "nfl" => {
            vec!["Front page", "Grades", "Schedule", "Annuals", "Ratings", "Fantasy", "Jerseys", "Shows", "Nicknames"]
        }
        "college" => vec!["Front page", "Grades", "Annuals", "Ratings", "Jerseys", "Shows", "Nicknames"],
        "hs" => vec!["Front page", "Shows", "Nicknames"],
        _ => vec!["Annuals", "Ratings", "Fantasy", "Jerseys", "Shows", "Nicknames"],
    }
}

fn media_desk(w: &World, tab: &str, args: &Value) -> Value {
    let tabs = media_tabs(w);
    let tab = if tabs.contains(&tab) { tab } else { tabs[0] };
    let body = match tab {
        "Front page" => front_page(w),
        "Grades" => grades(w),
        "Schedule" => schedule(w),
        "Annuals" => annuals(w, args.get("year").and_then(Value::as_i64).map(|y| y as i32)),
        "Ratings" => ratings(w),
        "Fantasy" => fantasy(w),
        "Jerseys" => jerseys(w),
        "Shows" => shows(w),
        _ => nicknames(w),
    };
    json!({
        "tabs": tabs,
        "tab": tab,
        "level": seat_kind(w),
        "team": st::seat_team(w).map(|t| brief(w, t)),
        "date": date_text(w.today),
        "season": w.year,
        "asks": asks(w),
        "body": body,
    })
}

/// What waits for your answer: a one-day contract, a documentary.
fn asks(w: &World) -> Vec<Value> {
    let mut v = Vec::new();
    if let Some((id, club, asked)) = w.stories.careers.offer {
        let p = who(w, id);
        v.push(json!({
            "what": "one_day",
            "title": format!("{} wants to retire with you", p["name"].as_str().unwrap_or("A former star")),
            "text": format!(
                "{} asked the {} for a one-day contract, to retire where it started. With no answer, the front office signs him.",
                p["link"].as_str().unwrap_or(""),
                w.nfl_team_name(club as usize)
            ),
            "by": date_text(asked.add(9)),
            "yes": "Bring him home",
            "no": "Not this time",
        }));
    }
    if let Some((year, show)) = &w.stories.media.doc_offer
        && *year as i32 == w.year
    {
        let answered = w.stories.media.doc_answer;
        let boss = if seat_kind(w) == "nfl" { "the owner" } else { "your athletic director" };
        v.push(json!({
            "what": "doc",
            "title": format!("{show} wants your team"),
            "text": format!("Cameras in meetings, the locker room and your office from camp on. With no answer by May 31, {boss} decides."),
            "by": date_text(Date::ymd(*year as i32, 5, 31)),
            "yes": "Say yes",
            "no": "Say no",
            "answered": answered.map(|y| if y { "You said yes." } else { "You said no." }),
        }));
    }
    v
}

fn report_json(w: &World, r: &nfl::Report, lines: Vec<String>) -> Value {
    let calls: Vec<Value> = r
        .calls
        .iter()
        .map(|c| {
            json!({
                "q": c.quarter,
                "clock": clock_text(c.clock),
                "sit": format!("4th and {} at {}", c.togo, spot_text(c.yardline)),
                "lead": c.lead,
                "chose": c.chose.word(),
                "bot": c.bot.word(),
                "agree": c.chose == c.bot,
                "cost": (c.cost * 10.0).round() / 10.0,
                "edge": (c.edge * 10.0).round() / 10.0,
                "result": c.result,
            })
        })
        .collect();
    let won = r.score[0] > r.score[1];
    let tone = ["Harsh", "Fair", "Praise"][r.tone.min(2) as usize];
    json!({
        "date": date_text(r.date),
        "opp": brief(w, r.opp),
        "score": r.score,
        "won": won,
        "bot": if r.bot.is_empty() { "The columnists".to_string() } else { r.bot.clone() },
        "tone": tone,
        "timeouts": r.timeouts_left,
        "calls": calls,
        "lines": lines,
        "given": (r.calls.iter().map(|c| c.cost).sum::<f32>() * 10.0).round() / 10.0,
    })
}

fn grades(w: &World) -> Value {
    let reports = &w.stories.nfl.reports;
    let texts: Vec<String> = reports.iter().rev().take(20).flat_map(|r| r.lines.clone()).collect();
    let mut linked = link_all(w, &texts, &views::mine(w).into_iter().collect::<Vec<_>>()).into_iter();
    let list: Vec<Value> = reports
        .iter()
        .rev()
        .take(20)
        .map(|r| {
            let lines: Vec<String> = linked.by_ref().take(r.lines.len()).collect();
            report_json(w, r, lines)
        })
        .collect();
    let year = w.year;
    let calls: Vec<&nfl::Call> = reports
        .iter()
        .filter(|r| r.date.year() == year || r.date.year() == year + 1 && r.date.month() <= 2)
        .flat_map(|r| &r.calls)
        .collect();
    let level = st::seat_team(w).map(|t| t.level()).unwrap_or(afm_engine::calendar::Level::Nfl);
    json!({
        "reports": list,
        "bot": nfl::bot(level, year, 0),
        "season": {
            "calls": calls.len(),
            "agreed": calls.iter().filter(|c| c.chose == c.bot).count(),
            "go_said": calls.iter().filter(|c| c.bot == nfl::Choice::Go).count(),
            "went": calls.iter().filter(|c| c.bot == nfl::Choice::Go && c.chose == nfl::Choice::Go).count(),
            "given": (calls.iter().map(|c| c.cost).sum::<f32>() * 10.0).round() / 10.0,
            "go_rate": w.stories.nfl.go_rate(year).map(|x| (x * 100.0).round()),
        },
    })
}

fn front_page(w: &World) -> Value {
    let me = st::seat_team(w);
    let year = w.year;
    // The latest grades.
    let grade = w.stories.nfl.reports.last().map(|r| {
        let lines = link_all(w, &r.lines, &views::mine(w).into_iter().collect::<Vec<_>>());
        report_json(w, r, lines)
    });
    // This season's annuals on you.
    let annual: Vec<Value> = w
        .stories
        .media
        .magazines
        .iter()
        .filter(|m| m.year as i32 == year)
        .filter_map(|m| {
            let mine = me?;
            let code = match mine {
                TeamKey::Nfl(n) if m.nfl => n as u32,
                TeamKey::College(p) if !m.nfl => p,
                _ => return None,
            };
            let rank = m.top.iter().position(|&x| x == code).map(|k| k + 1);
            let mine_ids: Vec<Value> = m.all_conf.iter().chain(&m.covers).filter(|&&id| is_mine(w, id)).map(|&id| who(w, id)).collect();
            Some(json!({ "title": m.title, "rank": rank, "of": m.top.len(), "covers": m.covers.iter().map(|&id| who(w, id)).collect::<Vec<_>>(), "yours": mine_ids }))
        })
        .collect();
    // Your best by the video game, your jerseys, your names.
    let mine = my_players(w);
    let mut rated: Vec<(u8, u32)> =
        mine.iter().filter_map(|&i| media::game_rating(w, i).map(|r| (r, w.people[i].player.id))).collect();
    rated.sort_by_key(|b| std::cmp::Reverse(b.0));
    // The game your players are in: the NFL's or the college one.
    let pro = matches!(me, Some(TeamKey::Nfl(_)));
    let vg = media::games_out(w.today).into_iter().find(|g| g.1 == pro).map(|g| g.0);
    let jerseys: Vec<Value> = w
        .stories
        .media
        .jerseys
        .iter()
        .rev()
        .take(2)
        .flat_map(|(y, college, ids)| {
            ids.iter()
                .enumerate()
                .filter(|(_, id)| is_mine(w, **id))
                .map(|(k, &id)| json!({ "year": y, "list": if *college { "College jerseys" } else { "NFLPA top 50" }, "rank": k + 1, "who": who(w, id) }))
                .collect::<Vec<_>>()
        })
        .collect();
    let names: Vec<Value> = w
        .stories
        .nicknames
        .iter()
        .rev()
        .filter(|n| me.is_some() && n.team == me)
        .take(6)
        .map(|n| nick_json(w, n))
        .collect();
    let docs: Vec<Value> =
        w.stories.media.docs.iter().rev().filter(|d| Some(d.team) == me).take(3).map(|d| doc_json(w, d)).collect();
    let fantasy: Vec<Value> = if matches!(me, Some(TeamKey::Nfl(_))) {
        let mut f: Vec<(f32, u32)> = mine
            .iter()
            .filter_map(|&i| {
                w.stories.media.fantasy.get(&w.people[i].player.id).map(|x| (x.pts, w.people[i].player.id))
            })
            .collect();
        f.sort_by(|a, b| b.0.total_cmp(&a.0));
        f.iter()
            .take(5)
            .map(|&(pts, id)| {
                let rank = media::fantasy_line(w, id).map(|x| x.2);
                json!({ "who": who(w, id), "pts": (pts * 10.0).round() / 10.0, "rank": rank })
            })
            .collect()
    } else {
        Vec::new()
    };
    // A high school's front page: the rivalry and the names around the state
    // (no annuals, video game or jersey lists cover high schoolers).
    let (rivals, around) = match me {
        Some(TeamKey::Hs(h)) => {
            let state = w.high_schools.get(h as usize).map(|x| x.state);
            let around: Vec<Value> = w
                .stories
                .nicknames
                .iter()
                .rev()
                .filter(|n| !n.retired && n.team != me)
                .filter(|n| match n.team {
                    Some(TeamKey::Hs(x)) => w.high_schools.get(x as usize).map(|s| s.state) == state,
                    _ => false,
                })
                .take(6)
                .map(|n| nick_json(w, n))
                .collect();
            (rivalries(w)["rows"].clone(), around)
        }
        _ => (json!([]), Vec::new()),
    };
    json!({
        "level": seat_kind(w),
        "bots": me.is_some_and(|t| nfl::bot(t.level(), year, 0).is_some()),
        "rivals": rivals,
        "around": around,
        "grade": grade,
        "annuals": annual,
        "rated": rated.iter().take(6).map(|&(r, id)| json!({ "who": who(w, id), "rating": r })).collect::<Vec<_>>(),
        "vg": vg,
        "jerseys": jerseys,
        "names": names,
        "docs": docs,
        "fantasy": fantasy,
        "schedule": schedule_brief(w),
    })
}

fn is_mine(w: &World, id: u32) -> bool {
    let me = st::seat_team(w);
    me.is_some() && views::person_index(w, id).and_then(|i| numbers::team_of(w, i)) == me
}

fn schedule_brief(w: &World) -> Value {
    let Some(s) = &w.stories.nfl.schedule else { return Value::Null };
    if s.year != w.year {
        return Value::Null;
    }
    json!({ "prime": s.prime, "short": s.short, "abroad": s.abroad })
}

fn schedule(w: &World) -> Value {
    let me = st::seat_team(w);
    let rights = match me {
        Some(TeamKey::Nfl(t)) => nfl::marketing_rights(&w.nfl_team_name(t as usize), w.year),
        _ => Vec::new(),
    };
    let Some(s) = w.stories.nfl.schedule.as_ref().filter(|s| s.year == w.year) else {
        return json!({ "none": true, "rights": rights, "release": "The schedule comes out in May." });
    };
    let games: Vec<Value> = s
        .games
        .iter()
        .map(|(d, opp, window)| json!({ "date": date_text(*d), "iso": d.0, "opp": brief(w, TeamKey::Nfl(*opp)), "window": window, "prime": matches!(window.as_str(), "the season opener" | "Thursday night" | "Sunday night" | "Monday night" | "Thanksgiving" | "Black Friday" | "Christmas") }))
        .collect();
    json!({
        "year": s.year,
        "prime": s.prime,
        "short": s.short,
        "abroad": s.abroad,
        "games": games,
        "rights": rights,
    })
}

fn annuals(w: &World, year: Option<i32>) -> Value {
    let years: Vec<i32> = {
        let mut v: Vec<i32> = w.stories.media.magazines.iter().map(|m| m.year as i32).collect();
        v.sort_unstable();
        v.dedup();
        v.reverse();
        v
    };
    let year = year.filter(|y| years.contains(y)).or_else(|| years.first().copied());
    let me = st::seat_team(w);
    let mags: Vec<Value> = w
        .stories
        .media
        .magazines
        .iter()
        .filter(|m| Some(m.year as i32) == year)
        .map(|m| {
            let top: Vec<Value> = m
                .top
                .iter()
                .enumerate()
                .map(|(k, &x)| {
                    let t = if m.nfl { TeamKey::Nfl(x as u16) } else { TeamKey::College(x) };
                    json!({ "rank": k + 1, "team": brief(w, t), "mine": Some(t) == me })
                })
                .collect();
            json!({
                "title": m.title,
                "nfl": m.nfl,
                "top": top,
                "covers": m.covers.iter().map(|&id| who(w, id)).collect::<Vec<_>>(),
                "all_conf": m.all_conf.iter().map(|&id| who(w, id)).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "years": years, "year": year, "magazines": mags })
}

fn ratings(w: &World) -> Value {
    // The games out now, yours first.
    let pro = matches!(st::seat_team(w), Some(TeamKey::Nfl(_)));
    let mut titles: Vec<(String, bool)> = media::games_out(w.today);
    titles.sort_by_key(|g| g.1 != pro);
    let titles: Vec<String> = titles.into_iter().map(|g| g.0).collect();
    let mut mine: Vec<(u8, u32)> =
        my_players(w).iter().filter_map(|&i| media::game_rating(w, i).map(|r| (r, w.people[i].player.id))).collect();
    mine.sort_by_key(|b| std::cmp::Reverse(b.0));
    let club: Vec<Value> = w
        .stories
        .media
        .vg_top
        .iter()
        .rev()
        .take(40)
        .map(|(y, title, id, r)| json!({ "year": y, "title": title, "who": who(w, *id), "rating": r }))
        .collect();
    json!({
        "released": !w.stories.media.ratings.is_empty(),
        "titles": titles,
        "mine": mine.iter().map(|&(r, id)| json!({ "who": who(w, id), "rating": r })).collect::<Vec<_>>(),
        "club": club,
        "note": "The game's numbers are the public's read: production, honors and pedigree, with only part of what film shows.",
    })
}

fn fantasy(w: &World) -> Value {
    const POS: [&str; 4] = ["QB", "RB", "WR", "TE"];
    let f = &w.stories.media.fantasy;
    let mut by: [Vec<(f32, u8, u32)>; 4] = Default::default();
    for (&id, x) in f {
        if let Some(v) = by.get_mut(x.pos as usize) {
            v.push((x.pts, x.games, id));
        }
    }
    let groups: Vec<Value> = by
        .iter_mut()
        .enumerate()
        .map(|(k, v)| {
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            json!({
                "pos": POS[k],
                "rows": v.iter().take(24).enumerate().map(|(r, &(pts, g, id))| json!({
                    "rank": r + 1, "who": who(w, id), "pts": (pts * 10.0).round() / 10.0, "games": g,
                    "per": if g > 0 { (pts / g as f32 * 10.0).round() / 10.0 } else { 0.0 },
                    "last": (f.get(&id).map_or(0.0, |x| x.last) * 10.0).round() / 10.0,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let past: Vec<Value> = w
        .stories
        .media
        .fantasy_leaders
        .iter()
        .rev()
        .take(24)
        .map(|&(y, pos, id, pts)| json!({ "year": y, "pos": POS.get(pos as usize).copied().unwrap_or(""), "who": who(w, id), "pts": (pts * 10.0).round() / 10.0 }))
        .collect();
    json!({ "year": w.stories.media.fantasy_year, "groups": groups, "past": past, "scoring": if w.year >= 2019 { "Points per reception (a point a catch), the big sites' default since ESPN switched in 2019." } else { "Standard scoring (no points for catches), the default of the time." } })
}

fn jerseys(w: &World) -> Value {
    let lists: Vec<Value> = w
        .stories
        .media
        .jerseys
        .iter()
        .rev()
        .map(|(y, college, ids)| {
            json!({
                "year": y,
                "college": college,
                "title": if *college { format!("College jersey sales, {y}") } else { format!("NFLPA top-selling players, {y}") },
                "rows": ids.iter().enumerate().map(|(k, &id)| {
                    // A college list names the school he sold for, wherever he is now.
                    let school = college
                        .then(|| views::person_index(w, id).and_then(|i| w.people[i].college))
                        .flatten()
                        .map(|p| brief(w, TeamKey::College(p as u32)));
                    json!({ "rank": k + 1, "who": who(w, id), "school": school })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "lists": lists, "college_from": 2021 })
}

fn doc_json(w: &World, d: &media::Doc) -> Value {
    json!({
        "year": d.year,
        "show": d.show,
        "team": brief(w, d.team),
        "how": match d.how.as_str() { "forced" => "Ordered by the league", "volunteered" => "Volunteered", "followed" => "Followed a season", "unknown" => "", x => x },
        "episodes": d.episodes,
        "stars": d.stars.iter().map(|&id| who(w, id)).collect::<Vec<_>>(),
        "mine": Some(d.team) == st::seat_team(w),
    })
}

fn shows(w: &World) -> Value {
    let docs: Vec<Value> = w.stories.media.docs.iter().rev().take(40).map(|d| doc_json(w, d)).collect();
    let show = &w.stories.traditions.show;
    let stops: Vec<Value> = show
        .stops
        .iter()
        .map(|&(d, h, a)| {
            json!({ "date": date_text(d), "iso": d.0, "host": brief(w, TeamKey::College(h)), "visitor": brief(w, TeamKey::College(a)), "past": d < w.today })
        })
        .collect();
    let mut visits = show.visits.clone();
    visits.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    json!({
        "docs": docs,
        "show": traditions::SHOW,
        "show_from": traditions::SHOW_FROM,
        "show_year": show.year,
        "stops": stops,
        "visits": visits.iter().take(20).map(|&(p, n, last)| json!({ "team": brief(w, TeamKey::College(p)), "visits": n, "last": last })).collect::<Vec<_>>(),
    })
}

fn nicknames(w: &World) -> Value {
    let me = st::seat_team(w);
    let s = &w.stories;
    let live = |n: &&Nickname| !n.retired;
    let yours: Vec<Value> =
        s.nicknames.iter().rev().filter(|n| me.is_some() && n.team == me).take(60).map(|n| nick_json(w, n)).collect();
    let mut national: Vec<&Nickname> =
        s.nicknames.iter().filter(live).filter(|n| n.stage >= 2 && matches!(n.who, Who::Player(_))).collect();
    national.sort_by(|a, b| b.stage.cmp(&a.stage).then(b.stick.total_cmp(&a.stick)));
    let coaches: Vec<Value> = s
        .nicknames
        .iter()
        .rev()
        .filter(live)
        .filter(|n| matches!(n.who, Who::Coach(_)))
        .take(30)
        .map(|n| nick_json(w, n))
        .collect();
    let newest: Vec<Value> = s
        .nicknames
        .iter()
        .rev()
        .filter(|n| matches!(n.who, Who::Player(_)))
        .take(40)
        .map(|n| nick_json(w, n))
        .collect();
    let unit_list: Vec<_> = s.units.iter().rev().take(30).collect();
    let play_list: Vec<_> = s.plays.iter().rev().take(30).collect();
    let mut texts = link_many(w, unit_list.iter().map(|u| &u.why).chain(play_list.iter().map(|p| &p.what)));
    let units: Vec<Value> = unit_list
        .iter()
        .map(|u| {
            json!({ "team": brief(w, u.team), "unit": u.unit, "name": u.name, "from": u.from, "last": u.last, "why": texts.next(), "mine": Some(u.team) == me })
        })
        .collect();
    let plays: Vec<Value> = play_list
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "date": date_text(p.date),
                "year": p.date.year(),
                "teams": p.teams.iter().map(|&t| brief(w, t)).collect::<Vec<_>>(),
                "players": p.players.iter().map(|&id| who(w, id)).collect::<Vec<_>>(),
                "what": texts.next(),
                "call": p.call,
            })
        })
        .collect();
    json!({
        "yours": yours,
        "national": national.iter().take(40).map(|n| nick_json(w, n)).collect::<Vec<_>>(),
        "newest": newest,
        "coaches": coaches,
        "units": units,
        "plays": plays,
        "can_give": me.is_some(),
    })
}

// --- The traditions --------------------------------------------------------------------------------

fn traditions_tabs(w: &World) -> Vec<&'static str> {
    match seat_kind(w) {
        "nfl" => vec!["Uniforms", "Honors", "Farewells", "Celebrations", "Parades and rings"],
        "college" => {
            vec!["Rivalries", "Uniforms", "Honors", "Pregame show", "Stormings", "Celebrations", "Parades and rings"]
        }
        "hs" => vec!["Rivalries", "Uniforms", "Honors"],
        _ => vec!["Honors", "Pregame show", "Stormings", "Farewells", "Celebrations", "Parades and rings"],
    }
}

fn traditions_page(w: &World, tab: &str) -> Value {
    let tabs = traditions_tabs(w);
    let tab = if tabs.contains(&tab) { tab } else { tabs[0] };
    let body = match tab {
        "Rivalries" => rivalries(w),
        "Uniforms" => uniforms(w),
        "Honors" => honors(w),
        "Pregame show" => shows(w),
        "Stormings" => stormings(w),
        "Farewells" => farewells(w),
        "Celebrations" => celebrations(w),
        _ => parades(w),
    };
    json!({
        "tabs": tabs,
        "tab": tab,
        "level": seat_kind(w),
        "team": st::seat_team(w).map(|t| brief(w, t)),
        "season": w.year,
        "body": body,
    })
}

fn rivalries(w: &World) -> Value {
    match st::seat_team(w) {
        Some(TeamKey::College(p)) => {
            let rows: Vec<Value> = traditions::college_rivalries(w, p as usize)
                .into_iter()
                .map(|r| {
                    json!({
                        "game": r.game,
                        "trophy": r.trophy,
                        "rival": brief(w, TeamKey::College(r.rival as u32)),
                        "w": r.wins[0], "l": r.wins[1], "t": r.ties,
                        "holder": r.holder.map(|h| brief(w, TeamKey::College(h as u32))),
                        "yours": r.holder == Some(p as usize),
                        "real": r.real,
                        "since": r.since,
                    })
                })
                .collect();
            json!({ "rows": rows, "level": "college" })
        }
        Some(TeamKey::Hs(h)) => {
            let rows: Vec<Value> = w
                .stories
                .traditions
                .hs_rivals
                .iter()
                .filter(|r| r.schools.contains(&h))
                .map(|r| {
                    let k = (r.schools[1] == h) as usize;
                    let rival = r.schools[1 - k];
                    json!({
                        "game": format!("The {} game", w.school_name(rival as usize)),
                        "trophy": r.trophy,
                        "rival": brief(w, TeamKey::Hs(rival)),
                        "w": r.wins[k], "l": r.wins[1 - k], "t": 0,
                        "holder": r.holder.map(|x| brief(w, TeamKey::Hs(x))),
                        "yours": r.holder == Some(h),
                        "real": false,
                        "since": r.since,
                        "last": r.last,
                    })
                })
                .collect();
            json!({ "rows": rows, "level": "hs" })
        }
        _ => json!({ "rows": [] }),
    }
}

fn uniforms(w: &World) -> Value {
    let Some(me) = st::seat_team(w) else { return json!({ "none": true }) };
    let u = &w.stories.traditions.uniforms;
    let sets = traditions::sets(w, me);
    let record = traditions::record_by_set(w);
    let rows: Vec<Value> = sets
        .iter()
        .enumerate()
        .map(|(k, name)| {
            let r = record.iter().find(|x| &x.0 == name);
            json!({ "i": k, "name": name, "w": r.map_or(0, |x| x.1), "l": r.map_or(0, |x| x.2), "next": u.next == Some(k as u8) })
        })
        .collect();
    let worn: Vec<Value> = u
        .worn
        .iter()
        .rev()
        .take(24)
        .map(|(d, set, won)| json!({ "date": date_text(*d), "set": set, "won": won }))
        .collect();
    let reveals: Vec<Value> = u
        .reveals
        .iter()
        .rev()
        .filter(|r| r.0 == me)
        .take(8)
        .map(|(_, y, what)| json!({ "year": y, "what": what }))
        .collect();
    json!({
        "sets": rows,
        "next": u.next,
        "worn": worn,
        "reveals": reveals,
        "note": "Your staff picks by the game when you don't: home and road by the site, an alternate or throwback for the big ones. Recruits on a visit notice.",
    })
}

fn honor_json(w: &World, h: &traditions::TeamHonor) -> Value {
    json!({
        "kind": h.kind.name(),
        "key": format!("{:?}", h.kind),
        "team": brief(w, h.team),
        "who": who(w, h.player),
        "name": h.name,
        "pos": h.pos.name(),
        "number": h.number,
        "year": h.year,
        "years": h.years,
        "done": h.ceremony_done,
    })
}

fn honors(w: &World) -> Value {
    let me = st::seat_team(w);
    let t = &w.stories.traditions;
    let yours: Vec<Value> = t.honors.iter().rev().filter(|h| Some(h.team) == me).map(|h| honor_json(w, h)).collect();
    let league: Vec<Value> =
        t.honors.iter().rev().filter(|h| Some(h.team) != me).take(40).map(|h| honor_json(w, h)).collect();
    let retired: Vec<u8> = me.map(|m| t.retired_numbers(m)).unwrap_or_default();
    let policy = me.map(|m| {
        let p = media::honor_policy(&st::team_name(w, m), m.level());
        format!("{p:?}")
    });
    json!({
        "yours": yours,
        "league": league,
        "retired": retired,
        "policy": policy,
        "statues": t.honors.iter().filter(|h| h.kind == HonorKind::Statue && Some(h.team) == me).count(),
    })
}

fn stormings(w: &World) -> Value {
    let me = st::seat_team(w);
    let conf = match me {
        Some(TeamKey::College(p)) => Some(w.programs[p as usize].conference.clone()),
        _ => None,
    };
    let rows: Vec<Value> = w
        .stories
        .traditions
        .stormings
        .iter()
        .rev()
        .take(60)
        .map(|s| {
            json!({
                "date": date_text(s.date),
                "year": s.date.year(),
                "team": brief(w, TeamKey::College(s.team)),
                "opp": brief(w, TeamKey::College(s.opp)),
                "score": s.score,
                "fine": s.fine,
                "mine": me == Some(TeamKey::College(s.team)) || me == Some(TeamKey::College(s.opp)),
            })
        })
        .collect();
    let policy = conf.as_ref().map(|c| {
        let fines = media::storming_fines(c, w.year);
        json!({ "conference": c, "fines": fines, "from": media::storming_policy_from(c, w.year) })
    });
    json!({ "rows": rows, "policy": policy })
}

fn farewells(w: &World) -> Value {
    let c = &w.stories.careers;
    let tours: Vec<Value> = c
        .farewells
        .iter()
        .rev()
        .take(30)
        .map(|f| {
            json!({
                "who": who(w, f.id),
                "year": f.year,
                "team": brief(w, TeamKey::Nfl(f.team)),
                "gifts": f.gifts.iter().map(|(t, g)| json!({ "team": brief(w, TeamKey::Nfl(*t)), "gift": g })).collect::<Vec<_>>(),
                "done": f.last_game,
            })
        })
        .collect();
    let one_day: Vec<Value> = c
        .one_day
        .iter()
        .rev()
        .take(30)
        .map(|&(id, y, club)| {
            json!({ "who": who(w, id), "year": y, "team": (club != u16::MAX).then(|| brief(w, TeamKey::Nfl(club))), "passed": club == u16::MAX })
        })
        .collect();
    json!({ "tours": tours, "one_day": one_day, "asks": asks(w) })
}

fn celebrations(w: &World) -> Value {
    let rows: Vec<Value> = w
        .stories
        .careers
        .celebrations
        .iter()
        .rev()
        .take(60)
        .map(|(id, name, y)| json!({ "who": who(w, *id), "name": name, "year": y }))
        .collect();
    let year = w.year;
    let rule = match seat_kind(w) {
        "college" if year >= 2011 => {
            "College: a taunt on the way to the end zone is a live-ball foul, enforced from the spot, so the points come off. Celebrations after the score cost 15 yards on the kickoff."
        }
        "college" => "College: unsportsmanlike celebrations cost 15 yards on the kickoff; the score stands.",
        "hs" => {
            "High school: any celebration aimed at an opponent draws an unsportsmanlike flag, and officials call it tight."
        }
        _ if year >= 2017 => {
            "NFL since 2017: group celebrations, using the ball as a prop and going to the ground are allowed. Taunting, sexual gestures and anything aimed at an opponent still draw 15 yards."
        }
        _ => {
            "NFL before 2017: the 'No Fun League'. Props, going to the ground and choreographed group celebrations draw 15 yards."
        }
    };
    json!({ "rows": rows, "rule": rule })
}

fn parades(w: &World) -> Value {
    let rows: Vec<Value> = w
        .stories
        .traditions
        .parades
        .iter()
        .rev()
        .take(40)
        .map(|&(y, t, crowd)| json!({ "year": y, "team": brief(w, t), "crowd": crowd, "mine": Some(t) == st::seat_team(w) }))
        .collect();
    let ring_list: Vec<_> = w.stories.story.office.iter().rev().filter(|m| m.kind == MementoKind::Ring).collect();
    let mut texts = link_many(w, ring_list.iter().map(|m| &m.text));
    let rings: Vec<Value> = ring_list
        .iter()
        .map(|m| json!({ "date": date_text(m.date), "text": texts.next(), "team": m.team.map(|t| brief(w, t)) }))
        .collect();
    json!({ "rows": rows, "rings": rings })
}

// --- The player page's Story tab -------------------------------------------------------------------

fn player(w: &World, id: u32) -> Result<Value, String> {
    let i = views::person_index(w, id).ok_or("No such player.")?;
    let s = &w.stories;
    let names: Vec<Value> = nick::all_of(s, Who::Player(id)).into_iter().map(|n| nick_json(w, n)).collect();
    let team = numbers::team_of(w, i);
    let mine = team.is_some() && team == st::seat_team(w);
    let covers: Vec<Value> = s
        .media
        .magazines
        .iter()
        .filter(|m| m.covers.contains(&id))
        .map(|m| json!({ "year": m.year, "title": m.title }))
        .collect();
    let all_conf: Vec<Value> = s
        .media
        .magazines
        .iter()
        .filter(|m| m.all_conf.contains(&id))
        .map(|m| json!({ "year": m.year, "title": m.title }))
        .collect();
    let jerseys: Vec<Value> = s
        .media
        .jerseys
        .iter()
        .filter_map(|(y, college, ids)| {
            ids.iter().position(|&x| x == id).map(|k| json!({ "year": y, "rank": k + 1, "list": if *college { "College jerseys" } else { "NFLPA top 50" } }))
        })
        .collect();
    let honors: Vec<Value> = s.traditions.honors.iter().filter(|h| h.player == id).map(|h| honor_json(w, h)).collect();
    let his: Vec<_> = s.plays.iter().filter(|p| p.players.contains(&id)).collect();
    let mut texts = link_many(w, his.iter().map(|p| &p.what));
    let plays: Vec<Value> = his
        .iter()
        .map(|p| json!({ "name": p.name, "date": date_text(p.date), "what": texts.next(), "call": p.call }))
        .collect();
    let docs: Vec<Value> = s
        .media
        .docs
        .iter()
        .filter(|d| d.stars.contains(&id))
        .map(|d| json!({ "year": d.year, "show": d.show }))
        .collect();
    let farewell = s.careers.farewells.iter().find(|f| f.id == id).map(|f| {
        json!({ "year": f.year, "gifts": f.gifts.iter().map(|(t, g)| json!({ "team": brief(w, TeamKey::Nfl(*t)), "gift": g })).collect::<Vec<_>>(), "done": f.last_game })
    });
    let one_day = s
        .careers
        .one_day
        .iter()
        .find(|x| x.0 == id && x.2 != u16::MAX)
        .map(|x| json!({ "year": x.1, "team": brief(w, TeamKey::Nfl(x.2)) }));
    let fantasy = media::fantasy_line(w, id)
        .map(|(pts, g, rank)| json!({ "pts": (pts * 10.0).round() / 10.0, "games": g, "rank": rank }));
    let number = team.and_then(|t| numbers::number_of(w, t, id));
    let pro = w.people[i].stage == afm_engine::world::Stage::Pro;
    let vg_title = media::games_out(w.today).into_iter().find(|g| g.1 == pro).map(|g| g.0);
    Ok(json!({
        "id": id,
        "name": w.people[i].name,
        "nick": nick::current(s, Who::Player(id)).map(|n| n.name.clone()),
        "names": names,
        "number": number,
        "rating": media::game_rating(w, i),
        "vg": vg_title,
        "fantasy": fantasy,
        "covers": covers,
        "all_conf": all_conf,
        "jerseys": jerseys,
        "honors": honors,
        "plays": plays,
        "docs": docs,
        "celebration": s.careers.celebration(id),
        "farewell": farewell,
        "one_day": one_day,
        "merch": nick::merch(s, id),
        "can_name": mine,
    }))
}

// --- Your answers --------------------------------------------------------------------------------

fn answer(w: &mut World, args: &Value) -> Reply {
    let what = args.get("what").and_then(Value::as_str).ok_or("Missing what.")?;
    let yes = args.get("yes").and_then(Value::as_bool).ok_or("Missing yes.")?;
    let (said, target, act) = match what {
        "one_day" => {
            let id = w.stories.careers.offer.map(|o| o.0).ok_or("No one is asking.")?;
            let said = st::careers::answer_one_day(w, yes)?;
            (said, id, if yes { st::ACT_ONE_DAY_YES } else { st::ACT_ONE_DAY_NO })
        }
        "doc" => {
            let year = w.stories.media.doc_offer.as_ref().map(|o| o.0 as u32).ok_or("No series is asking.")?;
            if w.stories.media.doc_answer.is_some() {
                return Err("You already answered.".into());
            }
            let said = media::answer_doc(w, yes)?;
            (said, year, if yes { st::ACT_DOC_YES } else { st::ACT_DOC_NO })
        }
        _ => return Err("No such ask.".into()),
    };
    // The letter shows the answer you gave here.
    if let Some(c) = w.career.as_mut() {
        for m in c.desk.mails.iter_mut().filter(|m| m.answered.is_none()) {
            if let Some(k) = m.answers.iter().position(|a| a.act == act && a.target == target) {
                m.answered = Some(k as u8);
                m.read = true;
            }
        }
    }
    Ok(json!({ "said": link_one(w, &said) }))
}

/// Files the letters for the asks that wait on you (after the clock moves).
pub fn after_days(w: &mut World) {
    if w.career.is_none() {
        return;
    }
    // A career just started: your story has the hire from the first day.
    let today = w.today;
    st::story::notice_job(w, today);
    if let Some((id, club, asked)) = w.stories.careers.offer {
        let name =
            views::person_index(w, id).map(|i| w.people[i].name.clone()).unwrap_or_else(|| "A former star".into());
        let club_name = w.nfl_team_name(club as usize);
        let you = w.career.as_ref().map(|c| c.you.name.clone()).unwrap_or_default();
        let mut m = mail::letter(
            &format!("stories-oneday-{id}"),
            asked,
            9 * 60 + 40,
            Head::Memo,
            club_name.clone(),
            "Player engagement".into(),
            "Director of player engagement".into(),
            "Player engagement".into(),
            you,
            format!("{name} wants to retire a {}", club_name.rsplit(' ').next().unwrap_or("member of the club")),
        );
        m.body = vec![
            format!("{} called this morning. He wants to sign a one-day contract and retire with us, where it started: a press conference at the facility, his family there, a jersey with his number.", links::player(id, &name)),
            "It costs nothing against the cap and the fans will love it. Say the word and we set the date; if we hear nothing in ten days, we'll go ahead and sign him.".into(),
        ];
        m.sign = "Player engagement".into();
        m.keys = mail::PEOPLE | mail::MEDIA;
        m.about = Some((mail::ABOUT_PLAYER, id));
        m.ids = vec![id];
        m.answers = vec![
            mail::answer("Bring him home", "He signs for a day and retires with your club.", st::ACT_ONE_DAY_YES, id),
            mail::answer("Not this time", "He retires without the ceremony.", st::ACT_ONE_DAY_NO, id),
        ];
        m.answer_by = Some(asked.add(9));
        m.loud = 2;
        mail::send(w, m);
    }
    if let Some((year, show)) = w.stories.media.doc_offer.clone()
        && year as i32 == w.year
        && let Some(me) = st::seat_team(w)
    {
        let asked = Date::ymd(year as i32, 5, 15);
        let team = st::team_name(w, me);
        let you = w.career.as_ref().map(|c| c.you.name.clone()).unwrap_or_default();
        let nfl = matches!(me, TeamKey::Nfl(_));
        let (org, from, role) = if nfl {
            ("NFL Films".to_string(), "Senior coordinating producer".to_string(), "NFL Films".to_string())
        } else {
            (show.clone(), "Series producer".to_string(), "Production".to_string())
        };
        let mut m = mail::letter(
            &format!("stories-doc-{year}"),
            asked,
            11 * 60 + 15,
            Head::Email,
            org,
            "Documentary series".into(),
            from,
            role,
            you,
            format!("{show}: we'd like to follow {team} this summer"),
        );
        m.body = vec![
            format!("We'd like {team} for {show} this year. Our crews would be with you from the start of camp: meetings, the locker room, the cut-down days, your office and your home if you'll have us."),
            "Teams on the show see their players' names travel (a breakout star comes out of every season) and their phones ring with sponsors. It is also a camera in every room, and some players perform for it.".into(),
            format!("We need your answer by May 31. With none, {} decides.", if nfl { "your owner" } else { "your athletic director" }),
        ];
        m.sign = "Documentary series".into();
        m.keys = mail::MEDIA;
        m.about = Some((mail::ABOUT_TEAM, mail::team_code(tref(me))));
        m.answers = vec![
            mail::answer(
                "Say yes",
                "The cameras come with camp; episodes air from August.",
                st::ACT_DOC_YES,
                year as u32,
            ),
            mail::answer(
                "Say no",
                "The series looks elsewhere (unless the league orders you on).",
                st::ACT_DOC_NO,
                year as u32,
            ),
        ];
        m.answer_by = Some(Date::ymd(year as i32, 5, 31));
        m.loud = 2;
        mail::send(w, m);
    }
}
