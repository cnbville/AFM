//! The pages' data beyond the first playable's views: Home (9a-9e), the
//! player page (S1-S3), hover cards (L5), search across everything (S6),
//! compare, the history books, playoff brackets, news feeds with their
//! certainty and story chains, the coach page, your career page and the
//! team page. Every line of text that names a team or a person carries the
//! link markup (`links`), and nothing hidden is shown.

use std::collections::HashMap;

use afm_engine::body::body_now;
use afm_engine::calendar::{Date, Event, Kind, Level as CalLevel, Loudness};
use afm_engine::player::Level;
use afm_engine::positions::Position;
use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::desk::Head;
use afm_engine::world::records::{Book, RECORD_STATS, STREAKS, Scope};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::links::{self, Linker};
use super::mail::{self, tidy};
use super::{desk, people, tables};
use crate::reads;
use crate::views::{self, TeamRef};

fn d_text(d: Date) -> String {
    views::date_text(d)
}

/// Events in a range that a filter keeps, newest first.
fn events_in(w: &World, from: Date, to: Date, keep: impl Fn(&Event) -> bool) -> Vec<(Date, &Event)> {
    let mut v: Vec<(Date, &Event)> = w
        .calendar
        .events
        .range(from..=to)
        .flat_map(|(d, evs)| evs.iter().map(move |e| (*d, e)))
        .filter(|(_, e)| keep(e))
        .collect();
    v.reverse();
    v
}

/// What kind of scene a story's poster draws (9c).
fn scene(e: &Event) -> &'static str {
    let t = e.text.as_str();
    match e.kind {
        Kind::Game if t.contains(" beat ") => "game",
        Kind::Game => "game",
        Kind::Commitment | Kind::Signing | Kind::Offer | Kind::Visit => "recruit",
        Kind::Transfer => "portal",
        Kind::Coaching => "coach",
        Kind::Award | Kind::Ceremony => "award",
        Kind::DraftPick | Kind::DraftChoice => "draft",
        Kind::Weather => "weather",
        Kind::Poll => "poll",
        Kind::RuleChange => "office",
        Kind::Breakout => "rise",
        Kind::Practice => "practice",
        _ if t.contains("injur") => "injury",
        _ if t.contains("record") => "record",
        _ => "news",
    }
}

fn kicker(e: &Event) -> &'static str {
    match e.kind {
        Kind::Game => "Result",
        Kind::Commitment => "Commitment",
        Kind::Signing => "Signing",
        Kind::Offer => "Offer",
        Kind::Visit => "Visit",
        Kind::Transfer => "Portal",
        Kind::Coaching => "Coaching",
        Kind::Award => "Honors",
        Kind::Ceremony => "Ceremony",
        Kind::DraftPick | Kind::DraftChoice => "Draft",
        Kind::Weather => "Weather",
        Kind::Poll => "Poll",
        Kind::RuleChange => "League office",
        Kind::Breakout => "Rising",
        Kind::Practice => "Practice",
        Kind::Whisper => "Word is",
        Kind::Window | Kind::Deadline => "Calendar",
        // Your own meetings, talks and pressers (people's dated slots) and
        // the week's rhythm.
        Kind::Slot => "Your day",
        Kind::Rhythm => "The week",
        _ => "News",
    }
}

// ---- Home (9a-9e) ----

pub fn dashboard(w: &World) -> Value {
    let me = views::mine(w);
    let status = views::status(w);
    let Some(me) = me else { return json!({ "status": status }) };
    let colors = views::team_colors(w, me);
    let team_name = views::team_name(w, me);
    let rec = views::record_text(w, me);
    let yours = views::yours_test(w);
    let rivals = mail::rivals(w, me);
    let today = w.today;

    // The lines Home quotes, linked once.
    let recent = events_in(w, today.add(-21), today, |e| e.kind != Kind::Slot && e.kind != Kind::Rhythm);
    let texts: Vec<&str> = recent.iter().map(|(_, e)| e.text.as_str()).collect();
    let linker = Linker::new(w, &texts, matches!(me, TeamRef::HighSchool(_)));

    // 9a: the hero's slides.
    let mut slides = Vec::new();
    if let Some(g) = views::next_game(w, me) {
        let opp = &g["opp"];
        let home = g["home"].as_bool().unwrap_or(false);
        let days = g["in"].as_i64().unwrap_or(0);
        slides.push(json!({
            "kind": "game",
            "kicker": if days == 0 { "Game day".to_string() } else { format!("Next game · in {days} day{}", if days == 1 { "" } else { "s" }) },
            "title": format!("{} {}", if home { "vs" } else { "at" }, opp["name"].as_str().unwrap_or("")),
            "sub": format!("{} · {}", g["date"].as_str().unwrap_or(""), g["name"].as_str().unwrap_or(if home { "Home" } else { "Away" })),
            "colors": opp["colors"],
            "go": { "screen": "team", "args": opp["ref"], "label": "Scout them" },
            "stat": rec,
            "stat_label": views::record_note(w, me),
        }));
    }
    let last_game = w.seat_games().iter().enumerate().next_back();
    if let Some((k, g)) = last_game {
        let side = usize::from(match me {
            TeamRef::College(i) | TeamRef::Nfl(i) | TeamRef::HighSchool(i) | TeamRef::Other(i) => g.teams[0] != i,
        });
        let (us, them) = (g.stats.score[side], g.stats.score[1 - side]);
        slides.push(json!({
            "kind": "result",
            "kicker": format!("Last game · {}", d_text(g.date)),
            "title": format!("{} {us}, {} {them}", g.names[side], g.names[1 - side]),
            "sub": if us > them { "A win. The film is ready." } else if us < them { "A loss. The film is ready." } else { "A tie." },
            "colors": colors,
            "go": { "screen": "game", "args": { "index": k }, "label": "Game page" },
            "stat": format!("{us}-{them}"),
            "stat_label": if us > them { "Won" } else { "Lost" },
        }));
    }
    for (d, e) in recent.iter().filter(|(_, e)| yours(e) && e.loud >= Loudness::Medium).take(2) {
        slides.push(json!({
            "kind": scene(e),
            "kicker": format!("{} · {}", kicker(e), d_text(*d)),
            "title": tidy(&linker.event(e)),
            "sub": team_name,
            "colors": colors,
        }));
    }
    if let TeamRef::College(p) = me {
        let class: Vec<usize> = w
            .people
            .iter()
            .enumerate()
            .filter(|(_, x)| x.cal.commit.is_some_and(|c| c.0 == p) && x.stage == Stage::HighSchool)
            .map(|(k, _)| k)
            .collect();
        if !class.is_empty() {
            let stars: f64 = class.iter().map(|&k| w.people[k].stars as f64).sum::<f64>() / class.len() as f64;
            slides.push(json!({
                "kind": "recruit",
                "kicker": format!("Class of {}", w.year + 1),
                "title": format!("{} {}", class.len(), afm_engine::world::folk::noun(class.len(), "commitment")),
                "sub": format!("Average {stars:.2} stars"),
                "colors": colors,
                "go": { "screen": "recruiting", "args": {}, "label": "Recruiting board" },
                "stat": class.len(),
                "stat_label": "Committed",
            }));
        }
    }
    slides.push(json!({
        "kind": "team",
        "kicker": "Your team",
        "title": team_name,
        "sub": views::team_line(w, me),
        "colors": colors,
        "go": { "screen": "team", "args": me.json(), "label": "Team page" },
        "stat": rec,
        "stat_label": views::record_note(w, me),
    }));

    // 9b: tracked players.
    let tracked = desk::my_players(w)["rows"].as_array().cloned().unwrap_or_default();

    // 9c: storylines as posters: your team, your rivals, your people.
    let rival_names: Vec<String> = rivals.iter().map(|&t| views::team_name(w, t)).collect();
    let mut stories = Vec::new();
    for (d, e) in &recent {
        if stories.len() >= 4 {
            break;
        }
        let about = yours(e) || rival_names.iter().any(|n| e.text.contains(n.as_str()));
        if !about || e.loud < Loudness::Medium && !yours(e) {
            continue;
        }
        stories.push(json!({
            "scene": scene(e),
            "kicker": kicker(e),
            "date": d_text(*d),
            "text": tidy(&linker.event(e)),
            "loud": e.loud.name(),
            "mine": yours(e),
        }));
    }

    // 9d: the next seven days and the boss's latest word.
    let seat = w.seat;
    let mut week: Vec<Value> = events_in(w, today, today.add(7), |e| {
        yours(e) || (e.loud >= Loudness::Medium && seat.is_none_or(|s| s.cares(e)))
    })
    .into_iter()
    .rev()
    .take(10)
    .map(|(d, e)| json!({ "date": d_text(d), "in": today.days_to(d), "text": tidy(&linker.event(e)), "mine": yours(e), "kind": kicker(e) }))
    .collect();
    if let Some(g) = views::next_game(w, me)
        && g["in"].as_i64().is_some_and(|n| n <= 7)
    {
        week.insert(
            0,
            json!({ "date": g["date"], "in": g["in"], "text": format!("{} {}", if g["home"].as_bool().unwrap_or(false) { "vs" } else { "at" }, links::team_as(TeamRef::from_args(w, &g["opp"]["ref"]).unwrap_or(me), g["opp"]["name"].as_str().unwrap_or(""))), "mine": true, "kind": "Game" }),
        );
    }
    let (boss_name, boss_role) = people::boss(w, me);
    let boss_mail = w.career.as_ref().and_then(|c| {
        c.desk.mails.iter().rev().find(|m| m.from == boss_name && matches!(m.head, Head::School | Head::Club))
    });
    let boss = json!({
        "name": boss_name,
        "role": boss_role,
        "subject": boss_mail.map(|m| m.subject.clone()),
        "line": boss_mail.and_then(|m| m.body.iter().find(|l| l.len() > 40).map(|l| links::strip(l))),
        "mail": boss_mail.map(|m| m.id.to_string()),
        "date": boss_mail.map(|m| d_text(m.date)),
    });

    // 9e: the team in words, and around football.
    let words = team_words(w, me);
    let around: Vec<Value> = recent
        .iter()
        .filter(|(_, e)| {
            !yours(e) && (e.loud == Loudness::Huge || (e.loud >= Loudness::Big && seat.is_none_or(|s| s.cares(e))))
        })
        .take(8)
        .map(|(d, e)| json!({ "date": d_text(*d), "text": tidy(&linker.event(e)), "kind": kicker(e) }))
        .collect();
    let mail_rows = desk::list(w, &json!({}))["rows"]
        .as_array()
        .map(|r| r.iter().take(6).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "status": status,
        "team": { "name": team_name, "colors": colors, "ref": me.json(), "record": rec, "line": views::team_line(w, me) },
        "slides": slides,
        "tracked": tracked,
        "stories": stories,
        "week": week,
        "boss": boss,
        "words": words,
        "around": around,
        "mail": mail_rows,
    })
}

