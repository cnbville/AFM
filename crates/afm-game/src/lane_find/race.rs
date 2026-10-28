//! A recruit's race (update 2, lane 7, board 8): each school's standing
//! with him week by week as your office followed it, your school against
//! the leader (a high school coach sees the leader's margin), the moments
//! that moved him, and the tug of war of his reasons against any school
//! after him.
//!
//! The standings come from the race log your office keeps
//! (`front_office::race`), which holds your staff's read of each week, never
//! his true interest; today's point is read the same way. His reasons are
//! the parts of what each school means to him
//! (`front_office::college::reasons`); how much each counts with whoever
//! decides for him is shown only as your staff's read, a range on 1 to 5
//! that narrows as your staff gets to know him. His ties are only the ones
//! anyone can see, and when he decides is a window your staff has heard,
//! never his own plan.

use afm_engine::calendar::Date;
use afm_engine::world::background::Decider;
use afm_engine::world::front_office::college::{self as desk, VisitKind, VisitState};
use afm_engine::world::front_office::{dollars, race};
use afm_engine::world::program::GradeCat;
use afm_engine::world::scouting::{dept, services};
use afm_engine::world::{Stage, Visit, World, geo, key_normal, ncaa};
use serde_json::{Value, json};

use crate::Reply;
use crate::views::{self, TeamRef};

/// The reasons, in `front_office::college::reasons`' order.
const REASONS: [&str; 7] = [
    "Close to home",
    "Playing time",
    "NIL money",
    "Coaching and development",
    "Winning and the name",
    "Academics",
    "His ties and feel",
];

/// A school's name short enough for a button: its initials ("SU" for a
/// State, the way the schools write it: Boise State is BSU), or the first
/// four letters of a one-word name.
fn short_name(name: &str) -> String {
    let words: Vec<&str> =
        name.split_whitespace().filter(|w| w.chars().next().is_some_and(char::is_uppercase) && *w != "of").collect();
    if words.len() >= 2 && words.last() == Some(&"State") {
        let head: String = words[..words.len() - 1].iter().filter_map(|w| w.chars().next()).collect();
        format!("{}SU", head.to_uppercase())
    } else if words.len() >= 2 {
        words.iter().filter_map(|w| w.chars().next()).collect::<String>().to_uppercase()
    } else {
        name.chars().filter(|c| c.is_alphanumeric()).take(4).collect::<String>().to_uppercase()
    }
}

/// Short names for the schools on one page, none shared: the usual
/// initials ("ASU"), then the first word's first four letters where two
/// schools would share them (Arizona State "ARIZ", Arkansas State "ARKA"),
/// then the full name.
fn unique_shorts(w: &World, ids: &[usize]) -> Vec<(usize, String)> {
    let mut left = ids.to_vec();
    left.sort_unstable();
    left.dedup();
    let four = |n: &str| {
        n.split_whitespace()
            .next()
            .unwrap_or(n)
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(4)
            .collect::<String>()
            .to_uppercase()
    };
    let mut done: Vec<(usize, String)> = Vec::new();
    for tier in 0..3 {
        let cand: Vec<String> = left
            .iter()
            .map(|&j| {
                let n = w.program_name(j);
                match tier {
                    0 => short_name(n),
                    1 => four(n),
                    _ => n.to_string(),
                }
            })
            .collect();
        let mut next = Vec::new();
        for (x, &j) in left.iter().enumerate() {
            let clash =
                cand.iter().enumerate().any(|(y, c)| y != x && *c == cand[x]) || done.iter().any(|d| d.1 == cand[x]);
            if clash && tier < 2 {
                next.push(j);
            } else {
                done.push((j, cand[x].clone()));
            }
        }
        left = next;
    }
    done
}

fn day(d: Date) -> String {
    super::college::short(d)
}

