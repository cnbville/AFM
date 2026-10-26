//! V16, the race: where your team stands week by week and why it moved.
//! College: the AP and Coaches polls (the Harris poll and the BCS
//! standings in their eras) and the committee's ranking, each week's
//! ranks, where the voters put a team on their latest ballots, and your
//! conference's standings. The NFL: the playoff picture, every team's
//! place in its conference week by week by the seeding rule, and your
//! division. High school: your state's polls, your class and all classes.

use std::cmp::Ordering;

use afm_engine::calendar::{CfbPostseason, Date};
use afm_engine::world::World;
use afm_engine::world::charts::{PollRow, PollWeek, StatePoll};
use serde_json::{Value, json};

use crate::Reply;
use crate::views::{self, TeamRef};

const CONF: [&str; 2] = ["AFC", "NFC"];
const DIV: [&str; 8] =
    ["AFC East", "AFC North", "AFC South", "AFC West", "NFC East", "NFC North", "NFC South", "NFC West"];

pub fn board(w: &World, args: &Value) -> Reply {
    let tab = args.get("tab").and_then(Value::as_str);
    match views::mine(w) {
        Some(TeamRef::College(j)) => college(w, j, tab),
        Some(TeamRef::Nfl(t)) => nfl(w, t, tab),
        Some(TeamRef::HighSchool(h)) => high_school(w, h, tab),
        Some(TeamRef::Other(_)) => Ok(json!({ "level": "none", "why": "Your league keeps no poll." })),
        None => Err("Start a career first.".into()),
    }
}

