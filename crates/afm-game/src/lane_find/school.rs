//! Colleges calling: the high school coach's side (FR15).
//!
//! A high school coach doesn't search for recruits: the colleges come to
//! him. This page shows his players being recruited, with every service's
//! stars and the composite (the same public word a college sees), and
//! every college staff in touch about them: who calls, about whom, how far
//! it has gone, the last contact, what the staff wants and how much it
//! takes his word.
//!
//! Offers, visits, commitments and signings are the world's own. The
//! staffs that show interest without an offer yet are read from what the
//! colleges see: a staff recruits where its footprint reaches, at the
//! level the services put the kid, and invites the younger ones to its
//! June camp. The last contact keeps to the recruiting calendar: no visit
//! to school in a dead or quiet period.

use std::collections::HashMap;

use afm_engine::calendar::{self as cal, Date, Period, Weekday};
use afm_engine::world::coach::Group;
use afm_engine::world::data::Division;
use afm_engine::world::front_office::college as desk;
use afm_engine::world::scouting::readers::Staff;
use afm_engine::world::scouting::{dept, hs, services};
use afm_engine::world::{Stage, Visit, World, geo, key_uniform};
use serde_json::{Value, json};

use super::fields::Ranks;
use super::query::thousands;
use super::short;
use crate::Reply;
use crate::views::{self, TeamRef};

/// How far a staff has gone with a kid, least to most.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Stage2 {
    Interest,
    Camp,
    Visit,
    Offer,
    Committed,
    Signed,
}

impl Stage2 {
    fn word(self) -> &'static str {
        match self {
            Stage2::Interest => "Interest",
            Stage2::Camp => "Camp invite",
            Stage2::Visit => "Visit",
            Stage2::Offer => "Offer",
            Stage2::Committed => "Committed",
            Stage2::Signed => "Signed",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Stage2::Interest => "interest",
            Stage2::Camp => "camp",
            Stage2::Visit => "visit",
            Stage2::Offer => "offer",
            Stage2::Committed | Stage2::Signed => "commit",
        }
    }
}

/// One staff's contact with one kid.
struct Touch {
    prog: usize,
    kid: usize,
    stage: Stage2,
    date: Date,
    how: &'static str,
    want: String,
}

/// How a staff's level sits against a kid's: the program's pull at its
/// own level.
fn pull(w: &World, prog: usize) -> f64 {
    let p = &w.programs[prog];
    match p.division {
        Division::FBS => 0.45 + 0.55 * p.prestige_rel,
        Division::FCS => 0.2 + 0.35 * p.prestige_rel,
        Division::JUCO => 0.15,
        _ => 0.25 * p.prestige_rel,
    }
}

/// The level the services' word puts a kid at.
fn want(p: &afm_engine::world::Person) -> f64 {
    match super::fields::rstars(p) {
        5 => 0.92,
        4 => 0.78,
        3 => 0.6,
        2 => 0.38,
        _ if p.cal.all_state > 0 => 0.25,
        _ => 0.12,
    }
}

/// A staff's June camp day (the first, second or third Saturday).
fn camp_day(prog: usize, year: i32) -> Date {
    let n = 1 + (key_uniform(0xCA4F ^ prog as u64, year as u64) * 3.0) as u32;
    Date::nth_weekday(year, 6, Weekday::Sat, n.min(3))
}

/// A recent day for a contact the world doesn't date, kept to the
/// calendar: a visit to school only when in-person contact is allowed.
fn recent(w: &World, prog: usize, id: u32, salt: u64) -> (Date, &'static str) {
    let back = (key_uniform(0x7E11 ^ salt ^ prog as u64, id as u64 ^ (w.today.0 / 14) as u64) * 20.0) as i32;
    let d = w.today.add(-back);
    let period = cal::recruiting_period(&w.dates, d);
    let u = key_uniform(0x7E12 ^ prog as u64, id as u64 ^ d.0 as u64);
    let how = match period {
        Period::Contact | Period::Evaluation if u < 0.3 => "visit to school",
        _ if u < 0.55 => "call",
        _ if u < 0.8 => "text",
        _ => "letter",
    };
    (d, how)
}