/// Your team in a few sentences (9e): record, streak, last result,
/// scoring, and where it stands.
fn team_words(w: &World, t: TeamRef) -> Vec<String> {
    let mut out = Vec::new();
    let games = w.seat_games();
    let side = |g: &afm_engine::world::seat::SeatGame| match t {
        TeamRef::College(i) | TeamRef::Nfl(i) | TeamRef::HighSchool(i) | TeamRef::Other(i) => {
            usize::from(g.teams[0] != i)
        }
    };
    let season: Vec<_> =
        games.iter().filter(|g| g.date.year() == w.today.year() || g.date.days_to(w.today) < 200).collect();
    if season.is_empty() {
        let note = views::record_note(w, t);
        out.push(if note.starts_with("in ") {
            format!("No games yet this season: {} {note}.", views::record_text(w, t))
        } else {
            "No games yet this season.".to_string()
        });
    } else {
        let (mut pf, mut pa, mut won) = (0i32, 0i32, 0u32);
        for g in &season {
            let s = side(g);
            pf += g.stats.score[s];
            pa += g.stats.score[1 - s];
            won += u32::from(g.stats.score[s] > g.stats.score[1 - s]);
        }
        let n = season.len() as f64;
        out.push(format!(
            "{won}-{} in the games we have film on, scoring {:.1} a game and allowing {:.1}.",
            season.len() as u32 - won,
            pf as f64 / n,
            pa as f64 / n
        ));
        let mut streak = 0;
        let mut kind = None;
        for g in games.iter().rev() {
            let s = side(g);
            let win = g.stats.score[s] > g.stats.score[1 - s];
            if kind.is_none_or(|k| k == win) {
                kind = Some(win);
                streak += 1;
            } else {
                break;
            }
        }
        if streak >= 2 {
            out.push(format!("{} straight {}.", streak, if kind == Some(true) { "wins" } else { "losses" }));
        }
    }
    if let TeamRef::College(p) = t {
        let prog = &w.programs[p];
        out.push(format!(
            "{} in the {}; {}.",
            views::record_text(w, t),
            prog.conference,
            views::prestige_words(prog.prestige_rel).to_lowercase()
        ));
    }
    out
}

// ---- The player page (S1-S3) ----

/// The five mental attributes that matter most at a position (S2).
fn key_mental(pos: Position) -> [&'static str; 5] {
    use Position as P;
    match pos {
        P::QB => ["decisions", "anticipation", "composure", "footballIq", "communication"],
        P::RB => ["footballIq", "toughness", "composure", "competitiveness", "concentration"],
        P::WR | P::TE => ["concentration", "competitiveness", "composure", "footballIq", "physicality"],
        P::OT | P::IOL => ["footballIq", "communication", "discipline", "toughness", "physicality"],
        P::EDGE | P::DT => ["motor", "playRecognition", "physicality", "toughness", "discipline"],
        P::LB => ["playRecognition", "footballIq", "communication", "toughness", "motor"],
        P::CB | P::S => ["anticipation", "playRecognition", "concentration", "discipline", "composure"],
        P::K | P::P | P::LS => ["composure", "concentration", "discipline", "competitiveness", "footballIq"],
    }
}

fn level_name(l: Level) -> &'static str {
    match l {
        Level::HS => "High school",
        Level::FBS => "FBS",
        Level::FCS => "FCS",
        Level::SmallCollege => "Small college",
        Level::NFL => "NFL",
        #[allow(unreachable_patterns)]
        _ => "Other",
    }
}

/// "Senior level 89", "FBS level 73": his level badge (minor bug 55).
fn level_rating(w: &World, k: usize) -> Option<String> {
    let (r, words) = level_read(w, k)?;
    Some(format!("{words} level {r}"))
}

/// His level badge from your staff's read (minor bugs 49-52): your staff's
/// overall now placed on the scale of his newest season's group (his class
/// at high school, every FBS player in college), in that season's words.
/// Nothing for a pro or a player nobody of yours has seen.
pub(crate) fn level_read(w: &World, k: usize) -> Option<(u8, &'static str)> {
    let p = w.people.get(k)?;
    let last = p.player.history.len().checked_sub(1)?;
    let centre = reads::seen(w, k).now?.0;
    afm_engine::world::level_from_read(w, k, last, centre)
}

/// The badge's marks for its hover (minor bug 56): what each level means
/// at his pool, from the scale's own anchors.
pub(crate) fn level_marks(w: &World, k: usize) -> Value {
    let Some(h) = w.people.get(k).and_then(|p| p.player.history.last()) else { return Value::Null };
    match afm_engine::world::level_pool(h.level) {
        Some(pool) => {
            json!(afm_engine::level_rating::marks(pool).into_iter().map(|(t, r)| json!([t, r])).collect::<Vec<_>>())
        }
        None => Value::Null,
    }
}

/// Measurables with where each was taken (S2).
/// Inches to the nearest eighth, as the combine writes them (32 3/8").
pub(crate) fn eighths(x: f64) -> String {
    let e = (x * 8.0).round() as i64;
    let (whole, part) = (e / 8, e % 8);
    match part {
        0 => format!("{whole}\""),
        4 => format!("{whole} 1/2\""),
        2 | 6 => format!("{whole} {}/4\"", part / 2),
        _ => format!("{whole} {part}/8\""),
    }
}

fn measurables(w: &World, k: usize) -> Vec<Value> {
    let p = &w.people[k];
    let b = &afm_engine::body::body_now(&p.player);
    let at = match p.stage {
        Stage::HighSchool | Stage::PrepSchool => format!("{} roster, school-listed", w.school_name(p.hs)),
        Stage::College => format!("{} spring measurements", p.college.map_or("Team", |c| w.program_name(c))),
        Stage::Away => format!("{} roster, school-listed, before he left", w.school_name(p.hs)),
        Stage::Declared | Stage::Pro | Stage::Done => {
            if p.pro.as_ref().and_then(|c| c.draft).is_some() || p.cal.combine_invite {
                "NFL combine, Indianapolis".to_string()
            } else {
                "Pro day".to_string()
            }
        }
    };
    // Minor bug 24: a high schooler's page shows what his school lists,
    // not his true body.
    let (ht, wt) = if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool | Stage::Away) {
        afm_engine::world::scouting::file::listed(&p.player)
    } else {
        (b.height, b.weight)
    };
    let mut v = vec![
        json!({ "label": "Height", "value": views::ft_in(ht), "at": at }),
        json!({ "label": "Weight", "value": format!("{:.0} lb", wt), "at": at }),
    ];
    // His recorded times (speed update S6e): his best at each event, as
    // MileSplit and the camps print them, with how each was timed.
    v.extend(times_rows(w, p));
    if matches!(p.stage, Stage::College | Stage::Declared | Stage::Pro | Stage::Done) {
        v.push(json!({ "label": "Arm", "value": eighths(b.arm), "at": at }));
        v.push(json!({ "label": "Hand", "value": eighths(b.hand), "at": at }));
        v.push(json!({ "label": "Wingspan", "value": eighths(b.wingspan), "at": at }));
    }
    let tested = matches!(p.stage, Stage::Declared | Stage::Pro | Stage::Done)
        && (p.cal.combine_invite || p.pro.as_ref().is_some_and(|c| c.draft.is_some()));
    if tested {
        let mut rng = afm_engine::rng::Rng::new(p.player.id ^ 0x00C0_FFEE);
        let c = afm_engine::body::combine(&p.player, &mut rng);
        v.push(json!({ "label": "40-yard dash", "value": format!("{:.2} s", c.forty), "at": at }));
        v.push(json!({ "label": "Bench (225)", "value": format!("{:.0} reps", c.bench), "at": at }));
        v.push(json!({ "label": "Vertical", "value": format!("{:.1} in", c.vertical), "at": at }));
        v.push(json!({ "label": "Broad jump", "value": format!("{}'{:.0}\"", (c.broad / 12.0) as u32, c.broad % 12.0), "at": at }));
        v.push(json!({ "label": "3-cone", "value": format!("{:.2} s", c.cone), "at": at }));
        v.push(json!({ "label": "Shuttle", "value": format!("{:.2} s", c.shuttle), "at": at }));
    }
    v
}

/// The height and weight a screen shows (minor bug 24): his true body for
/// your own players and anyone past high school, an outside high
/// schooler's school listing until your staff has measured him.
pub(crate) fn shown_body(w: &World, p: &afm_engine::world::Person, own: bool) -> (f64, f64) {
    let b = body_now(&p.player);
    if own || !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) || crate::reads::hs_tested(w, p.player.id) {
        (b.height, b.weight)
    } else {
        afm_engine::world::scouting::file::listed(&p.player)
    }
}

/// A high schooler's best recorded time at each event (the forty and its
/// split from a laser, the 100m and 200m from the track), where and how.
pub(crate) fn times_rows(w: &World, p: &afm_engine::world::Person) -> Vec<Value> {
    use afm_engine::world::times::{At, Event, Timed};
    let all = w.times.of(p.player.id);
    let mut out = Vec::new();
    for (ev, label) in [
        (Event::Forty, "40-yard dash"),
        (Event::Split, "10-yard split"),
        (Event::M100, "100 m"),
        (Event::M200, "200 m"),
    ] {
        // Electronic first; a hand time only when there is nothing else.
        let best = |hand: bool| {
            all.iter()
                .filter(|t| t.event == ev && (t.timed == Timed::Hand) == hand)
                .min_by(|a, b| a.secs.total_cmp(&b.secs).then(b.date.0.cmp(&a.date.0)))
        };
        let Some(t) = best(false).or_else(|| best(true)) else { continue };
        let place = match t.at {
            At::Meet => "Track meet".to_string(),
            At::StateMeet => "State meet".to_string(),
            At::Camp(j) => format!("{} camp", w.program_name(j as usize)),
            At::Regional => "Regional showcase".to_string(),
            At::National => "National showcase finals".to_string(),
            At::School => "School-listed".to_string(),
        };
        let how = match t.timed {
            Timed::Laser => "laser",
            Timed::Fat => "fully automatic",
            Timed::Hand => "hand-timed",
        };
        let dec = if t.timed == Timed::Hand && matches!(ev, Event::M100 | Event::M200) { 1 } else { 2 };
        out.push(json!({
            "label": label,
            "value": format!("{:.*} s", dec, t.secs),
            "at": format!("{place}, {how}, {}", views::date_text(t.date)),
        }));
    }
    out
}