/// Your staff's read of how much a reason counts with him: a range on 1
/// to 5 around the truth, narrower with each call, visit and camp your
/// staff has had with him (a high school coach knows his own player).
fn weight_read(w: &World, i: usize, own: bool, contacts: usize) -> Vec<[u8; 2]> {
    let p = &w.people[i];
    // The six wants, and his ties and feel at his average want (the model
    // gives them no want of their own).
    let six = desk::reason_weights(w, i);
    let wts: Vec<f64> = six.iter().copied().chain([six.iter().sum::<f64>() / six.len() as f64]).collect();
    let top = wts.iter().copied().fold(0.01, f64::max);
    let half = if own { 0.5 } else { (2.0 - 0.35 * contacts as f64).max(0.5) };
    wts.iter()
        .enumerate()
        .map(|(k, &x)| {
            let truth = 1.0 + 4.0 * (x / top).clamp(0.0, 1.0);
            let centre = truth + 0.6 * half * key_normal(0x7A6E ^ k as u64, p.player.id as u64).clamp(-1.5, 1.5);
            let lo = (centre - half).round().clamp(1.0, 5.0) as u8;
            let hi = (centre + half).round().clamp(1.0, 5.0) as u8;
            [lo, hi.max(lo)]
        })
        .collect()
}

/// Players at his position on a program now with a season left after this
/// one: who would be ahead of him.
fn ahead_at(w: &World, progs: &[usize], i: usize) -> Vec<usize> {
    let pos = w.people[i].player.position;
    let mut n = vec![0usize; progs.len()];
    for p in &w.people {
        if p.stage != Stage::College || p.player.position != pos || p.college_seasons >= 4 {
            continue;
        }
        if let Some(k) = p.college.and_then(|c| progs.iter().position(|&x| x == c)) {
            n[k] += 1;
        }
    }
    n
}