/// What a staff wants now, in the words its coach would use.
fn wants(w: &World, prog: usize, p: &afm_engine::world::Person, stage: Stage2, senior: bool) -> String {
    let m = w.today.month();
    let year = w.today.year();
    let pr = &w.programs[prog];
    let camp = camp_day(prog, year);
    match stage {
        Stage2::Signed => "Signed; sends his summer reporting date".into(),
        Stage2::Committed => "Keeping him close until he signs".into(),
        Stage2::Offer => {
            if let Some(v) = p.cal.visits.iter().find(|v| v.0 == prog && v.1 > w.today) {
                format!("{} visit set for {}", if v.2 == Visit::Official { "Official" } else { "A" }, short(v.1))
            } else if senior {
                "Wants his decision before signing day".into()
            } else if (1..=3).contains(&m) {
                "Wants him on campus for a junior day".into()
            } else if m <= 6 && w.today < camp {
                format!("Wants him at their {} camp", short(camp))
            } else {
                "Asked for his latest film".into()
            }
        }
        Stage2::Visit => match p.cal.visits.iter().filter(|v| v.0 == prog).max_by_key(|v| v.1) {
            Some(v) if v.1 > w.today => format!("Visit set for {}", short(v.1)),
            Some(v) => format!("Visited {}; still evaluating", short(v.1)),
            None => "Wants him on campus".into(),
        },
        Stage2::Camp => format!("Camp {}", short(camp)),
        Stage2::Interest => {
            if matches!(pr.academic_tier, afm_engine::world::data::AcademicTier::Aau)
                && key_uniform(0xAC4D ^ prog as u64, p.player.id as u64) < 0.6
            {
                "Academic fit: asked for his transcript".into()
            } else {
                match m {
                    1..=3 => "Asked for his junior film".into(),
                    4..=5 => "Watched him in spring practice".into(),
                    6..=8 => "Wants him at a summer camp".into(),
                    9..=11 => "Watched him play on a Friday".into(),
                    _ => "Sent a letter; asked about his grades".into(),
                }
            }
        }
    }
}

/// Every staff in touch about one kid.
fn touches(w: &World, i: usize, sign: i32) -> Vec<Touch> {
    let p = &w.people[i];
    let id = p.player.id;
    let senior = p.class_of <= sign;
    let mut out: Vec<Touch> = Vec::new();
    let mut put =
        |prog: usize, stage: Stage2, date: Date, how: &'static str| match out.iter_mut().find(|t| t.prog == prog) {
            Some(t) => {
                if stage > t.stage {
                    t.stage = stage;
                }
                if date > t.date {
                    t.date = date;
                    t.how = how;
                }
            }
            None => out.push(Touch { prog, kid: i, stage, date, how, want: String::new() }),
        };
    if let Some(s) = p.cal.signed {
        put(s.program, Stage2::Signed, s.date, "signing");
    }
    if let Some(c) = p.cal.commit {
        put(c.0, Stage2::Committed, c.1, "commitment");
    }
    for &(c, d) in &p.cal.early_offers {
        put(c, Stage2::Offer, d, "offer");
    }
    for &c in &p.cal.offers {
        let (d, how) = recent(w, c, id, 1);
        put(c, Stage2::Offer, d, how);
    }
    for v in &p.cal.visits {
        let d = v.1.min(w.today);
        put(v.0, Stage2::Visit, d, if v.1 > w.today { "visit set" } else { "visit" });
    }
    // The staffs with interest and no offer yet: where their footprint
    // reaches, at the level the services put him.
    let reach = match super::fields::rstars(p) {
        5 => 10,
        4 => 7,
        3 => 5,
        2 => 3,
        _ if p.cal.all_state > 0 => 2,
        _ => 1,
    };
    let reach = if p.class_of > sign + 1 { reach / 2 } else { reach };
    let late = senior && (w.today >= w.dates.signing_day || p.cal.signed.is_some() || p.cal.commit.is_some());
    if reach > 0 && !late && p.class_of <= sign + 2 {
        let lvl = want(p);
        let st = p.home_state.min(50);
        let mut cands: Vec<(f64, usize)> = w
            .programs
            .iter()
            .enumerate()
            .filter(|(k, pr)| {
                pr.head_coach.is_some()
                    && (pr.footprint[st] >= 0.25 || pr.state == p.home_state)
                    && (pull(w, *k) - lvl).abs() < 0.2
                    && !out.iter().any(|t| t.prog == *k)
            })
            .map(|(k, pr)| {
                let noise = key_uniform(0x1A7E ^ k as u64, id as u64);
                (
                    pr.footprint[st] + if pr.state == p.home_state { 0.3 } else { 0.0 } - (pull(w, k) - lvl).abs()
                        + 0.25 * noise,
                    k,
                )
            })
            .collect();
        cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        for &(_, k) in cands.iter().take(reach) {
            let camp = camp_day(k, w.today.year());
            let invited = !senior && (2..=6).contains(&w.today.month()) && w.today <= camp;
            let (d, how) = recent(w, k, id, 2);
            out.push(Touch {
                prog: k,
                kid: i,
                stage: if invited { Stage2::Camp } else { Stage2::Interest },
                date: d,
                how,
                want: String::new(),
            });
        }
    }
    for t in out.iter_mut() {
        t.want = wants(w, t.prog, p, t.stage, senior);
    }
    out
}