/// Every season's stat line he has: high school, college, NFL.
pub(crate) fn seasons(w: &World, k: usize) -> Vec<Value> {
    let p = &w.people[k];
    let id = p.player.id;
    let mut v = Vec::new();
    for l in &p.hs_lines {
        let g = |s: afm_engine::world::records::Stat| {
            use afm_engine::world::hs_year::HsStat as H;
            use afm_engine::world::records::Stat as S;
            let h = match s {
                S::Att => H::Att,
                S::Cmp => H::Cmp,
                S::PassYds => H::PassYds,
                S::PassTd => H::PassTd,
                S::Int => H::Int,
                S::Rush => H::Rush,
                S::RushYds => H::RushYds,
                S::RushTd => H::RushTd,
                S::Rec => H::Rec,
                S::RecYds => H::RecYds,
                S::RecTd => H::RecTd,
                S::Tackles => H::Tackles,
                S::Tfl => H::Tfl,
                S::Sacks => H::Sacks,
                S::Ints => H::Ints,
                S::Pbus => H::Pbus,
                S::FgMade => H::FgMade,
                S::FgAtt => H::FgAtt,
                _ => return 0.0,
            };
            l.get(h)
        };
        v.push(json!({
            "year": l.year, "level": "High school",
            "team": views::team_brief(w, TeamRef::HighSchool(l.school as usize)),
            "pos": l.pos.name(), "games": l.games, "line": mail::stat_line(l.pos, g),
        }));
    }
    for r in w.stats.career(id) {
        v.push(json!({
            "year": r.year, "level": r.division.name(),
            "team": views::team_brief(w, TeamRef::College(r.program as usize)),
            "pos": r.pos.name(), "games": r.games, "line": mail::stat_line(r.pos, |s| r.get(s)),
            "all_america": w.stats.honors.get(&id).is_some_and(|h| h.all_america.iter().any(|a| a.0 == r.year as i32)),
        }));
    }
    if let Some(c) = &p.pro {
        for s in &c.seasons {
            let line = afm_engine::world::records::from_line(&s.stats);
            v.push(json!({
                "year": s.year, "level": "NFL",
                "team": s.team.map(|t| views::team_brief(w, TeamRef::Nfl(t))),
                "pos": p.player.position.name(), "games": s.games, "starts": s.starts,
                "line": if s.games > 0 { mail::stat_line(p.player.position, |k| line[k as usize] as f64) } else { "Did not play".to_string() },
                "pro_bowl": s.pro_bowl, "all_pro": s.all_pro, "awards": s.awards,
            }));
        }
    }
    // Seasons in the other leagues (the CFL, arena football, the spring
    // leagues), in the order played.
    let others = crate::lane_money::other_career_rows(w, k);
    if !others.is_empty() {
        v.extend(others);
        v.sort_by_key(|r| r["year"].as_i64().unwrap_or(0));
    }
    v
}

/// His lines in your kept games (the game logs and splits, S3).
fn game_logs(w: &World, id: u32) -> (Vec<Value>, Vec<Value>) {
    let mut logs = Vec::new();
    let (mut home, mut away) = ([0f64; 4], [0f64; 4]);
    for (k, g) in w.seat_games().iter().enumerate() {
        for side in 0..2 {
            let Some(l) = g.stats.players[side].get(&id) else { continue };
            let opp = &g.names[1 - side];
            let (us, them) = (g.stats.score[side], g.stats.score[1 - side]);
            let mut parts = Vec::new();
            if l.att > 0 {
                parts.push(format!("{}/{}, {:.0} yds, {} TD, {} INT", l.cmp, l.att, l.pass_yds, l.pass_td, l.int));
            }
            if l.rush > 0 {
                parts.push(format!("{} car, {:.0} yds, {} TD", l.rush, l.rush_yds, l.rush_td));
            }
            if l.targets > 0 {
                parts.push(format!("{} rec, {:.0} yds, {} TD", l.rec, l.rec_yds, l.rec_td));
            }
            if l.tackles + l.assists > 0 || l.sacks > 0.0 || l.ints > 0 {
                parts.push(format!("{} tkl, {:.1} sk, {} INT", l.tackles + l.assists, l.sacks, l.ints));
            }
            let yards = l.pass_yds + l.rush_yds + l.rec_yds;
            let tds = (l.pass_td + l.rush_td + l.rec_td) as f64;
            let split = if side == 0 { &mut home } else { &mut away };
            split[0] += 1.0;
            split[1] += yards;
            split[2] += tds;
            split[3] += (l.tackles + l.assists) as f64;
            logs.push(json!({
                "index": k, "date": d_text(g.date), "opp": format!("{} {opp}", if side == 0 { "vs" } else { "at" }),
                "result": format!("{} {us}-{them}", if us > them { "W" } else if us < them { "L" } else { "T" }),
                "line": if parts.is_empty() { "Played".to_string() } else { parts.join(" · ") },
            }));
        }
    }
    logs.reverse();
    let split = |name: &str, s: [f64; 4]| json!({ "name": name, "games": s[0], "yards": if s[0] > 0.0 { s[1] / s[0] } else { 0.0 }, "td": s[2], "tackles": if s[0] > 0.0 { s[3] / s[0] } else { 0.0 } });
    let splits = if logs.is_empty() { Vec::new() } else { vec![split("Home", home), split("Away", away)] };
    (logs, splits)
}

/// The seasons he played for you, and the contacts you logged (S3).
fn with_you(w: &World, k: usize) -> Value {
    let Some(c) = &w.career else { return Value::Null };
    let p = &w.people[k];
    let id = p.player.id;
    let mut seasons: Vec<Value> = p
        .college_staff
        .iter()
        .filter(|s| s.2 == Some(c.coach))
        .map(|s| json!({ "year": s.0, "team": views::team_brief(w, TeamRef::College(s.1)) }))
        .collect();
    if let Some(pro) = &p.pro
        && let Some(TeamRef::Nfl(t)) = views::mine(w)
    {
        seasons.extend(
            pro.seasons
                .iter()
                .filter(|s| s.team == Some(t) && s.year >= c.started.year())
                .map(|s| json!({ "year": s.year, "team": views::team_brief(w, TeamRef::Nfl(t)) })),
        );
    }
    let contacts: Vec<Value> = c
        .desk
        .contacts
        .iter()
        .rev()
        .filter(|x| x.kind == 0 && x.id == id)
        .take(20)
        .map(|x| json!({ "date": d_text(x.date), "what": x.what }))
        .collect();
    json!({ "seasons": seasons, "contacts": contacts, "listed": desk::is_listed(w, id) })
}

/// His recruitment (S3): stars, offers with their days, visits, the commit.
fn recruiting(w: &World, k: usize) -> Value {
    let p = &w.people[k];
    let cal = &p.cal;
    let mut offers: Vec<Value> = cal
        .early_offers
        .iter()
        .map(|&(j, d)| json!({ "team": views::team_brief(w, TeamRef::College(j)), "date": d_text(d) }))
        .collect();
    for &o in &cal.offers {
        if !cal.early_offers.iter().any(|x| x.0 == o) {
            offers.push(json!({ "team": views::team_brief(w, TeamRef::College(o)), "date": Value::Null }));
        }
    }
    let visits: Vec<Value> = cal
        .visits
        .iter()
        .map(|&(j, d, v)| {
            json!({ "team": views::team_brief(w, TeamRef::College(j)), "date": d_text(d), "kind": match v {
                afm_engine::world::Visit::Official => "Official visit",
                afm_engine::world::Visit::Unofficial => "Unofficial visit",
                afm_engine::world::Visit::DropIn => "Coach drop-in",
                #[allow(unreachable_patterns)]
                _ => "Contact",
            } })
        })
        .collect();
    json!({
        "stars": p.stars,
        "camp_stars": p.camp_stars,
        "class_of": p.class_of,
        "offers": offers,
        "visits": visits,
        "commit": cal.commit.map(|(j, d, walk)| json!({ "team": views::team_brief(w, TeamRef::College(j)), "date": d_text(d), "walk_on": walk })),
        "signed": cal.signed.map(|s| json!({ "team": views::team_brief(w, TeamRef::College(s.program)), "date": d_text(s.date), "early": s.early_enrollee })),
        "all_state": match cal.all_state { 2 => Some("First-team all-state"), 1 => Some("All-state honorable mention"), _ => None },
        "test": cal.test_score,
    })
}

/// Where else he could play (S2): your staff's read of him at the
/// positions it reads him best at, from his file (minor bug 54); his listed
/// position only while nobody of yours has seen him.
fn position_fit(w: &World, k: usize) -> Vec<Value> {
    let p = &w.people[k];
    let s = reads::seen(w, k);
    if s.positions.is_empty() {
        return vec![json!({ "pos": p.player.position.name(), "read": Value::Null, "now": true })];
    }
    s.positions
        .iter()
        .map(|&(pos, (lo, hi))| json!({ "pos": pos.name(), "read": [lo, hi], "now": pos == p.player.position }))
        .collect()
}