/// His race: the page.
pub fn page(w: &World, args: &Value) -> Reply {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Which recruit?")? as u32;
    let i = views::person_index(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let me = match views::mine(w) {
        Some(TeamRef::College(j)) => Some(j),
        _ => None,
    };
    // A high school coach's page; he reads closely only his own player.
    let seat_hs = matches!(views::mine(w), Some(TeamRef::HighSchool(_)));
    let rd = race::reader(w, i);
    let own = rd.own_player;
    // Who decides for him and how he feels about a school's ties: known to
    // his own coach, or once your staff has been in touch (as on Find
    // recruits' "Who decides").
    let known = own || super::staff::contacted(w, id);
    let log = race::of(w, id);
    let today = w.today;
    // Every school in his weeks, with its line (None where it wasn't in his top).
    let weeks: Vec<&race::RaceWeek> = log.map(|l| l.weeks.iter().collect()).unwrap_or_default();
    let mut progs: Vec<usize> = Vec::new();
    for wk in &weeks {
        for &(j, _) in &wk.schools {
            if !progs.contains(&(j as usize)) {
                progs.push(j as usize);
            }
        }
    }
    // Today's race too, so a school just in it shows.
    let now_race = desk::race_at(w, i);
    for &(j, _) in now_race.iter().take(race::SCHOOLS) {
        if !progs.contains(&j) {
            progs.push(j);
        }
    }
    if let Some(j) = me
        && now_race.iter().any(|r| r.0 == j)
        && !progs.contains(&j)
    {
        progs.push(j);
    }
    let base = log.map(|l| l.base).unwrap_or_else(|| now_race.first().map_or(0.0, |r| (r.1 - 0.10) as f32));
    // The log holds your staff's reads; today's is read the same way.
    let now_of = |j: usize| {
        now_race.iter().find(|r| r.0 == j).map(|r| race::read(rd, id, j, today, race::interest(base, r.1)) as i64)
    };
    let first = weeks.first().map_or(today, |x| x.date);
    let span = first.days_to(today).max(1);
    let mut schools: Vec<Value> = progs
        .iter()
        .map(|&j| {
            let mut pts: Vec<[i64; 2]> = weeks
                .iter()
                .filter_map(|wk| {
                    wk.schools.iter().find(|s| s.0 as usize == j).map(|s| [first.days_to(wk.date) as i64, s.1 as i64])
                })
                .collect();
            if let Some(v) = now_of(j)
                && weeks.last().is_none_or(|x| x.date < today)
            {
                pts.push([span as i64, v]);
            }
            let name = w.program_name(j).to_string();
            json!({
                "id": j, "team": views::team_brief(w, TeamRef::College(j)), "name": name, "short": short_name(&name),
                "me": Some(j) == me, "pts": pts, "now": now_of(j),
                "committed": p.cal.commit.is_some_and(|c| c.0 == j), "signed": p.cal.signed.is_some_and(|s| s.program == j),
            })
        })
        .collect();
    // Month marks from the first week to today.
    let mut months: Vec<Value> = Vec::new();
    let (y0, m0, _) = first.parts();
    let (mut y, mut m) = (y0, m0);
    loop {
        let d = Date::ymd(y, m, 1);
        if d > today {
            break;
        }
        if d >= first {
            months.push(json!({ "t": first.days_to(d), "label": day(d) }));
        }
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    // The moments: dated steps in his recruitment since your office began
    // following him, each with what it did to that school's line.
    // A school's line just before a day, and the first week from it on.
    let before = |j: usize, d: Date| -> Option<i64> {
        weeks
            .iter()
            .rev()
            .filter(|wk| wk.date < d)
            .find_map(|wk| wk.schools.iter().find(|s| s.0 as usize == j))
            .map(|s| s.1 as i64)
    };
    let after = |j: usize, d: Date| -> Option<i64> {
        weeks
            .iter()
            .filter(|wk| wk.date >= d)
            .find_map(|wk| wk.schools.iter().find(|s| s.0 as usize == j))
            .map(|s| s.1 as i64)
            .or_else(|| if d <= today { now_of(j) } else { None })
    };
    let mut moments: Vec<(Date, String, Option<usize>, bool)> = Vec::new();
    let name = |j: usize| w.program_name(j).to_string();
    for v in &p.cal.visits {
        let what = match v.2 {
            Visit::Official => "Official visit to",
            Visit::Unofficial => "Unofficial visit to",
            Visit::Home => "Home visit from",
            Visit::DropIn => "A coach from",
        };
        let tail = if v.2 == Visit::DropIn { " watched him at school" } else { "" };
        moments.push((v.1, format!("{what} {}{tail}", name(v.0)), Some(v.0), v.1 > today));
    }
    for &(j, d) in &p.cal.early_offers {
        moments.push((d, format!("{} offers", name(j)), Some(j), false));
    }
    if let Some(c) = p.cal.commit {
        moments.push((c.1, format!("Commits to {}", name(c.0)), Some(c.0), false));
    }
    if let Some(s) = p.cal.signed {
        moments.push((s.date, format!("Signs with {}", name(s.program)), Some(s.program), false));
    }
    if let Some(j) = me {
        for t in w.front_office.college.talks.iter().filter(|t| t.id == id) {
            moments.push((t.date, format!("Your call: {}", t.answer), Some(j), false));
        }
        for v in
            w.front_office.college.visits.iter().filter(|v| v.id == id && v.state == VisitState::Set && v.date > today)
        {
            let what = match v.kind {
                VisitKind::Official => "His official visit to you",
                VisitKind::Unofficial => "His unofficial visit to you",
                VisitKind::Home => "Your home visit",
                VisitKind::DropIn => "Your drop-in at his school",
            };
            moments.push((v.date, what.to_string(), Some(j), true));
        }
    }
    moments.retain(|m| m.0 >= first.add(-7) || m.3);
    moments.sort_by_key(|m| m.0);
    moments.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    let mut moments: Vec<Value> = moments
        .iter()
        .rev()
        .take(8)
        .rev()
        .map(|(d, t, j, planned)| {
            let delta = match (j, planned) {
                (Some(j), false) => match (before(*j, *d), after(*j, d.add(1))) {
                    (Some(a), Some(b)) => Some(b - a),
                    _ => None,
                },
                _ => None,
            };
            json!({
                "t": first.days_to(*d), "date": views::date_text(*d), "text": t, "planned": planned,
                "school": j.map(|j| json!({ "id": j, "short": short_name(&name(j)), "me": Some(j) == me })), "delta": delta,
            })
        })
        .collect();
    // His reasons against each school in his race now (yours too, when
    // you're in it).
    let reads = weight_read(w, i, own, rd.contacts);
    let mut cmp: Vec<usize> = now_race.iter().take(race::SCHOOLS).map(|r| r.0).collect();
    if let Some(j) = me
        && !cmp.contains(&j)
        && (progs.contains(&j) || now_race.iter().any(|r| r.0 == j))
    {
        cmp.push(j);
    }
    let parts: Vec<[(f64, f64); 7]> = cmp.iter().map(|&j| desk::reasons(w, i, j)).collect();
    let ahead = ahead_at(w, &cmp, i);
    let you_mi = |j: usize| {
        let pr = &w.programs[j];
        let home = match w.high_schools.get(p.hs) {
            Some(h) => geo::place(h.state, &h.town),
            None => geo::place(p.home_state, ""),
        };
        geo::miles_between(home, geo::campus(pr.state, &pr.city)).round()
    };
    let nil_of = |j: usize| -> f64 {
        if Some(j) == me {
            w.front_office
                .college
                .board
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.nil)
                .unwrap_or_else(|| desk::usual_offer(w, j, id))
        } else {
            desk::usual_offer(w, j, id)
        }
    };
    let grade = |j: usize, g: GradeCat| w.programs[j].grades[g as usize];
    let reasons: Vec<Value> = REASONS
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let by: Vec<Value> = cmp
                .iter()
                .enumerate()
                .map(|(s, &j)| {
                    let fact = match k {
                        0 => format!("{:.0} {} from home", you_mi(j), if you_mi(j).round() == 1.0 { "mile" } else { "miles" }),
                        1 => format!("{} {}{} with a year left", ahead[s], p.player.position.name(), if ahead[s] == 1 { "" } else { "s" }),
                        2 if ncaa::nil_era(w.year) => format!("{} a year", dollars(nil_of(j))),
                        2 => "no NIL before 2021".to_string(),
                        3 => format!(
                            "pro pipeline {:.0}, coaches {:.0}",
                            grade(j, GradeCat::ProPipeline),
                            grade(j, GradeCat::CoachPrestige)
                        ),
                        4 => format!("{} name, title chances {:.0}", w.programs[j].division.name(), grade(j, GradeCat::TitleChances)),
                        5 => format!("academics {:.0}", grade(j, GradeCat::Academics)),
                        _ => ties_words(w, i, j, known),
                    };
                    // His ties score only the ones anyone can see (never his
                    // own feel for a school).
                    let score = if k == 6 { ties_score(w, i, j, known) } else { parts[s][k].1 };
                    json!({ "id": j, "score": (score * 100.0).round(), "fact": fact })
                })
                .collect();
            json!({ "k": k, "name": n, "weight": reads.get(k), "by": by, "move": move_words(k, seat_hs), "source": source_words(w, i, k, me, known) })
        })
        .collect();
    // Where it stands.
    // His decision: done (signed or committed), planned, or overdue.
    let decides = if let Some(s) = p.cal.signed {
        // A walk-on signs no letter: he takes a roster place.
        let when = match (s.walk_on, s.date.month()) {
            (true, _) => "a preferred walk-on place",
            (_, 12) => "in the early period",
            (_, 1..=4) => "in the regular period",
            _ => "after the signing periods",
        };
        let label = if s.walk_on { "He chose" } else { "He signed" };
        Some(
            json!({ "label": label, "date": views::date_text(s.date), "what": when, "t": first.days_to(s.date), "past": true }),
        )
    } else if let Some((_, d, _)) = p.cal.commit {
        Some(
            json!({ "label": "He committed", "date": views::date_text(d), "what": "not signed yet", "t": first.days_to(d), "past": true }),
        )
    } else {
        decides_read(w, i, own, known).map(|r| {
            if r.overdue {
                let late = format!("was expected by {}", r.when);
                return json!({ "label": "He decides", "date": "Still open", "what": late, "t": first.days_to(today), "past": true });
            }
            json!({ "label": "He decides", "date": r.when, "what": r.what, "t": first.days_to(r.until), "past": false, "window": !r.exact })
        })
    };
    let next_visit = me
        .and_then(|j| {
            w.front_office
                .college
                .visits
                .iter()
                .filter(|v| v.id == id && v.state == VisitState::Set && v.date > today)
                .min_by_key(|v| v.date)
                .map(|v| (v.date, w.program_name(j).to_string()))
        })
        .or_else(|| {
            p.cal
                .visits
                .iter()
                .filter(|v| v.1 > today)
                .min_by_key(|v| v.1)
                .map(|v| (v.1, w.program_name(v.0).to_string()))
        });
    let decider = if !known {
        "Not known yet"
    } else {
        match desk::family_read(w, i).decider {
            Decider::Himself => "Himself",
            Decider::Parent => "His parents",
            Decider::Grandparent => "His grandparents",
            Decider::Handler => "A trainer close to the family",
        }
    };
    let reader = me.and_then(|j| dept::area_coach(w, j, p.home_state.min(50) as u8)).map(
        |c| json!({ "id": c, "name": w.coaches[c].name, "state": w.states.get(p.home_state).map(|s| s.code.clone()) }),
    );
    let stars = super::fields::rstars(p);
    // No two schools on the page under one short name.
    let ids: Vec<usize> = schools
        .iter()
        .chain(moments.iter().map(|m| &m["school"]))
        .filter_map(|v| v.get("id").and_then(Value::as_u64))
        .map(|j| j as usize)
        .collect();
    let shorts = unique_shorts(w, &ids);
    for v in schools.iter_mut().chain(moments.iter_mut().map(|m| &mut m["school"])) {
        if let Some(j) = v.get("id").and_then(Value::as_u64)
            && let Some(s) = shorts.iter().find(|s| s.0 == j as usize)
        {
            v["short"] = json!(s.1);
        }
    }
    Ok(json!({
        "id": id, "name": p.name, "pos": p.player.position.name(), "stars": stars, "class_of": p.class_of,
        "town": views::hometown(w, p), "seat": if seat_hs { "hs" } else { "college" },
        "me": me, "team": views::mine(w).map(|t| views::team_brief(w, t)),
        "followed": !weeks.is_empty(), "since": day(first), "span": span,
        "months": months, "schools": schools, "moments": moments, "reasons": reasons, "compare": cmp,
        "decides": decides, "listens": decider,
        "next_visit": next_visit.map(|(d, n)| json!({ "date": views::date_text(d), "where": n })),
        "reader": reader,
        "committed": p.cal.commit.map(|c| w.program_name(c.0).to_string()),
        "signed": p.cal.signed.map(|s| w.program_name(s.program).to_string()),
        "walk_on": p.cal.signed.map_or(p.cal.commit.is_some_and(|c| c.2), |s| s.walk_on),
        "on_board": me.is_some() && w.front_office.college.board.iter().any(|t| t.id == id),
    }))
}