/// The coach who calls for a staff about a kid: its recruiter for his
/// state, with his job.
fn caller(w: &World, prog: usize, st: usize) -> Value {
    let Some(c) = dept::default_area_coach(w, prog, st as u8) else {
        return json!({ "name": "Their staff", "role": "" });
    };
    let pr = &w.programs[prog];
    let role = if pr.head_coach == Some(c) {
        "head coach".to_string()
    } else if pr.oc == Some(c) {
        "offensive coordinator".to_string()
    } else if pr.dc == Some(c) {
        "defensive coordinator".to_string()
    } else {
        pr.position_coaches
            .iter()
            .position(|x| *x == Some(c))
            .map_or("assistant".to_string(), |g| Group::ALL[g].coach_title().to_string())
    };
    json!({ "id": c, "name": w.coaches[c].name, "role": role })
}

/// His recruitment in a word, for the players table.
fn heat(p: &afm_engine::world::Person, offers: usize, calls: usize, sign: i32) -> &'static str {
    if p.cal.signed.is_some() {
        "Signed"
    } else if p.cal.commit.is_some() {
        "Committed"
    } else if offers >= 8 || (offers >= 3 && super::fields::rstars(p) >= 4) {
        "Hot"
    } else if offers >= 1 || calls >= 3 {
        "Building"
    } else if p.class_of > sign {
        "Early"
    } else {
        "Quiet"
    }
}

fn credit_word(c: f64) -> &'static str {
    if c >= 0.6 {
        "High"
    } else if c >= 0.35 {
        "Middle"
    } else {
        "Low"
    }
}