pub fn profile(w: &World, id: u32) -> Result<Value, String> {
    let mut v = views::player(w, id)?;
    let k = views::person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[k];
    let team = views::team_of(w, p);
    let mental: Vec<Value> = key_mental(p.player.position)
        .iter()
        .filter_map(|key| {
            v["groups"]
                .as_array()?
                .iter()
                .flat_map(|g| g["attrs"].as_array().into_iter().flatten())
                .find(|a| a["key"] == *key)
                .cloned()
        })
        .collect();
    let (logs, splits) = game_logs(w, id);
    let honors = w.stats.honors.get(&id).map(|h| {
        json!({
            "awards": h.awards.iter().map(|(y, a)| json!({ "year": y, "award": a })).collect::<Vec<_>>(),
            "all_america": h.all_america.iter().map(|(y, d)| json!({ "year": y, "division": d.name() })).collect::<Vec<_>>(),
            "all_conference": h.all_conference,
            "hall": h.inducted,
        })
    });
    let nfl_awards: Vec<Value> =
        w.nfl_book.awards.iter().filter(|a| a.2 == id).map(|a| json!({ "year": a.0, "award": a.1 })).collect();
    let stories: Vec<String> = p.notes.iter().rev().take(40).map(|s| s.to_string()).collect();
    v["jersey"] = json!(team.and_then(|t| people::jersey(w, t, k)));
    // The name he goes by, wherever he shows (stories, A14).
    v["nick"] = json!(w.nickname_of(id));
    v["level_rating"] = json!(level_rating(w, k));
    v["level_marks"] = level_marks(w, k);
    v["measurables"] = json!(measurables(w, k));
    v["key_mental"] = json!(mental);
    v["fit"] = json!(position_fit(w, k));
    v["seasons"] = json!(seasons(w, k));
    v["game_logs"] = json!(logs);
    v["splits"] = json!(splits);
    v["honors"] = json!(honors);
    v["nfl_awards"] = json!(nfl_awards);
    v["with_you"] = with_you(w, k);
    v["recruiting"] = recruiting(w, k);
    v["stage"] = json!(format!("{:?}", p.stage));
    v["listed"] = json!(desk::is_listed(w, id));
    // Your own college player: what a cut would do (minor bug 60).
    v["college_cut"] = json!(afm_engine::world::front_office::college::cut_words(w, id).ok());
    // Every move that fits him for your seat (minor bugs 62-63).
    v["moves"] = json!(super::moves::player(w, id));
    v["career_story"] = json!(stories);
    v["initials"] = json!(p.name.split_whitespace().filter_map(|s| s.chars().next()).take(2).collect::<String>());
    v["colors"] = match team {
        Some(t) => json!(views::team_colors(w, t)),
        // A club in another league lights the page in its colours.
        None => v["team"]["colors"].clone(),
    };
    v["level_name"] = json!(p.player.history.last().map(|h| level_name(h.level)));
    v["positions_detail"] = json!(
        p.player
            .position_history
            .iter()
            .map(|(age, pos)| json!({ "age": age, "pos": pos.name(), "where": "" }))
            .collect::<Vec<_>>()
    );
    Ok(v)
}

// ---- Hover cards (L5) ----

pub fn peek(w: &World, args: &Value) -> Result<Value, String> {
    let kind = args.get("kind").and_then(Value::as_str).unwrap_or("");
    let num = args.get("id").and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    match kind {
        "player" => {
            let id = num.ok_or("Missing id.")? as u32;
            let k = views::person_index(w, id).ok_or("No longer in football.")?;
            let p = &w.people[k];
            let team = views::team_of(w, p);
            let own = team.is_some_and(|t| views::is_mine(w, t));
            let last =
                w.stats.career(id).last().map(|r| format!("{}: {}", r.year, mail::stat_line(r.pos, |s| r.get(s))));
            let last = last.or_else(|| {
                p.pro.as_ref().and_then(|c| c.seasons.iter().rev().find(|s| s.games > 0)).map(|s| {
                    let line = afm_engine::world::records::from_line(&s.stats);
                    format!("{}: {}", s.year, mail::stat_line(p.player.position, |k| line[k as usize] as f64))
                })
            });
            Ok(json!({
                "kind": "player", "id": id, "name": p.name, "pos": p.player.position.name(), "age": p.player.age,
                "class": views::class_text(w, p), "team": team.map(|t| views::team_brief(w, t)), "where": views::where_now(w, p),
                "ht": views::ft_in(shown_body(w, p, own).0), "wt": shown_body(w, p, own).1.round(), "stars": p.stars,
                "read": reads::overall(w, k).json(), "yours": reads::mine(w, k).json(),
                "level_rating": level_rating(w, k), "level_marks": level_marks(w, k), "last": last, "status": views::status_text(w, p),
                "listed": desk::is_listed(w, id), "noted": desk::has_note(w, id),
                "jersey": team.and_then(|t| people::jersey(w, t, k)),
                "initials": p.name.split_whitespace().filter_map(|s| s.chars().next()).take(2).collect::<String>(),
                "own": own,
                "moves": super::moves::player(w, id),
            }))
        }
        "coach" => {
            let c = num.ok_or("Missing id.")? as usize;
            let v = views::coach(w, c)?;
            Ok(
                json!({ "kind": "coach", "id": c, "name": v["name"], "age": v["age"], "reputation": v["reputation"], "job": v["job"], "titles": v["titles"], "came_up": v["came_up"] }),
            )
        }
        // A team's hover card comes asked by the team's own kind.
        "team" | "nfl" | "college" | "hs" => {
            let t = TeamRef::from_args(w, args)?;
            let mut v = views::team_brief(w, t);
            v["kind"] = json!("team");
            v["record"] = json!(views::record_text(w, t));
            v["line"] = json!(views::team_line(w, t));
            v["coach"] = json!(match t {
                TeamRef::College(p) => w.programs[p].head_coach.map(|c| w.coaches[c].name.clone()),
                TeamRef::HighSchool(h) => w.high_schools[h].coach.map(|c| w.coaches[c].name.clone()),
                TeamRef::Nfl(i) => w
                    .coaches
                    .iter()
                    .find(|c| c.job.is_some_and(|j| j.employer == Employer::Nfl(i) && j.role == Role::HeadCoach))
                    .map(|c| c.name.clone()),
                TeamRef::Other(f) => afm_engine::world::pro::other::head_coach(w, f).map(|c| w.coaches[c].name.clone()),
            });
            v["next"] = views::next_game(w, t).unwrap_or(Value::Null);
            Ok(v)
        }
        // A club in another league (later extras X7).
        "other" => {
            let fr = num.ok_or("Missing id.")? as usize;
            let t = crate::lane_money::league_team(w, fr)?;
            let mut v = t["team"].clone();
            v["kind"] = json!("team");
            v["line"] =
                json!(format!("{} · {}", t["league"]["name"].as_str().unwrap_or(""), t["city"].as_str().unwrap_or("")));
            v["record"] =
                t["record"]["text"].as_str().map_or(Value::Null, |r| json!(format!("{r} in {}", t["record"]["year"])));
            v["coach"] = t["staff"].get(0).map_or(Value::Null, |c| c["name"].clone());
            v["next"] = t["upcoming"].get(0).cloned().unwrap_or(Value::Null);
            Ok(v)
        }
        "game" => {
            let k = num.ok_or("Missing index.")? as usize;
            let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
            Ok(json!({ "kind": "game", "index": k, "date": d_text(g.date), "names": g.names, "score": g.stats.score }))
        }
        _ => Err("Nothing to show.".into()),
    }
}

// ---- Search everything (S6) ----

/// The screens search reaches (S6): name and where it goes.
const SCREENS: &[(&str, &str)] = &[
    ("Home", "home"),
    ("Inbox", "inbox"),
    ("Calendar", "calendar"),
    ("My players", "myplayers"),
    ("Roster", "roster"),
    ("Depth chart", "depth"),
    ("Training", "training"),
    ("Recruiting", "recruiting"),
    ("Contracts", "contracts"),
    ("Schedule", "schedule"),
    ("Games", "games"),
    ("Staff", "staff"),
    ("News", "news"),
    ("Standings", "standings"),
    ("Leaders", "leaders"),
    ("Player finder", "finder"),
    ("History and records", "history"),
    ("Hall of Fame", "history"),
    ("Your team", "team"),
    ("You", "you"),
    ("Settings", "settings"),
];