/// "1st", "22nd".
fn ord(n: usize) -> String {
    let s = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

/// "Sun 10 Oct".
fn short(d: Date) -> String {
    let t = views::date_text(d);
    t.rsplit_once(' ').map_or(t.clone(), |x| x.0.to_string())
}

/// A rank's move between two lists (0 = not ranked).
fn change(prev: usize, now: usize) -> String {
    match (prev, now) {
        (0, 0) => "unranked".into(),
        (0, _) => "new".into(),
        (_, 0) => "out".into(),
        (p, n) if n < p => format!("up {}", p - n),
        (p, n) if n > p => format!("down {}", n - p),
        _ => "same".into(),
    }
}

/// One of your games as the board tells it.
struct Res {
    date: Date,
    opp: usize,
    home: bool,
    us: u32,
    them: u32,
}

impl Res {
    fn won(&self) -> bool {
        self.us > self.them
    }
    fn text(&self, opp: &str, rank: usize) -> String {
        let o = if rank > 0 { format!("No. {rank} {opp}") } else { opp.to_string() };
        let verb = match (self.us.cmp(&self.them), self.home) {
            (Ordering::Greater, true) => format!("Beat {o}"),
            (Ordering::Greater, false) => format!("Won at {o}"),
            (Ordering::Less, true) => format!("Lost to {o}"),
            (Ordering::Less, false) => format!("Lost at {o}"),
            (Ordering::Equal, _) => format!("Tied {o}"),
        };
        format!("{verb} {}-{}", self.us, self.them)
    }
}

/// The kept game of yours on a date, for a link to its page.
fn seat_game(w: &World, d: Date) -> Option<usize> {
    w.seat_games().iter().position(|g| g.date == d)
}

/// Letters A, B, ... on the last five moves.
fn lettered(mut moves: Vec<Value>) -> Vec<Value> {
    let skip = moves.len().saturating_sub(5);
    moves.drain(..skip);
    for (k, m) in moves.iter_mut().enumerate() {
        m["k"] = json!(["A", "B", "C", "D", "E"][k]);
    }
    moves
}

// ---------------------------------------------------------------------------
// College.

fn rank_in(p: &PollWeek, t: usize) -> usize {
    p.top.iter().position(|&x| x == t).map_or(0, |k| k + 1)
}

fn col_label(w: &World, p: &PollWeek) -> String {
    if p.kind.starts_with("preseason") {
        "PRE".into()
    } else if p.kind.starts_with("final") {
        "FINAL".into()
    } else if p.key == "cfp" || p.key == "bcs" {
        let (_, m, d) = p.date.parts();
        const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        format!("{d} {}", M[(m - 1) as usize].to_uppercase())
    } else {
        format!("WK {}", w.dates.cfb_weeks.iter().filter(|&&d| d < p.date).count())
    }
}

fn college(w: &World, j: usize, tab: Option<&str>) -> Reply {
    // Before this season's first poll, last season's (the offseason).
    let now_weeks = w.poll_weeks(false);
    let prev = now_weeks.is_empty() && !w.poll_weeks(true).is_empty();
    let weeks = if prev { w.poll_weeks(true) } else { now_weeks };
    let year = w.college_year().map_or(w.year, |y| if prev { y - 1 } else { y });
    let prog = &w.programs[j];
    let conf = prog.conference.clone();
    let conf_name = w.conference_name(&conf);
    let name = |t: usize| views::team_name(w, TeamRef::College(t));
    let mut keys: Vec<(&str, &str)> = Vec::new();
    for (k, n) in [
        ("ap", "AP poll"),
        ("coaches", "Coaches poll"),
        ("harris", "Harris poll"),
        ("bcs", "BCS standings"),
        ("cfp", "Playoff ranking"),
    ] {
        if weeks.iter().any(|p| p.key == k) {
            keys.push((k, n));
        }
    }
    let tab = tab
        .filter(|t| *t == "conf" || keys.iter().any(|k| k.0 == *t))
        .unwrap_or_else(|| keys.first().map_or("conf", |k| k.0))
        .to_string();
    let mut tabs: Vec<Value> = keys.iter().map(|(k, n)| json!({ "k": k, "n": n })).collect();
    tabs.push(json!({ "k": "conf", "n": format!("{conf_name} standings") }));

    // Every game played this season, and ours.
    let fixtures = w.college_fixtures_in(None, prev);
    let played: Vec<_> = fixtures.iter().filter(|f| f.score.is_some()).collect();
    let ours_in = |a: Date, b: Date| -> Vec<Res> {
        played
            .iter()
            .filter(|f| f.date > a && f.date <= b && (f.home == j || f.away == j))
            .map(|f| {
                let s = f.score.unwrap_or_default();
                let home = f.home == j;
                Res {
                    date: f.date,
                    opp: if home { f.away } else { f.home },
                    home: home || f.neutral,
                    us: s[if home { 0 } else { 1 }],
                    them: s[if home { 1 } else { 0 }],
                }
            })
            .collect()
    };
    let lost_in = |t: usize, a: Date, b: Date| {
        played.iter().any(|f| {
            f.date > a && f.date <= b && {
                let s = f.score.unwrap_or_default();
                (f.home == t && s[0] < s[1]) || (f.away == t && s[1] < s[0])
            }
        })
    };
    let rec = w.college_record_in(j, prev);
    let latest = |key: &str| weeks.iter().rev().find(|p| p.key == key);
    let rank_text = |p: Option<&PollWeek>| {
        p.map_or("—".to_string(), |p| {
            let r = rank_in(p, j);
            if r > 0 { ord(r) } else { "NR".into() }
        })
    };
    let mut hero = vec![
        json!({ "k": "AP", "v": rank_text(latest("ap")) }),
        json!({ "k": "Coaches", "v": rank_text(latest("coaches")) }),
    ];
    if latest("cfp").is_some() {
        hero.push(json!({ "k": "Committee", "v": rank_text(latest("cfp")) }));
    }
    let pts = latest("ap").and_then(|p| {
        let k = p.top.iter().position(|&x| x == j)?;
        p.points.get(k).copied()
    });
    hero.push(json!({ "k": "AP points", "v": pts.map_or("—".to_string(), |x| x.to_string()) }));
    hero.push(json!({
        "k": "Record",
        // An independent has no conference record to show.
        "v": if conf_name.starts_with("Independent") {
            format!("{}-{} · {conf_name}", rec.wins, rec.losses)
        } else {
            format!("{}-{} · {}-{} {}", rec.wins, rec.losses, rec.conf_wins, rec.conf_losses, conf_name)
        },
    }));

    // The conference's standings.
    let mut table: Vec<(usize, afm_engine::world::seat::Record)> = w
        .programs
        .iter()
        .filter(|p| p.conference == conf && p.division == prog.division)
        .map(|p| (p.id, w.college_record_in(p.id, prev)))
        .collect();
    // As conferences publish them: conference winning share (0-0 last),
    // fewer conference losses, then the overall share and wins.
    let pct = |w: u32, l: u32| if w + l > 0 { w as f64 / (w + l) as f64 } else { -1.0 };
    table.sort_by(|a, b| {
        let (x, y) = (&a.1, &b.1);
        pct(y.conf_wins, y.conf_losses)
            .total_cmp(&pct(x.conf_wins, x.conf_losses))
            .then(x.conf_losses.cmp(&y.conf_losses))
            .then(pct(y.wins, y.losses).total_cmp(&pct(x.wins, x.losses)))
            .then(y.wins.cmp(&x.wins))
    });
    let ap = latest("ap").or(latest("cfp"));
    let standings = json!({
        "title": format!("{conf_name} standings"),
        "cols": ["Conference", "Overall", "AP"],
        "rows": table.iter().enumerate().map(|(k, (p, r))| json!({
            "place": k + 1,
            "t": views::team_brief(w, TeamRef::College(*p)),
            "me": *p == j,
            "cells": [format!("{}-{}", r.conf_wins, r.conf_losses), format!("{}-{}", r.wins, r.losses),
                ap.map_or(String::new(), |a| { let x = rank_in(a, *p); if x > 0 { x.to_string() } else { "—".into() } })],
        })).collect::<Vec<_>>(),
    });

    // The committee's first ranking, in its era.
    let note = match w.rules.cfb_postseason {
        CfbPostseason::FourTeam | CfbPostseason::TwelveTeam => {
            let d = w.dates.first_cfp_ranking;
            Some(json!({ "t": "Playoff committee's first ranking", "d": short(d), "done": w.today >= d }))
        }
        _ => None,
    };
    let fbs = prog.division == afm_engine::world::data::Division::FBS;
    let polls_only_fbs = (!fbs).then_some("The polls rank FBS teams. The world keeps no poll at your level, so your conference's standings are your race.");

    let cols_polls: Vec<&PollWeek> = weeks.iter().filter(|p| p.key == tab).collect();
    let Some(last) = cols_polls.last().copied() else {
        return Ok(json!({
            "level": "college",
            "team": views::team_brief(w, TeamRef::College(j)),
            "head": format!("{year} college football · {}", if weeks.is_empty() { "no polls yet: the preseason polls come out in August" } else if prev { "last season's conference race" } else { "the conference race" }),
            "tabs": tabs, "tab": tab, "hero": hero, "table": standings, "note": note, "fbsOnly": polls_only_fbs,
            "next": next_college(w, j, None),
        }));
    };
    let cols: Vec<String> = cols_polls.iter().map(|p| col_label(w, p)).collect();
    // The teams drawn: the latest list, the ones that fell out of the two
    // before, the conference's ranked teams, and you.
    let mut set: Vec<usize> = last.top.clone();
    for p in cols_polls.iter().rev().skip(1).take(2) {
        for &t in &p.top {
            if !set.contains(&t) {
                set.push(t);
            }
        }
    }
    for p in &cols_polls {
        for &t in &p.top {
            if w.programs[t].conference == conf && !set.contains(&t) {
                set.push(t);
            }
        }
    }
    if !set.contains(&j) {
        set.push(j);
    }
    // The latest ballots behind a voters' poll, where they are public: the
    // AP's since 2007, the Harris poll's, and the coaches' final ballots
    // only (their weekly ones are confidential). Else only the points and
    // first-place votes.
    let ballots: Vec<&[u16]> = match tab.as_str() {
        "ap" if year >= 2007 => w.voters.ap.iter().map(|v| v.ballot.as_slice()).collect(),
        "harris" => w.voters.harris.iter().map(|v| v.ballot.as_slice()).collect(),
        "coaches" if last.fin => w.voters.coaches.iter().map(|v| v.ballot.as_slice()).collect(),
        _ => Vec::new(),
    };
    let ballots: Vec<&[u16]> = ballots.into_iter().filter(|b| !b.is_empty()).collect();
    let lines: Vec<Value> = set
        .iter()
        .map(|&t| {
            let r = w.college_record_in(t, prev);
            let at = last.top.iter().position(|&x| x == t);
            let counts = (!ballots.is_empty()).then(|| {
                let mut c = [0u32; 26];
                for b in &ballots {
                    let k = b.iter().position(|&x| x as usize == t).map_or(25, |k| k.min(25));
                    c[k] += 1;
                }
                c.to_vec()
            });
            json!({
                "id": t,
                "t": views::team_brief(w, TeamRef::College(t)),
                "name": name(t),
                "rec": format!("{}-{}", r.wins, r.losses),
                "grp": w.conference_name(&w.programs[t].conference),
                "hl": w.programs[t].conference == conf,
                "me": t == j,
                "r": cols_polls.iter().map(|p| rank_in(p, t)).collect::<Vec<_>>(),
                "pts": at.and_then(|k| last.points.get(k)),
                "firsts": at.and_then(|k| last.firsts.get(k)),
                "b": counts,
            })
        })
        .collect();

    // Why you moved, poll to poll.
    let mut moves: Vec<Value> = Vec::new();
    for k in 1..cols_polls.len() {
        let (a, b) = (cols_polls[k - 1], cols_polls[k]);
        let (rp, rc) = (rank_in(a, j), rank_in(b, j));
        let games = ours_in(a.date, b.date);
        if games.is_empty() && rp == rc {
            continue;
        }
        let r = if games.is_empty() {
            "Bye week".to_string()
        } else {
            games.iter().map(|g| g.text(&name(g.opp), rank_in(a, g.opp))).collect::<Vec<_>>().join(", ")
        };
        // The ranked teams above you that lost, and who passed you.
        let above: Vec<usize> = a.top.iter().copied().take(if rp > 0 { rp - 1 } else { 25 }).collect();
        let fell: Vec<usize> = above.iter().copied().filter(|&t| lost_in(t, a.date, b.date)).collect();
        let passed: Vec<usize> = if rc > 0 {
            b.top
                .iter()
                .copied()
                .take(rc - 1)
                .filter(|&t| rp > 0 && (rank_in(a, t) == 0 || rank_in(a, t) > rp))
                .collect()
        } else {
            Vec::new()
        };
        let main = games.last();
        let opp_rank = main.map_or(0, |g| rank_in(a, g.opp));
        let why = college_why(rp, rc, main, opp_rank, &fell, &passed, &name);
        moves.push(json!({
            "col": k,
            "r": r,
            "rank": if rc > 0 { rc.to_string() } else { "NR".into() },
            "ch": change(rp, rc),
            "why": why,
            "game": main.and_then(|g| seat_game(w, g.date)),
        }));
    }
    let kind = keys.iter().find(|k| k.0 == tab).map_or("poll", |k| k.1);
    let head = format!(
        "{year} college football · {} · {} of {}",
        match cols.last().map(String::as_str) {
            Some("PRE") => "preseason".to_string(),
            Some("FINAL") => "final".to_string(),
            Some(c) if c.starts_with("WK ") => format!("after week {}", &c[3..]),
            _ => "this week".to_string(),
        },
        kind,
        short(last.date)
    );
    Ok(json!({
        "level": "college",
        "team": views::team_brief(w, TeamRef::College(j)),
        "head": head,
        "tabs": tabs,
        "tab": tab,
        "hero": hero,
        "chart": {
            "cols": cols,
            "depth": 25,
            "cut": Value::Null,
            "out": "NR",
            "lines": lines,
            "hls": [conf_name.clone(), "Top 10".to_string(), "All".to_string()],
            "poll": kind,
            "date": short(last.date),
        },
        "ballots": (!ballots.is_empty()).then(|| json!({ "voters": ballots.len() })),
        "moves": lettered(moves),
        "next": next_college(w, j, Some(last)),
        "note": note,
        "table": standings,
        "fbsOnly": polls_only_fbs,
    }))
}

fn college_why(
    rp: usize,
    rc: usize,
    game: Option<&Res>,
    opp_rank: usize,
    fell: &[usize],
    passed: &[usize],
    name: &dyn Fn(usize) -> String,
) -> String {
    let names = |v: &[usize]| {
        let n: Vec<String> = v.iter().take(2).map(|&t| name(t)).collect();
        match v.len() {
            0 => String::new(),
            1 => n[0].clone(),
            2 => format!("{} and {}", n[0], n[1]),
            k => format!("{}, {} and {} others", n[0], n[1], k - 2),
        }
    };
    let made_room = if fell.is_empty() {
        String::new()
    } else if fell.len() == 1 {
        format!(" {} lost and made room.", names(fell))
    } else {
        format!(" {} teams above you lost.", fell.len())
    };
    let Some(g) = game else {
        return match rc.cmp(&rp) {
            _ if rp == 0 && rc > 0 => format!("A bye week, and still in.{made_room}"),
            _ if rc == 0 => "A bye week, and teams below you won their way past.".into(),
            Ordering::Less => format!("A bye week.{made_room}"),
            Ordering::Greater => format!("A bye week: {} won and passed you.", names(passed)),
            Ordering::Equal => "A bye week, and nothing changed above you.".into(),
        };
    };
    let road = if g.home { "" } else { " Road wins count." };
    if g.won() {
        match (rp, rc) {
            (0, 0) => "A win, but not enough votes to be ranked yet.".into(),
            (0, _) if opp_rank > 0 => {
                format!("In the poll: a win over a ranked team counts for more than the score.{made_room}")
            }
            (0, _) => format!("In the poll after the win.{made_room}"),
            (_, 0) => {
                if passed.is_empty() {
                    "Won, but voters dropped you out: others passed you with bigger wins.".into()
                } else {
                    format!("Won, but voters dropped you out: {} passed you with bigger wins.", names(passed))
                }
            }
            (p, c) if c < p => {
                if fell.is_empty() {
                    format!("Voters liked the win more than the results above you.{road}")
                } else {
                    format!("{}{road}", made_room.trim_start())
                }
            }
            (p, c) if c == p => {
                if fell.is_empty() {
                    "Every team just above you also won, so there was no room to climb.".into()
                } else {
                    "Teams below you won bigger games and kept pace.".into()
                }
            }
            _ => {
                let close = g.us.saturating_sub(g.them) <= 7 && opp_rank == 0;
                let lead = if close { "A close win over an unranked team cost you. " } else { "" };
                if passed.is_empty() {
                    format!("{lead}Voters moved others ahead.").trim().to_string()
                } else {
                    format!("{lead}{} passed you with bigger wins.", names(passed))
                }
            }
        }
    } else {
        match (rp, rc) {
            (0, _) => "A loss: out of the race for votes for now.".into(),
            (_, 0) if opp_rank == 0 => "A ranked team that loses to an unranked one almost always drops out.".into(),
            (_, 0) => "Out of the poll after the loss.".into(),
            (p, c) if c <= p => "Teams above you lost too.".into(),
            (p, c) if opp_rank > 0 && opp_rank <= 10 && c - p <= 5 => {
                format!("A loss to No. {opp_rank} costs less than most.")
            }
            (p, c) => format!("The loss cost {} spots.", c - p),
        }
    }
}

/// Your games still to play, with what each can do for you.
fn next_college(w: &World, j: usize, poll: Option<&PollWeek>) -> Vec<Value> {
    let me = poll.map_or(0, |p| rank_in(p, j));
    let mut left: Vec<_> = w
        .college_fixtures(Some(j))
        .into_iter()
        .filter(|f| f.score.is_none() && !f.cancelled && f.date >= w.today)
        .collect();
    left.sort_by_key(|f| f.date);
    let best = left
        .iter()
        .map(|f| poll.map_or(0, |p| rank_in(p, if f.home == j { f.away } else { f.home })))
        .filter(|&r| r > 0)
        .min();
    left.iter()
        .take(4)
        .map(|f| {
            let home = f.home == j;
            let opp = if home { f.away } else { f.home };
            let r = poll.map_or(0, |p| rank_in(p, opp));
            let rec = w.college_record(opp);
            let fcs = w.programs[opp].division == afm_engine::world::data::Division::FCS;
            let words = match r {
                0 if fcs => "FCS: little to gain, a lot to lose",
                0 if me > 0 && rec.wins > rec.losses => "unranked but winning: a loss costs a lot",
                0 if me > 0 => "unranked: little to gain, a lot to lose",
                0 if rec.wins > rec.losses => "a win over a winning team keeps you in view",
                0 => "a win keeps you in view",
                r if Some(r) == best && (me == 0 || r < me) => "the biggest chance",
                _ if home => "a ranked win at home",
                _ => "a ranked win on the road",
            };
            json!({
                "d": short(f.date),
                "o": format!("{}{}", if home || f.neutral { "" } else { "at " }, views::team_name(w, TeamRef::College(opp))),
                "t": views::team_brief(w, TeamRef::College(opp)),
                "r": if r > 0 { format!("No. {r}") } else { "—".into() },
                "s": words,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The NFL.

fn nfl(w: &World, t: usize, tab: Option<&str>) -> Reply {
    // Before this season's first games, last season's race (the
    // offseason).
    let now = w.nfl_race();
    let last = now.as_ref().is_none_or(|r| r.weeks.is_empty()).then(|| w.nfl_race_prev()).flatten();
    let (race, prev) = match (now, last) {
        (_, Some(r)) if !r.weeks.is_empty() => (r, true),
        (Some(r), _) => (r, false),
        _ => return Ok(json!({ "level": "nfl", "none": true, "why": "The season's schedule isn't out yet." })),
    };
    if t >= race.conf.len() {
        return Ok(json!({ "level": "nfl", "none": true, "why": "Your club isn't on this season's schedule." }));
    }
    let name = |x: usize| views::team_name(w, TeamRef::Nfl(x));
    let my_conf = race.conf[t];
    let my_div = race.div[t];
    let cut = (w.rules.nfl_playoff_teams / 2) as usize;
    let tab = match tab {
        Some("other") => "other",
        Some("division") => "division",
        _ => "picture",
    };
    let tabs = json!([
        { "k": "picture", "n": format!("{} picture", CONF[my_conf]) },
        { "k": "other", "n": format!("{} picture", CONF[1 - my_conf]) },
        { "k": "division", "n": DIV[my_div] },
    ]);
    let conf = if tab == "other" { 1 - my_conf } else { my_conf };
    let now = race.weeks.last().map(|x| &x.1);
    let rec = |x: usize| {
        now.map_or("0-0".to_string(), |rows| {
            let r = rows[x];
            if r.ties > 0 { format!("{}-{}-{}", r.wins, r.losses, r.ties) } else { format!("{}-{}", r.wins, r.losses) }
        })
    };
    let fixtures = w.nfl_fixtures_in(None, prev);
    let left_of = |x: usize| fixtures.iter().filter(|f| !f.played && (f.home == x || f.away == x)).count();
    let div_place = |x: usize| -> usize {
        now.map_or(0, |rows| {
            1 + (0..race.conf.len()).filter(|&y| race.div[y] == race.div[x] && rows[y].place < rows[x].place).count()
        })
    };
    // Games behind the last seed: half the swing in wins and losses.
    let back = |x: usize| -> Option<f64> {
        let rows = now?;
        let last = (0..race.conf.len()).find(|&y| race.conf[y] == race.conf[x] && rows[y].seed == Some(cut as u8))?;
        let (a, b) = (rows[last], rows[x]);
        Some(((a.wins as f64 - b.wins as f64) + (b.losses as f64 - a.losses as f64)) / 2.0)
    };
    let seed_text = |x: usize| -> String {
        let Some(rows) = now else { return "—".into() };
        match rows[x].seed {
            Some(s) => format!("{} seed", ord(s as usize)),
            None => format!("Out: {} in the {}", ord(rows[x].place as usize), CONF[race.conf[x]]),
        }
    };
    let gb = back(t).filter(|&g| g > 0.0 && now.is_some_and(|r| r[t].seed.is_none()));
    let hero = json!([
        { "k": "Seed", "v": seed_text(t), "warn": now.is_some_and(|r| r[t].seed.is_none()) },
        { "k": "Record", "v": rec(t) },
        { "k": "Division", "v": format!("{} in the {}", ord(div_place(t)), DIV[my_div]) },
        { "k": "Games back", "v": gb.map_or("—".to_string(), |g| format!("{g}")) },
        { "k": "Games left", "v": left_of(t).to_string() },
    ]);
    // The division table.
    let mut div_teams: Vec<usize> = (0..race.conf.len()).filter(|&x| race.div[x] == my_div).collect();
    div_teams.sort_by_key(|&x| now.map_or(0, |r| r[x].place));
    let table = json!({
        "title": DIV[my_div],
        "cols": ["Record", "Conference", "Seed"],
        "rows": div_teams.iter().enumerate().map(|(k, &x)| json!({
            "place": k + 1,
            "t": views::team_brief(w, TeamRef::Nfl(x)),
            "me": x == t,
            "cells": [rec(x), now.map_or("—".to_string(), |r| ord(r[x].place as usize)), now.and_then(|r| r[x].seed).map_or("—".to_string(), |s| s.to_string())],
        })).collect::<Vec<_>>(),
    });
    let note = if race.booked {
        json!({ "t": "The field is set", "d": "", "done": true })
    } else {
        race.dates
            .iter()
            .flatten()
            .max()
            .map_or(Value::Null, |d| json!({ "t": "The regular season ends", "d": short(*d), "done": false }))
    };
    // Your games still to play.
    let mut mine: Vec<_> = fixtures.iter().filter(|f| !f.played && (f.home == t || f.away == t)).collect();
    mine.sort_by_key(|f| f.week);
    let next: Vec<Value> = mine
        .iter()
        .take(4)
        .map(|f| {
            let home = f.home == t;
            let o = if home { f.away } else { f.home };
            let words = if race.div[o] == my_div {
                "a division game: it counts in the tiebreaks twice over"
            } else if race.conf[o] == my_conf {
                if now.is_some_and(|r| r[o].place as usize <= cut + 3) {
                    "chasing the same seeds: the conference record breaks ties"
                } else {
                    "a conference game: the conference record breaks ties"
                }
            } else {
                "against the other conference"
            };
            json!({
                "d": f.when.map_or(format!("Week {}", f.week), short),
                "o": format!("{}{}", if home { "" } else { "at " }, name(o)),
                "t": views::team_brief(w, TeamRef::Nfl(o)),
                "r": rec(o),
                "s": words,
            })
        })
        .collect();
    if race.weeks.is_empty() || tab == "division" {
        return Ok(json!({
            "level": "nfl", "team": views::team_brief(w, TeamRef::Nfl(t)),
            "head": format!("{} NFL season · {}", race.year, if race.weeks.is_empty() { "no games played yet" } else { "the division race" }),
            "tabs": tabs, "tab": tab, "hero": hero, "table": table, "note": note, "next": next,
        }));
    }
    let cols: Vec<String> = race.weeks.iter().map(|(k, _)| format!("WK {}", k + 1)).collect();
    let teams: Vec<usize> = (0..race.conf.len()).filter(|&x| race.conf[x] == conf).collect();
    let lines: Vec<Value> = teams
        .iter()
        .map(|&x| {
            let mut games: Vec<_> = fixtures.iter().filter(|f| !f.played && (f.home == x || f.away == x)).collect();
            games.sort_by_key(|f| f.week);
            json!({
                "id": x,
                "t": views::team_brief(w, TeamRef::Nfl(x)),
                "name": name(x),
                "rec": rec(x),
                "grp": DIV[race.div[x]],
                "hl": race.div[x] == my_div,
                "me": x == t,
                "r": race.weeks.iter().map(|(_, rows)| rows[x].place).collect::<Vec<_>>(),
                "seed": now.and_then(|r| r[x].seed),
                "lead": now.is_some_and(|r| r[x].leader),
                "back": back(x),
                "left": games.iter().take(9).map(|f| {
                    let o = if f.home == x { f.away } else { f.home };
                    json!({ "w": f.week, "o": format!("{}{}", if f.home == x { "" } else { "at " }, name(o)), "r": rec(o) })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();
    // Why you moved, week to week (your own conference only).
    let mut moves: Vec<Value> = Vec::new();
    if conf == my_conf {
        for k in 1..race.weeks.len() {
            let (wk, rows) = (race.weeks[k].0, &race.weeks[k].1);
            let prev = &race.weeks[k - 1].1;
            let (pp, pc) = (prev[t].place as usize, rows[t].place as usize);
            let game = fixtures.iter().find(|f| f.week == wk + 1 && f.played && (f.home == t || f.away == t));
            let res = game.and_then(|f| {
                let s = f.score?;
                let home = f.home == t;
                Some(Res {
                    date: f.when.unwrap_or(w.today),
                    opp: if home { f.away } else { f.home },
                    home,
                    us: s[if home { 0 } else { 1 }],
                    them: s[if home { 1 } else { 0 }],
                })
            });
            if res.is_none() && pp == pc {
                continue;
            }
            let lost_wk = |x: usize| {
                fixtures.iter().any(|f| {
                    f.week == wk + 1
                        && f.played
                        && f.score.is_some_and(|s| (f.home == x && s[0] < s[1]) || (f.away == x && s[1] < s[0]))
                })
            };
            // Who you passed and who passed you.
            let passed_them: Vec<usize> = teams
                .iter()
                .copied()
                .filter(|&x| prev[x].place < prev[t].place && rows[x].place > rows[t].place)
                .collect();
            let passed_you: Vec<usize> = teams
                .iter()
                .copied()
                .filter(|&x| prev[x].place > prev[t].place && rows[x].place < rows[t].place)
                .collect();
            let list = |v: &[usize]| {
                let n: Vec<String> = v.iter().take(2).map(|&x| name(x)).collect();
                if v.len() > 2 { format!("{} and {} others", n.join(", "), v.len() - 2) } else { n.join(" and ") }
            };
            let seed = |p: usize| {
                if p <= cut { format!("the {} seed", ord(p)) } else { format!("{} in the {}", ord(p), CONF[my_conf]) }
            };
            let won = res.as_ref().is_some_and(|r| r.won());
            let head = match &res {
                None => "A bye week".to_string(),
                Some(_) if won => "Won".to_string(),
                Some(r) if r.us == r.them => "Tied".to_string(),
                Some(_) => "Lost".to_string(),
            };
            let why = match pc.cmp(&pp) {
                Ordering::Less => {
                    let lost: Vec<usize> = passed_them.iter().copied().filter(|&x| lost_wk(x)).collect();
                    if lost.is_empty() {
                        format!("{head}, and up to {}: {} fell behind you.", seed(pc), list(&passed_them))
                    } else {
                        format!("{head}, and {} lost: up to {}.", list(&lost), seed(pc))
                    }
                }
                Ordering::Greater => {
                    if won {
                        format!(
                            "Won, but {} won too and hold the edge on record or division wins: down to {}.",
                            list(&passed_you),
                            seed(pc)
                        )
                    } else {
                        format!("{head}: {} passed you, down to {}.", list(&passed_you), seed(pc))
                    }
                }
                Ordering::Equal => {
                    if pc <= cut {
                        format!("{head}, holding {}.", seed(pc))
                    } else {
                        let gb = back(t).filter(|&g| g > 0.0);
                        format!(
                            "{head}, still {}{}.",
                            seed(pc),
                            gb.map_or(String::new(), |g| format!(", {g} games back of the last seed"))
                        )
                    }
                }
            };
            moves.push(json!({
                "col": k,
                "r": res.as_ref().map_or("Bye week".to_string(), |r| r.text(&name(r.opp), 0)),
                "rank": if pc <= cut { format!("{} seed", ord(pc)) } else { ord(pc) },
                "ch": if pc < pp { format!("up {}", pp - pc) } else if pc > pp { format!("down {}", pc - pp) } else { "same".into() },
                "why": why,
                "game": res.as_ref().and_then(|r| seat_game(w, r.date)),
            }));
        }
    }
    let last_week = race.weeks.last().map_or(0, |x| x.0 + 1);
    Ok(json!({
        "level": "nfl",
        "team": views::team_brief(w, TeamRef::Nfl(t)),
        "head": format!(
            "{} NFL season · {}after week {last_week} · the {} playoff picture",
            race.year,
            if prev { "last season, " } else { "" },
            CONF[conf]
        ),
        "tabs": tabs,
        "tab": tab,
        "hero": hero,
        "chart": {
            "cols": cols,
            "depth": teams.len(),
            "cut": cut,
            "out": Value::Null,
            "lines": lines,
            "hls": [DIV[my_div].to_string(), "In the hunt".to_string(), "All".to_string()],
            "poll": "playoff picture",
            "date": cols.last(),
        },
        "moves": lettered(moves),
        "next": next,
        "note": note,
        "table": table,
    }))
}

// ---------------------------------------------------------------------------
// High school.

fn high_school(w: &World, h: usize, tab: Option<&str>) -> Reply {
    // This season's polls kept for this school (a move mid-year leaves the
    // old school's behind).
    let st = w.high_schools[h].state;
    let polls: Vec<&StatePoll> = w
        .state_polls()
        .iter()
        .filter(|p| p.year == w.year)
        .filter(|p| p.state.is_none_or(|s| s as usize == st) && p.school.is_none_or(|x| x as usize == h))
        .collect();
    let class = views::hs_class(w, h);
    let state = w
        .states
        .get(w.high_schools[h].state)
        .map_or("the state".to_string(), |s| afm_engine::world::folk::facts::state_name(&s.code).to_string());
    let tab = match tab {
        Some("state") => "state",
        Some("table") => "table",
        _ => "class",
    };
    let tabs = json!([
        { "k": "class", "n": format!("{class} rankings") },
        { "k": "state", "n": format!("{state} top 25") },
        { "k": "table", "n": "Your league" },
    ]);
    // The polls publish 25: your own place past that stays off the board.
    let rows_of = |p: &StatePoll| -> Vec<PollRow> {
        let rows = if tab == "state" { &p.top } else { &p.class };
        rows.iter().filter(|r| r.rank <= 25).cloned().collect()
    };
    let rank_of =
        |rows: &[PollRow], x: usize| rows.iter().find(|r| r.school as usize == x).map_or(0, |r| r.rank as usize);
    let in25 = |rows: &[PollRow], x: usize| {
        let r = rank_of(rows, x);
        if r > 25 { 0 } else { r }
    };
    let name = |x: usize| views::team_name(w, TeamRef::HighSchool(x));
    let last = polls.last().copied();
    let my_class = last.map_or(0, |p| rank_of(&p.class, h));
    let my_state = last.map_or(0, |p| rank_of(&p.top, h));
    let standings = w.hs_standings(h);
    let rec = standings
        .as_ref()
        .and_then(|(_, rows)| rows.iter().find(|r| r.0 == h).map(|r| format!("{}-{}", r.1, r.2)))
        .unwrap_or_else(|| "0-0".into());
    let hero = json!([
        { "k": "Class", "v": if my_class > 0 && my_class <= 25 { format!("{} in {class}", ord(my_class)) } else { "NR".into() } },
        { "k": "State", "v": if my_state > 0 && my_state <= 25 { format!("{} in {state}", ord(my_state)) } else { "NR".into() } },
        { "k": "Record", "v": rec },
        { "k": "Poll", "v": last.map_or("none yet".to_string(), |p| format!("{} of {}", p.which.to_lowercase(), short(p.date))) },
    ]);
    let table = standings.as_ref().map(|(n, rows)| {
        json!({
            "title": n,
            "cols": ["Record", "Class rank"],
            "rows": rows.iter().enumerate().map(|(k, r)| json!({
                "place": k + 1,
                "t": views::team_brief(w, TeamRef::HighSchool(r.0)),
                "me": r.0 == h,
                "cells": [format!("{}-{}", r.1, r.2), last.map_or("—".to_string(), |p| { let x = rank_of(&p.class, r.0); if x > 0 && x <= 25 { x.to_string() } else { "—".into() } })],
            })).collect::<Vec<_>>(),
        })
    });
    // Your games still to play.
    let mut left: Vec<_> = w.hs_fixtures(h).into_iter().filter(|f| f.date >= w.today).collect();
    left.sort_by_key(|f| f.date);
    let next: Vec<Value> = left
        .iter()
        .take(4)
        .map(|f| {
            let home = f.home == h;
            let o = if home { f.away } else { f.home };
            let r = last.map_or(0, |p| in25(&p.class, o));
            let same = w.high_schools[o].classification == w.high_schools[h].classification;
            let words = match r {
                0 if my_class > 0 && my_class <= 25 => "unranked: little to gain, a lot to lose",
                0 => "a win keeps you in view",
                r if my_class == 0 || r < my_class => "a chance to climb",
                _ => "a ranked win",
            };
            json!({
                "d": short(f.date),
                "o": format!("{}{}", if home { "" } else { "at " }, name(o)),
                "t": views::team_brief(w, TeamRef::HighSchool(o)),
                "r": if r > 0 { format!("No. {r}") } else { "—".into() },
                "s": if same { words.to_string() } else { format!("{words} (another class)") },
            })
        })
        .collect();
    let head_base = format!("{} {state} high school football", w.year);
    let Some(last) = last.filter(|_| tab != "table") else {
        return Ok(json!({
            "level": "hs", "team": views::team_brief(w, TeamRef::HighSchool(h)),
            "head": format!("{head_base} · {}", if polls.is_empty() { "no poll yet: the preseason poll comes out the day before the first games" } else { "your league" }),
            "tabs": tabs, "tab": if polls.is_empty() { "table" } else { tab }, "hero": hero, "table": table, "next": next,
        }));
    };
    let mut weekly = 0;
    let cols: Vec<String> = polls
        .iter()
        .map(|p| match p.which.as_str() {
            "Preseason" => "PRE".to_string(),
            "Final" => "FINAL".to_string(),
            _ => {
                weekly += 1;
                format!("WK {weekly}")
            }
        })
        .collect();
    let last_rows = rows_of(last);
    let mut set: Vec<usize> = last_rows.iter().filter(|r| r.rank <= 25).map(|r| r.school as usize).collect();
    for p in polls.iter().rev().skip(1).take(2) {
        for r in rows_of(p).iter().filter(|r| r.rank <= 25) {
            if !set.contains(&(r.school as usize)) {
                set.push(r.school as usize);
            }
        }
    }
    if !set.contains(&h) {
        set.push(h);
    }
    let all: Vec<Vec<PollRow>> = polls.iter().map(|p| rows_of(p)).collect();
    let league: Vec<usize> = standings.as_ref().map_or(Vec::new(), |(_, rows)| rows.iter().map(|r| r.0).collect());
    let lines: Vec<Value> = set
        .iter()
        .map(|&x| {
            let r = last_rows.iter().find(|r| r.school as usize == x);
            json!({
                "id": x,
                "t": views::team_brief(w, TeamRef::HighSchool(x)),
                "name": name(x),
                "rec": r.map_or(String::new(), |r| format!("{}-{}", r.wins, r.losses)),
                "grp": views::hs_class(w, x),
                "hl": if tab == "state" {
                    w.high_schools[x].classification == w.high_schools[h].classification
                } else {
                    league.contains(&x)
                },
                "me": x == h,
                "r": all.iter().map(|rows| in25(rows, x)).collect::<Vec<_>>(),
                "true": r.map(|r| r.rank),
            })
        })
        .collect();
    // Why you moved, poll to poll: your own games between them.
    let mine = w.seat_team().map(|x| x.1);
    let mut moves: Vec<Value> = Vec::new();
    for k in 1..polls.len() {
        let (a, b) = (polls[k - 1], polls[k]);
        let (ra, rb) = (&all[k - 1], &all[k]);
        let (rp, rc) = (in25(ra, h), in25(rb, h));
        let games: Vec<(usize, Res)> = w
            .seat_games()
            .iter()
            .enumerate()
            .filter(|(_, g)| g.date > a.date && g.date <= b.date)
            .filter_map(|(gi, g)| {
                let us = g.teams.iter().position(|&x| Some(x) == mine)?;
                Some((
                    gi,
                    Res {
                        date: g.date,
                        opp: g.teams[1 - us],
                        home: us == 0,
                        us: g.stats.score[us].max(0) as u32,
                        them: g.stats.score[1 - us].max(0) as u32,
                    },
                ))
            })
            .collect();
        if games.is_empty() && rp == rc {
            continue;
        }
        let r = if games.is_empty() {
            "No game".to_string()
        } else {
            games.iter().map(|(_, g)| g.text(&name(g.opp), in25(ra, g.opp))).collect::<Vec<_>>().join(", ")
        };
        // Schools ranked above you that lost between the polls.
        let my_prev = rank_of(ra, h);
        let fell = ra
            .iter()
            .filter(|x| (x.rank as usize) < my_prev.max(1) || my_prev == 0)
            .filter(|x| x.school as usize != h)
            .filter(|x| rb.iter().find(|y| y.school == x.school).is_some_and(|y| y.losses > x.losses))
            .count();
        let won = games.last().is_some_and(|(_, g)| g.won());
        let why = match (games.is_empty(), won, rc.cmp(&rp)) {
            _ if rp == 0 && rc == 0 => {
                "Outside the top 25: the writers rank by record first, then margins and what they know of a school."
                    .to_string()
            }
            (true, _, _) => "No game this week; the schools around you played.".into(),
            (_, true, _) if rp == 0 => "Into the top 25 after the win.".into(),
            (_, true, _) if rc == 0 => {
                "Won, but out of the top 25: schools with better records or bigger wins passed you.".into()
            }
            (_, true, Ordering::Less) if fell > 0 => {
                format!("{fell} {} above you lost.", if fell == 1 { "school" } else { "schools" })
            }
            (_, true, Ordering::Less) => "The win moved you past schools with weaker wins.".into(),
            (_, true, Ordering::Equal) => "Every school just above you won too.".into(),
            (_, true, Ordering::Greater) => "Schools below you won bigger games and passed you.".into(),
            (_, false, _) if rc == 0 => "Out of the top 25 after the loss.".into(),
            (_, false, Ordering::Greater) => "The loss put schools with fewer losses ahead of you.".into(),
            (_, false, _) => "Schools above you lost too.".into(),
        };
        moves.push(json!({
            "col": k,
            "r": r,
            "rank": if rc > 0 { rc.to_string() } else { "NR".into() },
            "ch": change(rp, rc),
            "why": why,
            "game": games.last().map(|x| x.0),
        }));
    }
    Ok(json!({
        "level": "hs",
        "team": views::team_brief(w, TeamRef::HighSchool(h)),
        "head": format!("{head_base} · {} poll of {}", last.which.to_lowercase(), short(last.date)),
        "tabs": tabs,
        "tab": tab,
        "hero": hero,
        "chart": {
            "cols": cols,
            "depth": 25,
            "cut": Value::Null,
            "out": "NR",
            "lines": lines,
            "hls": [if tab == "state" { format!("{class} schools") } else { "Your league".to_string() }, "Top 10".to_string(), "All".to_string()],
            "poll": if tab == "state" { format!("{state} top 25") } else { format!("{class} rankings") },
            "date": short(last.date),
        },
        "moves": lettered(moves),
        "next": next,
        "table": table,
    }))
}