/// Colleges calling: the page.
pub fn calling(w: &World, _args: &Value) -> Reply {
    let h = hs::school(w).ok_or("Only a high school coach has this page.")?;
    let sign = desk::next_class(w);
    let staff = Staff::School(h);
    let kids: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.hs == h && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool))
        .map(|(i, _)| i)
        .collect();
    let mut ranks: HashMap<i32, Ranks> = HashMap::new();
    for &i in &kids {
        let c = w.people[i].class_of;
        ranks.entry(c).or_insert_with(|| Ranks::of_class(w, c));
    }
    let services: Vec<(usize, String)> = super::college::services_of(w)
        .iter()
        .filter_map(|v| Some((v["k"].as_u64()? as usize, v["name"].as_str()?.to_string())))
        .collect();
    let mut all: Vec<Touch> = Vec::new();
    let mut players: Vec<Value> = Vec::new();
    for &i in &kids {
        let p = &w.people[i];
        let id = p.player.id;
        let t = touches(w, i, sign);
        let offers = views::offers_of(p).len();
        let r = &ranks[&p.class_of];
        let by: Vec<Value> = services
            .iter()
            .map(|(k, name)| {
                let s = r.boards.iter().find(|b| b.0 == *k).and_then(|b| b.1.get(&id));
                json!({ "k": k, "name": name, "stars": s.map(|x| x.0), "rank": s.map(|x| x.1) })
            })
            .collect();
        let comp = r.comp.get(&id);
        let (ht, wt) = afm_engine::world::scouting::file::listed(&p.player);
        players.push(json!({
            "id": id,
            "name": p.name,
            "pos": p.player.position.name(),
            "class_of": p.class_of,
            "cls": views::class_text(w, p),
            "ht": afm_engine::body::height_text(ht),
            "wt": wt,
            "stars": p.stars,
            "camp_stars": if p.player.position.is_specialist() && p.camp_stars > 0 { Some(p.camp_stars) } else { None },
            "by": by,
            "comp": comp.map(|c| format!("{:.4}", c.1)),
            "nat": comp.map(|c| c.0),
            "offers": offers,
            "calls": t.len(),
            "heat": heat(p, offers, t.len(), sign),
            "committed": p.cal.commit.map(|c| views::team_brief(w, TeamRef::College(c.0))),
            "signed": p.cal.signed.map(|s| views::team_brief(w, TeamRef::College(s.program))),
            "walk_on": p.cal.signed.map_or(p.cal.commit.is_some_and(|c| c.2), |s| s.walk_on),
            "read": crate::lane_scouting::brief_read(w, staff, id),
            "promoted": w.scouting.hs.promoted.iter().filter(|x| x.id == id).count(),
        }));
        all.extend(t);
    }
    // One line per staff: the kids it calls about, its furthest stage and
    // its last contact.
    let mut by_prog: Vec<(usize, Vec<&Touch>)> = Vec::new();
    for t in &all {
        match by_prog.iter_mut().find(|x| x.0 == t.prog) {
            Some(x) => x.1.push(t),
            None => by_prog.push((t.prog, vec![t])),
        }
    }
    // Your school on the map, for the colleges' campuses around it.
    let home = w.high_schools.get(h).map_or((39.8, -98.6), |x| geo::place(x.state, &x.town));
    let mut lines: Vec<(Date, Value)> = by_prog
        .into_iter()
        .map(|(prog, ts)| {
            let top = ts.iter().max_by_key(|t| (t.stage, t.date)).copied().expect("a staff has a contact");
            let last = ts.iter().max_by_key(|t| t.date).copied().expect("a staff has a contact");
            let pr = &w.programs[prog];
            let kid = &w.people[top.kid];
            let stage = if ts.len() > 1 && top.stage >= Stage2::Offer {
                format!("{} ({})", top.stage.word(), afm_engine::world::names::surname_of(&kid.name))
            } else {
                top.stage.word().to_string()
            };
            let cred = hs::credibility(w, prog);
            let at = geo::campus(pr.state, &pr.city);
            let v = json!({
                "team": views::team_brief(w, TeamRef::College(prog)),
                "level": super::division_word(pr.division),
                "conf": w.conference_name(&pr.conference),
                "caller": caller(w, prog, kid.home_state),
                "about": ts.iter().map(|t| { let q = &w.people[t.kid]; json!({ "id": q.player.id, "name": q.name }) }).collect::<Vec<_>>(),
                "stage": stage,
                "stage_key": top.stage.key(),
                "last": format!("{} · {}", views::date_text(last.date), last.how),
                "last_date": short(last.date),
                "want": top.want,
                "credit": cred,
                "credit_word": credit_word(cred),
                "at": [round3(at.0), round3(at.1)],
                "mi": geo::miles_between(home, at).round(),
                "st": w.states.get(pr.state).map_or("", |x| x.code.as_str()),
            });
            (last.date, v)
        })
        .collect();
    lines.sort_by_key(|l| std::cmp::Reverse(l.0));
    let offers: usize = kids.iter().map(|&i| views::offers_of(&w.people[i]).len()).sum();
    let offered = kids.iter().filter(|&&i| !views::offers_of(&w.people[i]).is_empty()).count();
    let camps = all.iter().filter(|t| t.stage == Stage2::Camp).count();
    let called: Vec<&hs::Promo> = w.scouting.hs.promoted.iter().filter(|x| x.date.year() >= w.year - 5).collect();
    let settled = called.iter().filter(|x| x.settled.is_some()).count();
    let right = called.iter().filter(|x| x.settled == Some(true)).count();
    let mean_cred = if lines.is_empty() {
        0.5
    } else {
        lines.iter().map(|l| l.1["credit"].as_f64().unwrap_or(0.5)).sum::<f64>() / lines.len() as f64
    };
    let about = all.iter().map(|t| t.kid).collect::<std::collections::HashSet<_>>().len();
    let calls: Vec<Value> = w
        .scouting
        .hs
        .promoted
        .iter()
        .rev()
        .take(40)
        .filter_map(|x| {
            let i = afm_engine::world::scouting::place(w, x.id)?;
            let p = &w.people[i];
            Some(json!({
                "date": views::date_text(x.date),
                "player": { "id": p.player.id, "name": p.name },
                "to": x.program.map(|c| views::team_brief(w, TeamRef::College(c))),
                "said": hs::STRENGTHS[x.strength as usize],
                "settled": x.settled.map(|ok| if ok { "He proved you right" } else { "It cost you" }),
            }))
        })
        .collect();
    let period = cal::recruiting_period(&w.dates, w.today);
    let period_word = match period {
        Period::Contact => "a contact period: staffs may visit",
        Period::Evaluation => "an evaluation period: staffs may watch, not talk off campus",
        Period::Quiet => "a quiet period: contact on their campus only",
        Period::Dead => "a dead period: calls and letters only",
    };
    let next = services::next_update(w.today);
    Ok(json!({
        "seat": "hs",
        "home": {
            "at": [round3(home.0), round3(home.1)],
            "town": w.high_schools.get(h).map_or(String::new(), |x| x.town.clone()),
            "st": w.high_schools.get(h).and_then(|x| w.states.get(x.state)).map_or("", |x| x.code.as_str()),
        },
        "team": views::team_brief(w, TeamRef::HighSchool(h)),
        "players": players,
        "services": services.iter().map(|(k, n)| json!({ "k": k, "name": n })).collect::<Vec<_>>(),
        "lines": lines.into_iter().map(|l| l.1).collect::<Vec<_>>(),
        "count": thousands(about as i64),
        "sums": {
            "offers": offers, "offered": offered, "camps": camps,
            "credit_word": credit_word(mean_cred),
            "calls_right": right, "calls_settled": settled,
        },
        "calls": calls,
        "strengths": hs::STRENGTHS,
        "colleges": [],
        "period": period_word,
        "next_update": format!("{} ({})", short(next.0), next.1),
    }))
}

fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