pub fn find(w: &World, q: &str) -> Value {
    let q = q.trim();
    if q.chars().count() < 2 {
        return json!({ "groups": [] });
    }
    let ql = q.to_lowercase();
    let hit = |s: &str| s.to_lowercase().contains(&ql);
    let starts = |s: &str| {
        s.to_lowercase().split_whitespace().any(|wd| wd.starts_with(&ql)) || s.to_lowercase().starts_with(&ql)
    };
    let mut groups = Vec::new();
    // Players: active first, then by stars and age.
    let mut ps: Vec<(i32, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| hit(&p.name))
        .map(|(k, p)| {
            let mut s = if p.stage == Stage::Done { 0 } else { 100 };
            if starts(&p.name) {
                s += 50;
            }
            if views::team_of(w, p).is_some_and(|t| views::is_mine(w, t)) {
                s += 200;
            }
            if desk::is_listed(w, p.player.id) {
                s += 150;
            }
            s += p.stars as i32 * 5;
            (s, k)
        })
        .collect();
    ps.sort_by_key(|x| std::cmp::Reverse(x.0));
    let players: Vec<Value> = ps
        .iter()
        .take(12)
        .map(|&(_, k)| {
            let p = &w.people[k];
            json!({ "kind": "player", "id": p.player.id, "name": p.name, "sub": format!("{} · {} · {}", p.player.position.name(), views::where_now(w, p), views::class_text(w, p)) })
        })
        .collect();
    if !players.is_empty() {
        groups.push(json!({ "name": "Players", "items": players, "more": ps.len().saturating_sub(12) }));
    }
    let coaches: Vec<Value> = w
        .coaches
        .iter()
        .enumerate()
        .filter(|(_, c)| hit(&c.name))
        .take(8)
        .map(|(i, c)| {
            let job = views::coach(w, i).ok().and_then(|v| v["job"].as_object().map(|j| format!("{} {}", j.get("name").and_then(Value::as_str).unwrap_or(""), j.get("role").and_then(Value::as_str).unwrap_or(""))));
            json!({ "kind": "coach", "id": i, "name": c.name, "sub": job.unwrap_or_else(|| if c.retired { "Retired".into() } else { "Out of coaching".into() }) })
        })
        .collect();
    if !coaches.is_empty() {
        groups.push(json!({ "name": "Coaches", "items": coaches }));
    }
    let mut teams: Vec<Value> = Vec::new();
    for p in &w.programs {
        if hit(w.program_name(p.id)) {
            teams.push(json!({ "kind": "team", "ref": TeamRef::College(p.id).json(), "name": w.program_name(p.id), "sub": format!("{} · {}", p.division.name(), p.conference), "colors": views::team_colors(w, TeamRef::College(p.id)) }));
        }
    }
    for i in 0..w.nfl.len() {
        let n = w.nfl_team_name(i);
        if hit(&n) {
            teams.push(json!({ "kind": "team", "ref": TeamRef::Nfl(i).json(), "name": n, "sub": "NFL", "colors": views::team_colors(w, TeamRef::Nfl(i)) }));
        }
    }
    let mut schools = 0;
    for h in &w.high_schools {
        if schools < 8 && hit(w.school_name(h.id)) {
            schools += 1;
            teams.push(json!({ "kind": "team", "ref": TeamRef::HighSchool(h.id).json(), "name": w.school_name(h.id), "sub": format!("High school · {}, {}", h.town, w.states[h.state].code) }));
        }
    }
    teams.truncate(12);
    if !teams.is_empty() {
        groups.push(json!({ "name": "Teams and schools", "items": teams }));
    }
    // Towns: high schools and colleges in a town.
    let mut towns: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for h in &w.high_schools {
        if towns.len() >= 6 {
            break;
        }
        if hit(&h.town) && seen.insert((h.town.clone(), h.state)) {
            let code = &w.states[h.state].code;
            towns.push(json!({ "kind": "finder", "name": format!("{}, {code}", h.town), "sub": "Players from this town", "args": { "filters": [{ "field": "town", "op": "=", "value": format!("{}, {code}", h.town) }] } }));
        }
    }
    if !towns.is_empty() {
        groups.push(json!({ "name": "Towns", "items": towns }));
    }
    let games: Vec<Value> = w
        .seat_games()
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, g)| g.names.iter().any(|n| hit(n)))
        .take(5)
        .map(|(k, g)| json!({ "kind": "game", "index": k, "name": format!("{} {} at {} {}", g.names[1], g.stats.score[1], g.names[0], g.stats.score[0]), "sub": d_text(g.date) }))
        .collect();
    if !games.is_empty() {
        groups.push(json!({ "name": "Games", "items": games }));
    }
    let mut records: Vec<Value> = Vec::new();
    for ((scope, book), list) in &w.stats.books {
        if records.len() >= 6 {
            break;
        }
        let Scope::Division(d) = scope else { continue };
        for e in list.iter().take(3) {
            if hit(&e.name) || book_name(*book).to_lowercase().contains(&ql) {
                records.push(json!({ "kind": "history", "name": format!("{} · {}", e.name, book_name(*book)), "sub": format!("{} record · {} in {}", d.name(), fmt_value(e.value), e.year), "args": { "tab": "records", "division": d.name() } }));
                break;
            }
        }
    }
    if !records.is_empty() {
        groups.push(json!({ "name": "Records", "items": records }));
    }
    let screens: Vec<Value> = SCREENS
        .iter()
        .filter(|(n, _)| hit(n))
        .map(|(n, go)| json!({ "kind": "screen", "name": n, "go": go, "sub": "Screen" }))
        .collect();
    if !screens.is_empty() {
        groups.push(json!({ "name": "Screens", "items": screens }));
    }
    let notes: Vec<Value> = desk::notes(w)["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| n["text"].as_str().is_some_and(hit))
        .take(5)
        .map(|n| json!({ "kind": "note", "name": n["text"], "sub": format!("{} · {}", n["date"].as_str().unwrap_or(""), n["page"].as_str().unwrap_or("")), "about": n["about"] }))
        .collect();
    if !notes.is_empty() {
        groups.push(json!({ "name": "Your notes", "items": notes }));
    }
    let mails: Vec<Value> = desk::list(w, &json!({ "q": q }))["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .take(5)
        .map(|m| json!({ "kind": "mail", "id": m["id"], "name": m["subject"], "sub": format!("{} · {}", m["from"].as_str().unwrap_or(""), m["day"].as_str().unwrap_or("")) }))
        .collect();
    if !mails.is_empty() {
        groups.push(json!({ "name": "Mail", "items": mails }));
    }
    json!({ "groups": groups })
}

fn book_name(b: Book) -> String {
    match b {
        Book::Season(i) => format!("Season {}", RECORD_STATS.get(i).map_or("record", |s| s.name())),
        Book::Career(i) => format!("Career {}", RECORD_STATS.get(i).map_or("record", |s| s.name())),
        Book::Streak(i) => STREAKS.get(i).map_or("Streak".to_string(), |s| s.to_string()),
        Book::WinStreak => "Consecutive wins".to_string(),
    }
}

fn fmt_value(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 { format!("{:.0}", v) } else { format!("{v:.1}") }
}

// ---- Compare (S8) ----

pub fn compare(w: &World, args: &Value) -> Value {
    let ids: Vec<u32> = args
        .get("ids")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_u64().map(|x| x as u32)).take(5).collect())
        .unwrap_or_default();
    let players: Vec<Value> = ids
        .iter()
        .filter_map(|&id| {
            let v = views::player(w, id).ok()?;
            let k = views::person_index(w, id)?;
            let last = w.stats.career(id).last().map(|r| format!("{}: {}", r.year, mail::stat_line(r.pos, |s| r.get(s))));
            Some(json!({
                "id": id, "name": v["name"], "pos": v["pos"], "age": v["age"], "team": v["team"], "class": v["class"],
                "ht": v["ht"], "wt": v["wt"], "stars": v["stars"], "ovr": v["ovr"], "groups": v["groups"], "mine": v["mine"], "yours": v["yours"],
                "level_rating": level_rating(w, k), "level_marks": level_marks(w, k), "last": last,
            }))
        })
        .collect();
    json!({ "players": players })
}

// ---- History (records, Hall of Fame, awards, the coaching carousel, drafts) ----