/// His ties to a school, in words: the ones anyone can see, and the team
/// he grew up a fan of once your staff has talked with him.
fn ties_words(w: &World, i: usize, j: usize, known: bool) -> String {
    let p = &w.people[i];
    let mut v: Vec<String> = Vec::new();
    if p.cal.signed.is_some_and(|s| s.program == j) {
        v.push("signed".into());
    } else if p.cal.commit.is_some_and(|c| c.0 == j) {
        v.push("committed".into());
    }
    if p.cal.visits.iter().any(|x| x.0 == j && x.2 == Visit::Official && x.1 <= w.today) {
        v.push("official visit".into());
    }
    if known && p.childhood_team == Some(j) {
        v.push("grew up a fan".into());
    }
    if p.family().older_brother_at == Some(j) {
        v.push("his brother played there".into());
    }
    if w.programs[j].state == p.home_state {
        v.push("in-state".into());
    }
    if w.programs[j].hs_pipeline.contains_key(&p.hs) {
        v.push("his school's pipeline".into());
    }
    if v.is_empty() { "nothing beyond the offer".into() } else { v.join(", ") }
}

/// His ties to a school as a score from 0 to 1, built only from what
/// `ties_words` names (proposed, not confirmed: 0.1 to start; signed is
/// 1; committed 0.35, an official visit taken 0.15, each unofficial 0.05 up
/// to two, a fan 0.15, a brother 0.15, in-state 0.1, his school's pipeline
/// 0.1).
fn ties_score(w: &World, i: usize, j: usize, known: bool) -> f64 {
    let p = &w.people[i];
    if p.cal.signed.is_some_and(|s| s.program == j) {
        return 1.0;
    }
    let past = |k: Visit| p.cal.visits.iter().filter(|x| x.0 == j && x.2 == k && x.1 <= w.today).count();
    let mut x = 0.1;
    if p.cal.commit.is_some_and(|c| c.0 == j) {
        x += 0.35;
    }
    if past(Visit::Official) > 0 {
        x += 0.15;
    }
    x += 0.05 * past(Visit::Unofficial).min(2) as f64;
    if known && p.childhood_team == Some(j) {
        x += 0.15;
    }
    if p.family().older_brother_at == Some(j) {
        x += 0.15;
    }
    if w.programs[j].state == p.home_state {
        x += 0.1;
    }
    if w.programs[j].hs_pipeline.contains_key(&p.hs) {
        x += 0.1;
    }
    x.clamp(0.0, 1.0)
}