pub fn history(w: &World, args: &Value) -> Value {
    let tab = args.get("tab").and_then(Value::as_str).unwrap_or("records");
    let mine_div = match views::mine(w) {
        Some(TeamRef::College(p)) => w.programs[p].division,
        _ => afm_engine::world::data::Division::FBS,
    };
    let div = args
        .get("division")
        .and_then(Value::as_str)
        .and_then(afm_engine::world::data::Division::parse)
        .unwrap_or(mine_div);
    let years: Vec<i32> = (w.options.start_year - w.options.history_years as i32..=w.year).rev().collect();
    let body = match tab {
        "records" => {
            let mut books: Vec<(Book, &Vec<afm_engine::world::records::RecordEntry>)> = w
                .stats
                .books
                .iter()
                .filter(|((s, _), _)| *s == Scope::Division(div))
                .map(|((_, b), l)| (*b, l))
                .collect();
            books.sort_by_key(|x| x.0);
            json!(books
                .iter()
                .map(|(b, list)| json!({
                    "name": book_name(*b),
                    "rows": list.iter().take(10).enumerate().map(|(i, e)| json!({
                        "rank": i + 1, "value": fmt_value(e.value), "id": e.id, "name": e.name,
                        "team": (e.id == 0 || e.program < w.programs.len()).then(|| views::team_brief(w, TeamRef::College(e.program))),
                        "year": if e.from != e.year && e.from != 0 { format!("{}-{}", e.from, e.year) } else { e.year.to_string() },
                        "live": e.live,
                    })).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>())
        }
        "nfl_records" => json!(w
            .nfl_book
            .records
            .iter()
            .map(|r| json!({ "stat": r.stat.to_string(), "career": r.career, "value": fmt_value(r.value), "holder": r.holder, "year": r.year, "world": r.world }))
            .collect::<Vec<_>>()),
        "hall" => {
            let mut v: Vec<Value> = w
                .stats
                .hall
                .iter()
                .rev()
                .map(|h| json!({ "year": h.year, "name": h.name, "coach": h.coach, "level": format!("College Football Hall of Fame · {}", h.division.name()), "teams": h.programs.iter().map(|&p| views::team_brief(w, TeamRef::College(p))).collect::<Vec<_>>(), "why": h.why }))
                .collect();
            v.extend(w.nfl_book.hall.iter().rev().map(|h| json!({ "year": h.0, "id": h.1, "name": h.2, "pos": h.3.name(), "level": "Pro Football Hall of Fame", "why": h.4 })));
            v.sort_by_key(|x| std::cmp::Reverse(x["year"].as_i64().unwrap_or(0)));
            json!(v)
        }
        "awards" => {
            let year = args.get("year").and_then(Value::as_i64).map_or(w.year - 1, |y| y as i32);
            let mut v: Vec<Value> = Vec::new();
            for (id, h) in &w.stats.honors {
                for (y, a) in &h.awards {
                    if *y == year {
                        v.push(json!({ "award": a, "id": id, "name": h.name, "pos": h.pos.map(|p| p.name()), "level": "College" }));
                    }
                }
            }
            for a in w.nfl_book.awards.iter().filter(|a| a.0 == year) {
                v.push(json!({ "award": a.1, "id": a.2, "name": a.3, "team": views::team_brief(w, TeamRef::Nfl(a.4)), "level": "NFL" }));
            }
            v.sort_by(|a, b| a["level"].as_str().cmp(&b["level"].as_str()).then(a["award"].as_str().cmp(&b["award"].as_str())));
            json!({ "year": year, "rows": v })
        }
        "carousel" => {
            let year = args.get("year").and_then(Value::as_i64).map_or(w.year, |y| y as i32);
            let names: Vec<String> = w
                .news
                .iter()
                .rev()
                .filter(|n| n.year == year && matches!(n.kind, afm_engine::world::news::NewsKind::CoachHired | afm_engine::world::news::NewsKind::CoachFired | afm_engine::world::news::NewsKind::CoachLeft | afm_engine::world::news::NewsKind::CoachExtended))
                .map(|n| n.text.clone())
                .collect();
            let lk = Linker::new(w, &names.iter().map(String::as_str).collect::<Vec<_>>(), false);
            json!({ "year": year, "rows": w.news.iter().rev().filter(|n| n.year == year && matches!(n.kind, afm_engine::world::news::NewsKind::CoachHired | afm_engine::world::news::NewsKind::CoachFired | afm_engine::world::news::NewsKind::CoachLeft | afm_engine::world::news::NewsKind::CoachExtended)).take(300).map(|n| json!({
                "date": n.date.map(d_text), "kind": format!("{:?}", n.kind).replace("Coach", ""),
                "text": lk.link_refs(&n.text, &n.refs, &n.program.map(TeamRef::College).into_iter().collect::<Vec<_>>()),
            })).collect::<Vec<_>>() })
        }
        "drafts" => {
            let year = args.get("year").and_then(Value::as_i64).map_or_else(
                || if w.today.month() >= 5 { w.today.year() } else { w.today.year() - 1 },
                |y| y as i32,
            );
            let mut picks: Vec<(u32, u32, &afm_engine::world::Person, usize)> = w
                .people
                .iter()
                .filter_map(|p| p.pro.as_ref().and_then(|c| c.draft).filter(|d| d.0 == year).map(|d| (d.1, d.2, p, d.3)))
                .collect();
            picks.sort_by_key(|x| (x.0, x.1));
            json!({ "year": year, "rows": picks.iter().map(|(r, pk, p, t)| json!({
                "round": r, "pick": pk, "id": p.player.id, "name": p.name, "pos": p.player.position.name(),
                "team": views::team_brief(w, TeamRef::Nfl(*t)),
                "college": p.path.last().map(|&c| views::team_brief(w, TeamRef::College(c))),
            })).collect::<Vec<_>>() })
        }
        _ => {
            // Champions by year.
            let mut rows: Vec<Value> = Vec::new();
            for p in &w.programs {
                for s in p.seasons.iter().filter(|s| s.national_title) {
                    rows.push(json!({ "year": s.year, "level": p.division.name(), "team": views::team_brief(w, TeamRef::College(p.id)), "record": format!("{}-{}", s.wins, s.losses), "coach": s.coach.map(|c| json!({ "id": c, "name": w.coaches[c].name })) }));
                }
            }
            for (y, t) in &w.nfl_champions {
                rows.push(json!({ "year": y, "level": "NFL", "team": views::team_brief(w, TeamRef::Nfl(*t)), "record": "Super Bowl" }));
            }
            rows.sort_by_key(|x| std::cmp::Reverse(x["year"].as_i64().unwrap_or(0)));
            json!(rows)
        }
    };
    json!({ "tab": tab, "division": div.name(), "years": years, "body": body })
}

// ---- Playoff brackets ----

pub fn brackets(w: &World, args: &Value) -> Value {
    let level = args.get("level").and_then(Value::as_str).unwrap_or(match views::mine(w) {
        Some(TeamRef::Nfl(_)) => "nfl",
        Some(TeamRef::HighSchool(_)) => "hs",
        _ => "college",
    });
    match level {
        "nfl" => {
            let names = ["Wild Card", "Divisional round", "Conference championships", "Super Bowl"];
            let games = w.nfl_fixtures(None);
            let rounds: Vec<Value> = (19..=22)
                .map(|wk| {
                    let gs: Vec<Value> = games
                        .iter()
                        .filter(|g| g.week == wk)
                        .map(|g| json!({ "home": views::team_brief(w, TeamRef::Nfl(g.home)), "away": views::team_brief(w, TeamRef::Nfl(g.away)), "score": g.score, "date": g.when.map(d_text) }))
                        .collect();
                    json!({ "name": names[wk - 19], "games": gs })
                })
                .collect();
            json!({ "level": "nfl", "title": format!("{} NFL playoffs", w.nfl_season_year().unwrap_or(w.year)), "rounds": rounds })
        }
        "hs" => {
            // The state's season keeps only who is still alive, not each
            // round's pairings, so the page shows your own road through the
            // playoffs: your games after the last regular-season week.
            let Some(TeamRef::HighSchool(h)) = views::mine(w) else { return json!({ "rounds": [] }) };
            let s = &w.high_schools[h];
            let last_regular = w.hs_fixtures(h).iter().map(|f| f.date).max();
            let rounds: Vec<Value> = last_regular
                .map(|last| {
                    w.seat_games()
                        .iter()
                        .enumerate()
                        .filter(|(_, g)| g.date > last && g.date.days_to(last) > -60 && g.teams.contains(&h))
                        .enumerate()
                        .map(|(i, (k, g))| {
                            let score = [g.stats.score[0].max(0) as u32, g.stats.score[1].max(0) as u32];
                            json!({ "name": format!("Round {}", i + 1), "games": [json!({
                                "home": views::team_brief(w, TeamRef::HighSchool(g.teams[0])), "away": views::team_brief(w, TeamRef::HighSchool(g.teams[1])),
                                "score": score, "date": d_text(g.date), "page": k,
                            })] })
                        })
                        .collect()
                })
                .unwrap_or_default();
            json!({ "level": "hs", "title": format!("{} {} playoffs · your road", w.states[s.state].code, views::hs_class(w, h)), "rounds": rounds, "pending": rounds.is_empty() })
        }
        _ => {
            use afm_engine::world::data::Division;
            let div = args.get("division").and_then(Value::as_str).and_then(Division::parse).unwrap_or(
                match views::mine(w) {
                    Some(TeamRef::College(p)) => w.programs[p].division,
                    _ => Division::FBS,
                },
            );
            let start = if div == Division::FBS { (12, 10) } else { (11, 20) };
            let all: Vec<_> = w
                .college_fixtures(None)
                .into_iter()
                .filter(|g| !g.conference && w.programs[g.home].division == div && w.programs[g.away].division == div)
                .filter(|g| (g.date.month(), g.date.day()) >= start || g.date.month() <= 2)
                .collect();
            // The bracket, built back from the final: each team's previous
            // postseason game feeds the round before.
            let mut rounds: Vec<Vec<usize>> = Vec::new();
            if let Some(fin) = (0..all.len()).max_by_key(|&k| all[k].date) {
                rounds.push(vec![fin]);
                loop {
                    let mut prev = Vec::new();
                    for &g in rounds.last().unwrap() {
                        for team in [all[g].home, all[g].away] {
                            if let Some(p) = (0..all.len())
                                .filter(|&k| all[k].date < all[g].date && (all[k].home == team || all[k].away == team))
                                .max_by_key(|&k| all[k].date)
                                && all[g].date.days_to(all[p].date).abs() <= 21
                                && !prev.contains(&p)
                            {
                                prev.push(p);
                            }
                        }
                    }
                    if prev.is_empty() || rounds.len() > 6 {
                        break;
                    }
                    rounds.push(prev);
                }
            }
            rounds.reverse();
            let n = rounds.len();
            let finished = all.iter().any(|g| g.score.is_some()) && rounds.len() > 1;
            let names = |i: usize| match n - i {
                1 => "National championship",
                2 => "Semifinals",
                3 => "Quarterfinals",
                4 => "Second round",
                _ => "First round",
            };
            let out: Vec<Value> = rounds
                .iter()
                .enumerate()
                .map(|(i, gs)| {
                    json!({ "name": names(i), "games": gs.iter().map(|&k| {
                        let g = &all[k];
                        json!({ "home": views::team_brief(w, TeamRef::College(g.home)), "away": views::team_brief(w, TeamRef::College(g.away)), "score": g.score, "date": d_text(g.date), "name": g.name })
                    }).collect::<Vec<_>>() })
                })
                .collect();
            json!({ "level": "college", "division": div.name(), "title": format!("{} {} playoff", w.college_year().unwrap_or(w.year), div.name()), "rounds": if finished { out } else { Vec::new() }, "pending": !finished })
        }
    }
}

// ---- News feeds: the beat, the national desk, recruiting, the fans ----

fn certainty(e: &Event) -> &'static str {
    match e.kind {
        Kind::Whisper => "Rumour",
        Kind::Breakout | Kind::Visit => "Reported",
        _ => "Confirmed",
    }
}

pub fn feed(w: &World, args: &Value) -> Value {
    let kind = args.get("kind").and_then(Value::as_str).unwrap_or("beat");
    let chain = args.get("chain").and_then(Value::as_str).unwrap_or("");
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(80) as usize;
    let me = views::mine(w);
    let yours = views::yours_test(w);
    let seat = w.seat;
    let from = w.today.add(-120);
    let keep = |e: &Event| -> bool {
        if e.kind == Kind::Slot || e.kind == Kind::Rhythm {
            return false;
        }
        match kind {
            "beat" => {
                yours(e)
                    || me.is_some_and(|t| {
                        mail::rivals(w, t).iter().any(|&r| e.text.contains(views::team_name(w, r).as_str()))
                    })
            }
            "national" => e.loud >= Loudness::Big,
            "recruits" => e.level == CalLevel::Recruiting,
            "level" => seat.is_none_or(|s| s.cares(e)) && e.loud >= Loudness::Medium,
            _ => true,
        }
    };
    let evs = events_in(w, from, w.today, keep);
    let texts: Vec<&str> = evs.iter().take(400).map(|(_, e)| e.text.as_str()).collect();
    let lk = Linker::new(w, &texts, matches!(me, Some(TeamRef::HighSchool(_))));
    let (paper, _) = me.map(|t| people::paper(w, t)).unwrap_or((people::NATIONAL_PAPER.to_string(), String::new()));
    // Story chains: each item's subjects (the people and teams it links);
    // an earlier item sharing one is the story so far.
    let mut items: Vec<(Value, Vec<String>)> = Vec::new();
    let mut seen_subject: HashMap<String, usize> = HashMap::new();
    for (d, e) in evs.iter().take(400) {
        let text = tidy(&lk.event(e));
        let subjects: Vec<String> = subjects_in(&text);
        if !chain.is_empty() && !subjects.iter().any(|s| s == chain) {
            continue;
        }
        let source = match kind {
            "beat" => paper.clone(),
            "recruits" => "The recruiting wire".to_string(),
            _ => people::NATIONAL_PAPER.to_string(),
        };
        // H6: a story about one of your people says why, as your staff sees it.
        let why = me.filter(|_| yours(e)).and_then(|t| {
            // One person's story only: a roundup of five commitments has no single why.
            let mut people = e.refs.iter().filter_map(|r| match r.to {
                afm_engine::calendar::Link::Person(id) => Some(id),
                _ => None,
            });
            match (people.next(), people.next()) {
                (Some(id), None) => crate::lane_help::news_why(w, t, *d, id),
                _ => None,
            }
        });
        items.push((
            json!({ "date": d_text(*d), "text": text, "kind": kicker(e), "certainty": certainty(e), "source": source, "loud": e.loud.name(), "mine": yours(e), "scene": scene(e), "why": why }),
            subjects,
        ));
        if items.len() >= limit {
            break;
        }
    }
    // Count, for each item, the older items on the same story.
    for k in (0..items.len()).rev() {
        let mut best: Option<(String, usize)> = None;
        for s in &items[k].1 {
            let n = *seen_subject.get(s).unwrap_or(&0);
            if n > 0 && best.as_ref().is_none_or(|b| n > b.1) {
                best = Some((s.clone(), n));
            }
        }
        for s in items[k].1.clone() {
            *seen_subject.entry(s).or_default() += 1;
        }
        if let Some((s, n)) = best {
            items[k].0["chain"] = json!({ "key": s, "count": n });
        }
    }
    // The fans' voice on your own games.
    let mut fans = Vec::new();
    if kind == "fans"
        && let Some(t) = me
    {
        for (k, g) in w.seat_games().iter().enumerate().rev().take(12) {
            let side = usize::from(match t {
                TeamRef::College(i) | TeamRef::Nfl(i) | TeamRef::HighSchool(i) | TeamRef::Other(i) => g.teams[0] != i,
            });
            let (us, them) = (g.stats.score[side], g.stats.score[1 - side]);
            let margin = us - them;
            let mood = match margin {
                m if m >= 21 => {
                    ["That was a statement.", "Book the trip to the title game.", "Best we've looked in years."]
                }
                m if m > 0 => ["A win is a win.", "Ugly but we'll take it.", "Fourth quarter defense won that one."],
                0 => ["A tie feels like a loss.", "Nobody goes home happy.", "Strange night."],
                m if m > -14 => {
                    ["We'll get them next week.", "Clock management cost us that one.", "Too many penalties."]
                }
                _ => ["Embarrassing.", "Somebody has to answer for that.", "I want my ticket money back."],
            };
            let handle =
                ["@SectionTwelve", "@TailgateTom", "@OldGoldFan", "@BlitzBoys", "@ThirdAndLong", "@HomecomingQueen"];
            for (j, line) in mood.iter().enumerate() {
                fans.push(json!({ "date": d_text(g.date), "text": format!("{} {}", line, links::game(k, &format!("{us}-{them}"))), "source": handle[(k + j) % handle.len()], "certainty": "Opinion", "kind": "Fans", "scene": "fans" }));
            }
        }
    }
    let rows: Vec<Value> = if kind == "fans" { fans } else { items.into_iter().map(|x| x.0).collect() };
    json!({ "kind": kind, "rows": rows, "chain": if chain.is_empty() { Value::Null } else { json!(chain) } })
}

/// The link targets a marked-up line names ("p:12", "t:college:4").
fn subjects_in(text: &str) -> Vec<String> {
    let mut v = Vec::new();
    for part in text.split("[[").skip(1) {
        if let Some(target) = part.split('|').next()
            && (target.starts_with("p:") || target.starts_with("t:") || target.starts_with("c:"))
        {
            v.push(target.to_string());
        }
    }
    v
}

// ---- The coach page ----

pub fn coach_page(w: &World, c: usize) -> Result<Value, String> {
    let mut v = views::coach(w, c)?;
    let co = &w.coaches[c];
    let tree: Vec<Value> = w
        .coaches
        .iter()
        .enumerate()
        .filter(|(_, x)| x.mentor == Some(c))
        .map(|(i, x)| {
            json!({ "id": i, "name": x.name, "job": views::coach(w, i).ok().map(|v| v["job"].clone()), "reputation": views::reputation_words(x.reputation) })
        })
        .collect();
    let mut developed: Vec<Value> = w
        .people
        .iter()
        .filter(|p| p.college_staff.iter().any(|s| s.2 == Some(c) || s.3 == Some(c)))
        .filter_map(|p| p.pro.as_ref().and_then(|x| x.draft).map(|d| (p, d)))
        .map(|(p, d)| json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "year": d.0, "round": d.1, "pick": d.2, "team": views::team_brief(w, TeamRef::Nfl(d.3)) }))
        .collect();
    developed.sort_by_key(|x| std::cmp::Reverse(x["year"].as_i64().unwrap_or(0)));
    developed.truncate(60);
    v["tree"] = json!(tree);
    v["developed"] = json!(developed);
    // Your own assistant's deal is the staff office's (update 2, Q5); the
    // world's contract book covers everyone else.
    let deal = w.folk.staff.deal(c).map(|d| {
        let owed = d.salary * (d.through - w.year + 1).max(0) as f64 * d.buyout as f64;
        json!({ "through": d.through, "salary": d.salary.round(), "buyout": owed.round() })
    });
    v["contract"] = json!(deal.or_else(|| {
        co.contract
            .as_ref()
            .map(|k| json!({ "through": k.through, "salary": k.salary.round(), "buyout": k.buyout(w.year).round() }))
    }));
    // Your read of him (update 2, Q2) comes with having him on your staff.
    v["your_staff"] = json!(afm_engine::world::staff_read::of_coach(w, c).is_some());
    v["mentor_line"] = json!(co.mentor.and_then(|m| w.coaches.get(m)).map(|m| links::coach(m.id, &m.name)));
    v["record"] = rec_record(w, c);
    v["notes"] = json!(w.career.as_ref().map(|k| {
        k.desk
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.about == Some((mail::ABOUT_COACH, c as u32)))
            .map(|(i, n)| json!({ "k": i, "date": d_text(n.date), "text": n.text, "page": n.page }))
            .collect::<Vec<_>>()
    }));
    Ok(v)
}