/// When he decides, as your staff has heard it.
struct DecidesRead {
    /// "Dec 2026", or the day itself once it's close and out.
    when: String,
    what: &'static str,
    /// The window's last day (the day itself when exact).
    until: Date,
    exact: bool,
    overdue: bool,
}

/// When he decides, as a read (R38): nothing until there's a signal (his
/// own coach, an announced top list, or your staff in touch with him);
/// then the period or the month his plan falls in; the day itself only
/// once his list is out (or he's your own player) and it's within 30 days.
/// Proposed, not confirmed.
fn decides_read(w: &World, i: usize, own: bool, known: bool) -> Option<DecidesRead> {
    let p = &w.people[i];
    let listed = w.scouting.rankings.tops.iter().any(|t| t.id == p.player.id);
    if !(own || listed || known) {
        return None;
    }
    let d = services::decision_day(w, i)?;
    let today = w.today;
    let (y, m, _) = d.parts();
    let month_end = if m == 12 { Date::ymd(y + 1, 1, 1).add(-1) } else { Date::ymd(y, m + 1, 1).add(-1) };
    let early = afm_engine::calendar::anchors::early_signing(p.class_of - 1);
    let (what, until) = if early == Some(d) {
        // The early period runs three days from its Wednesday.
        ("the early signing period", d.add(2))
    } else if m == 2 {
        ("signing day", month_end)
    } else {
        ("a commitment", month_end)
    };
    let exact = (own || listed) && (0..=30).contains(&today.days_to(d));
    let when = if exact { views::date_text(d) } else { month_year(d) };
    Some(DecidesRead { when, what, until: if exact { d } else { until }, exact, overdue: until < today })
}