/// His recommendation record (SK30): who he wanted, who he passed on, how
/// they turned out, and how often you overruled him. Null without calls.
fn rec_record(w: &World, c: usize) -> Value {
    use afm_engine::world::front_office::chain;
    let calls = chain::record(w, c);
    if calls.is_empty() {
        return Value::Null;
    }
    let settled: Vec<_> = calls.iter().filter(|x| x.settled.is_some()).collect();
    let right = settled.iter().filter(|x| x.settled == Some(true)).count();
    let rows: Vec<Value> = calls
        .iter()
        .take(40)
        .map(|x| {
            let (name, pos) = views::person_index(w, x.id)
                .map(|k| (w.people[k].name.clone(), w.people[k].player.position.name()))
                .unwrap_or_default();
            json!({
                "id": x.id, "name": name, "pos": pos, "date": d_text(x.date),
                "call": if x.wanted { "Wanted him" } else { "Passed" },
                "you": if x.overruled { "Overruled" } else { "Agreed" },
                "turned": x.turned, "right": x.settled,
            })
        })
        .collect();
    json!({
        "wanted": calls.iter().filter(|x| x.wanted).count(),
        "passed": calls.iter().filter(|x| !x.wanted).count(),
        "overruled": calls.iter().filter(|x| x.overruled).count(),
        "settled": settled.len(), "right": right,
        "heart": chain::heart(w, c) < 1.0,
        "rows": rows,
    })
}

// ---- Your career page: the tree, your contract, the job market ----

pub fn career_page(w: &World) -> Value {
    let Some(c) = &w.career else { return Value::Null };
    let mut v = views::you(w);
    let co = &w.coaches[c.coach];
    v["contract"] = json!(co.contract.as_ref().map(|k| json!({ "through": k.through, "salary": k.salary.round(), "buyout": k.buyout(w.year).round(), "buyout_share": k.buyout_share })));
    v["mentor"] =
        json!(co.mentor.and_then(|m| w.coaches.get(m)).map(
            |m| json!({ "id": m.id, "name": m.name, "job": views::coach(w, m.id).ok().map(|x| x["job"].clone()) })
        ));
    v["tree"] =
        json!(w
        .coaches
        .iter()
        .enumerate()
        .filter(|(_, x)| x.mentor == Some(c.coach))
        .map(|(i, x)| json!({ "id": i, "name": x.name, "job": views::coach(w, i).ok().map(|v| v["job"].clone()) }))
        .collect::<Vec<_>>());
    // The job market: head jobs that opened this year at your level and the
    // one above, from the news desk's firings and departures.
    let open: Vec<Value> = w
        .news
        .iter()
        .rev()
        .filter(|n| n.year >= w.year - 1 && matches!(n.kind, afm_engine::world::news::NewsKind::CoachFired | afm_engine::world::news::NewsKind::CoachLeft))
        .filter_map(|n| n.program.map(|p| (n, p)))
        .take(30)
        .map(|(n, p)| {
            let prog = &w.programs[p];
            json!({
                "team": views::team_brief(w, TeamRef::College(p)),
                "line": format!("{} · {}", prog.division.name(), prog.conference),
                "date": n.date.map(d_text),
                "filled": prog.head_coach.and_then(|h| w.coaches.get(h)).filter(|h| h.job.is_some_and(|j| j.since >= n.year)).map(|h| json!({ "id": h.id, "name": h.name })),
                "prestige": views::prestige_words(prog.prestige_rel),
            })
        })
        .collect();
    v["market"] = json!(open);
    v["stock"] = json!(views::reputation_words(co.reputation));
    v
}

// ---- The team page ----

pub fn team_page(w: &World, t: TeamRef) -> Value {
    let mut v = views::team(w, t);
    let rivals = mail::rivals(w, t);
    v["rivals"] = json!(rivals.iter().take(8).map(|&r| views::team_brief(w, r)).collect::<Vec<_>>());
    v["ref"] = t.json();
    let name = views::team_name(w, t);
    let lines: Vec<(Date, &Event)> = events_in(w, w.today.add(-60), w.today, |e| match t {
        TeamRef::College(p) => e.program == Some(p) || e.text.contains(name.as_str()),
        TeamRef::Nfl(i) => e.nfl_team == Some(i) || e.text.contains(name.as_str()),
        TeamRef::HighSchool(_) | TeamRef::Other(_) => e.text.contains(name.as_str()),
    });
    let texts: Vec<&str> = lines.iter().take(20).map(|(_, e)| e.text.as_str()).collect();
    let lk = Linker::new(w, &texts, matches!(t, TeamRef::HighSchool(_)));
    v["news"] = json!(
        lines
            .iter()
            .take(20)
            .map(|(d, e)| json!({ "date": d_text(*d), "text": tidy(&lk.event(e)), "kind": kicker(e) }))
            .collect::<Vec<_>>()
    );
    v["coach"] = json!(
        match t {
            TeamRef::College(p) => w.programs[p].head_coach,
            TeamRef::HighSchool(h) => w.high_schools[h].coach,
            TeamRef::Nfl(i) => w
                .coaches
                .iter()
                .position(|c| c.job.is_some_and(|j| j.employer == Employer::Nfl(i) && j.role == Role::HeadCoach)),
            TeamRef::Other(f) => afm_engine::world::pro::other::head_coach(w, f),
        }
        .map(|c| json!({ "id": c, "name": w.coaches[c].name }))
    );
    if let TeamRef::College(p) = t {
        // The team's statistical leaders last season.
        let year = w.stats.year_index.last().map_or(w.year - 1, |x| x.0);
        let mut leaders = Vec::new();
        for (label, key) in [
            ("Passing", afm_engine::world::records::Stat::PassYds),
            ("Rushing", afm_engine::world::records::Stat::RushYds),
            ("Receiving", afm_engine::world::records::Stat::RecYds),
            ("Tackles", afm_engine::world::records::Stat::Tackles),
            ("Sacks", afm_engine::world::records::Stat::Sacks),
        ] {
            if let Some(r) = w
                .stats
                .year(year)
                .iter()
                .filter(|r| r.program as usize == p)
                .max_by(|a, b| a.get(key).total_cmp(&b.get(key)))
                && r.get(key) > 0.0
            {
                let name = views::person_index(w, r.id).map(|k| w.people[k].name.clone()).unwrap_or_default();
                leaders.push(json!({ "label": label, "id": r.id, "name": name, "pos": r.pos.name(), "line": mail::stat_line(r.pos, |s| r.get(s)), "year": year }));
            }
        }
        v["leaders"] = json!(leaders);
    }
    let _ = tables::COLS.len();
    v
}

/// Lines from another screen linked (L1 helper for other lanes' pages).
pub fn linked(w: &World, args: &Value) -> Value {
    let texts: Vec<String> = args
        .get("texts")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let schools = args.get("schools").and_then(Value::as_bool).unwrap_or(false);
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let lk = Linker::new(w, &refs, schools);
    json!({ "texts": texts.iter().map(|t| tidy(&lk.link(t, &[]))).collect::<Vec<_>>() })
}

// ---- The calendar month ----

fn parse_day(s: &str) -> Option<Date> {
    let mut it = s.split('-').map(|x| x.parse::<i32>().ok());
    let (y, m, d) = (it.next()??, it.next()??, it.next().flatten().unwrap_or(1));
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then(|| Date::ymd(y, m as u32, d as u32))
}

fn iso(d: Date) -> String {
    let (y, m, dd) = d.parts();
    format!("{y:04}-{m:02}-{dd:02}")
}

/// One month of the calendar (topic 4 on screen): every day with your games
/// (and their scores once played), the moments that concern you, and the
/// loud ones for everyone. A picked day lists everything posted on it.
pub fn calendar_month(w: &World, args: &Value) -> Value {
    let picked = args.get("day").and_then(Value::as_str).and_then(parse_day);
    let focus = picked.or_else(|| args.get("month").and_then(Value::as_str).and_then(parse_day)).unwrap_or(w.today);
    let first = Date::ymd(focus.year(), focus.month(), 1);
    let next = if focus.month() == 12 {
        Date::ymd(focus.year() + 1, 1, 1)
    } else {
        Date::ymd(focus.year(), focus.month() + 1, 1)
    };
    let prev = if focus.month() == 1 {
        Date::ymd(focus.year() - 1, 12, 1)
    } else {
        Date::ymd(focus.year(), focus.month() - 1, 1)
    };
    let start = first.add(-(first.weekday() as i32));
    let yours = views::yours_test(w);
    let my_slot = views::my_slot_test(w);
    // Your games this month, with the kept game's page when there is one.
    let me = views::mine(w);
    let mut games: HashMap<i32, Value> = HashMap::new();
    let kept = w.seat_games();
    let page_of = |d: Date| kept.iter().position(|g| g.date == d);
    match me {
        Some(TeamRef::College(p)) => {
            for f in w
                .college_fixtures(Some(p))
                .into_iter()
                .filter(|f| f.date >= start && f.date < next.add(7) && !f.cancelled)
            {
                let home = f.home == p;
                let opp = if home { f.away } else { f.home };
                let at = if home || f.neutral { "vs" } else { "at" };
                let score = f.score.map(|s| if home { s } else { [s[1], s[0]] });
                games.insert(f.date.0, json!({ "opp": views::team_brief(w, TeamRef::College(opp)), "at": at, "name": f.name, "score": score, "page": page_of(f.date) }));
            }
        }
        Some(TeamRef::Nfl(t)) => {
            for f in w.nfl_fixtures(Some(t)) {
                let Some(d) = f.when.filter(|d| *d >= start && *d < next.add(7)) else { continue };
                let home = f.home == t;
                let opp = if home { f.away } else { f.home };
                let score = f.score.map(|s| if home { s } else { [s[1], s[0]] });
                games.insert(d.0, json!({ "opp": views::team_brief(w, TeamRef::Nfl(opp)), "at": if home { "vs" } else { "at" }, "name": Value::Null, "score": score, "page": page_of(d) }));
            }
        }
        Some(TeamRef::HighSchool(h)) => {
            for f in w.hs_fixtures(h).into_iter().filter(|f| f.date >= start && f.date < next.add(7)) {
                let home = f.home == h;
                let opp = if home { f.away } else { f.home };
                let score = kept.iter().find(|g| g.date == f.date).map(|g| {
                    let s = usize::from(g.teams[0] != h);
                    [g.stats.score[s].max(0) as u32, g.stats.score[1 - s].max(0) as u32]
                });
                games.insert(f.date.0, json!({ "opp": views::team_brief(w, TeamRef::HighSchool(opp)), "at": if home { "vs" } else { "at" }, "name": if f.playoff { Some("Playoffs") } else { None }, "score": score, "page": page_of(f.date) }));
            }
        }
        Some(TeamRef::Other(fr)) => {
            for (d, opp, home, score, round) in afm_engine::world::pro::other::season_games(w, fr) {
                if d < start || d >= next.add(7) {
                    continue;
                }
                let score = score.map(|s| [s[0] as u32, s[1] as u32]);
                games.insert(d.0, json!({ "opp": views::team_brief(w, TeamRef::Other(opp)), "at": if home { "vs" } else { "at" }, "name": (round > 0).then_some("Playoffs"), "score": score, "page": page_of(d) }));
            }
        }
        None => {}
    }
    let dates: Vec<(Date, String)> = views::fixed_dates(w, &[focus.year() - 1, focus.year(), focus.year() + 1]);
    let mut weeks = Vec::new();
    let mut d = start;
    while d < next || weeks.len() < 5 {
        let mut days = Vec::new();
        for _ in 0..7 {
            let evs: Vec<&Event> = w
                .calendar
                .on(d)
                .iter()
                .filter(|e| my_slot(e) || !matches!(e.kind, Kind::Slot | Kind::Rhythm | Kind::Game))
                .filter(|e| {
                    yours(e)
                        || my_slot(e)
                        || e.loud >= Loudness::Big
                        || matches!(e.kind, Kind::Date | Kind::Deadline | Kind::Window)
                            && e.loud >= Loudness::Medium
                            && w.seat.is_some_and(|s| s.cares(e))
                })
                .collect();
            let texts: Vec<&str> = evs.iter().map(|e| e.text.as_str()).take(4).collect();
            let lk = Linker::new(w, &texts, matches!(me, Some(TeamRef::HighSchool(_))));
            let fixed: Vec<Value> = dates
                .iter()
                .filter(|x| x.0 == d)
                .map(|x| json!({ "text": x.1, "kind": "Date", "mine": false, "loud": "Medium" }))
                .collect();
            days.push(json!({
                "iso": iso(d), "day": d.day(), "in_month": d >= first && d < next, "today": d == w.today, "past": d < w.today,
                "game": games.get(&d.0),
                "items": fixed.iter().cloned().chain(evs.iter().take(4).map(|e| json!({ "text": tidy(&lk.event(e)), "kind": kicker(e), "mine": yours(e) || my_slot(e), "loud": e.loud.name() }))).collect::<Vec<_>>(),
                "more": evs.len().saturating_sub(4),
            }));
            d = d.add(1);
        }
        weeks.push(days);
        if d >= next {
            break;
        }
    }
    // The picked day in full.
    let day = picked.map(|p| {
        let evs: Vec<&Event> = w.calendar.on(p).iter().filter(|e| my_slot(e) || !matches!(e.kind, Kind::Slot | Kind::Rhythm)).collect();
        let texts: Vec<&str> = evs.iter().map(|e| e.text.as_str()).take(60).collect();
        let lk = Linker::new(w, &texts, matches!(me, Some(TeamRef::HighSchool(_))));
        json!({
            "iso": iso(p), "pretty": p.pretty(), "in": w.today.days_to(p), "game": games.get(&p.0),
            "items": dates.iter().filter(|x| x.0 == p).map(|x| json!({ "text": x.1, "kind": "Date", "mine": false, "loud": "Medium" }))
                .chain(evs.iter().take(60).map(|e| json!({ "text": tidy(&lk.event(e)), "kind": kicker(e), "mine": yours(e) || my_slot(e), "loud": e.loud.name() }))).collect::<Vec<_>>(),
            "kept": p >= w.today.add(-400),
        })
    });
    let ahead = views::calendar(w);
    json!({
        "title": format!("{} {}", ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"][focus.month() as usize - 1], focus.year()),
        "month": format!("{:04}-{:02}", focus.year(), focus.month()),
        "prev": format!("{:04}-{:02}", prev.year(), prev.month()),
        "next": format!("{:04}-{:02}", next.year(), next.month()),
        "today": iso(w.today),
        "weeks": weeks,
        "day": day,
        "ahead": ahead["items"].as_array().map(|a| a.iter().take(24).cloned().collect::<Vec<_>>()),
    })
}