/// "Dec 2026".
fn month_year(d: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, _) = d.parts();
    format!("{} {y}", M[(m as usize).clamp(1, 12) - 1])
}

/// What could move a reason, in your staff's words.
fn move_words(k: usize, seat_hs: bool) -> &'static str {
    if seat_hs {
        return match k {
            0 => "Nothing: the miles stay",
            1 => "Each staff's depth chart at his position",
            2 => "The collectives' numbers",
            3 => "Who coaches his position there",
            4 => "Their seasons",
            5 => "His grades and their programs",
            _ => "Visits, and who he trusts",
        };
    }
    match k {
        0 => "A home visit; the miles stay",
        1 => "Show him your depth chart on a visit",
        2 => "Your NIL promise on his file",
        3 => "Your position coach's calls",
        4 => "Win your next big games",
        5 => "Your academic staff on an official visit",
        _ => "An official visit, and your calls",
    }
}

/// Where your staff learned it.
fn source_words(w: &World, i: usize, k: usize, me: Option<usize>, known: bool) -> Vec<String> {
    let p = &w.people[i];
    let mut v: Vec<String> = Vec::new();
    if let Some(t) = w.front_office.college.talks.iter().filter(|t| t.id == p.player.id).max_by_key(|t| t.date)
        && me.is_some()
    {
        v.push(format!("Your call, {}", views::date_text(t.date)));
    }
    match k {
        0 => v.push("His hometown".into()),
        2 if ncaa::nil_era(w.year) => v.push("The deal trackers".into()),
        5 => v.push("His transcript".into()),
        _ => {}
    }
    // Who he listens to is a source only once your staff knows it.
    if known {
        match desk::family_read(w, i).decider {
            Decider::Parent => v.push("His parents".into()),
            Decider::Grandparent => v.push("His grandparents".into()),
            Decider::Handler => v.push("His trainer".into()),
            Decider::Himself => v.push("His high school coach".into()),
        }
    } else {
        v.push("His high school coach".into());
    }
    v.truncate(3);
    v
}
