//! The boards' words for your level's five chases (boards 34 and 36): each
//! chase's word, its chart, what it pays and what it risks, read from the
//! systems' own records. The game lays them out; nothing here is hidden.

use super::Yr;
use crate::calendar::Level;
use crate::world::World;
use crate::world::coach::{Employer, Group};
use crate::world::news::count;
use crate::world::stories::TeamKey;

/// One chase as its panel shows it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Chase {
    pub code: &'static str,
    pub name: String,
    /// "Open", "Contested", "Strong".
    pub word: String,
    /// 0 good, 1 plain, 2 bad.
    pub tone: u8,
    /// The screen it lives on, in words ("Front office › Cap").
    pub home: String,
    /// One line for the Legacy screen's list of five.
    pub line: String,
    pub vis: Vis,
    /// (label, words, tone).
    pub kv: Vec<(String, String, u8)>,
}

/// A chase's chart.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Vis {
    /// Stacked bars by year: series names, then (x label, a value per series).
    Bars { names: Vec<String>, data: Vec<(String, Vec<f64>)>, tops: Option<Vec<String>> },
    /// Rows with a bar each: (label, value, words, tone), out of `max`.
    Rows { rows: Vec<(String, f64, String, u8)>, max: f64 },
    /// A line by point: `invert` puts the lowest value on top (a rank).
    Line { pts: Vec<(String, f64)>, lo: f64, hi: f64, invert: bool, marks: Vec<(usize, String)>, note: String },
    /// Nothing to draw yet.
    Empty { text: String },
}

/// Your level and its five chases, when you run a team.
pub fn five(w: &World) -> Option<(Level, Vec<Chase>)> {
    let (_, team) = super::you(w)?;
    Some(match team {
        TeamKey::Nfl(t) => (
            Level::Nfl,
            vec![l1(w, t as usize), l2(w, t as usize), l3(w, t as usize), l4(w, t as usize), l5(w, t as usize)],
        ),
        TeamKey::College(p) => {
            let p = p as usize;
            (Level::College, vec![l6(w, p), l7(w, p), l8(w, p), l9(w, p), l10(w, p)])
        }
        TeamKey::Hs(h) => {
            let h = h as usize;
            (Level::HighSchool, vec![l11(w, h), l12(w, h), l13(w, h), l14(w, h), l15(w, h)])
        }
        TeamKey::Other(_) => return None,
    })
}

/// "The NFL's five", "College's five", "High school's five".
pub fn five_title(level: Level) -> &'static str {
    match level {
        Level::Nfl => "The NFL's five",
        Level::College => "College's five",
        _ => "High school's five",
    }
}

/// The page's context line: "Iowa State · 14 seasons coaching, 4 as head
/// coach · Sun Dec 5, 2027".
pub fn context(w: &World) -> String {
    let Some((me, team)) = super::you(w) else { return String::new() };
    let c = &w.coaches[me];
    let span = |s: &crate::world::coach::Stint| (s.to - s.from).max(1);
    let mut all: i32 = c.history.iter().map(span).sum();
    let mut head: i32 = c.history.iter().filter(|s| s.role == crate::world::coach::Role::HeadCoach).map(span).sum();
    // Seasons with games at the job now (and its years before the career,
    // when the career took over a coach already in it).
    if let Some(j) = c.job {
        let n = super::ambitions::years_at(w, team) as i32;
        all += n;
        if j.role == crate::world::coach::Role::HeadCoach {
            head += n;
        }
    }
    let seasons = |n: i32| if n == 1 { "1 season".to_string() } else { format!("{n} seasons") };
    let as_head = if head > 0 { format!(", {head} as head coach") } else { String::new() };
    let career =
        if all == 0 { "first season coaching".to_string() } else { format!("{} coaching{as_head}", seasons(all)) };
    format!("{} · {career} · {}", super::tname(w, team), w.today.pretty())
}

fn money(x: f64) -> String {
    crate::world::front_office::dollars(x)
}

fn kv(k: &str, v: String, tone: u8) -> (String, String, u8) {
    (k.to_string(), v, tone)
}

fn short(w: &World, t: TeamKey) -> String {
    crate::world::stories::short_name(w, t)
}

fn person_name(w: &World, id: u32) -> Option<String> {
    crate::world::storylines::pi(w, id).map(|i| w.people[i].name.clone())
}

fn surname(name: &str) -> &str {
    name.rsplit(' ').next().unwrap_or(name)
}

/// The last thing a system paid, in words: "+0.75 into Evaluation in 2026".
fn last_paid(w: &World, sys: &str) -> Option<String> {
    let p = w.legacy.log.iter().rev().find(|p| p.sys == sys && p.lesson > 0.0)?;
    Some(format!("{}: +{:.2} into {}, {}", p.what, p.lesson, p.into, p.date.year()))
}

// ---------------------------------------------------------------------------
// The NFL: L1-L5
// ---------------------------------------------------------------------------

fn l1(w: &World, t: usize) -> Chase {
    let win = super::nfl::window(w, t);
    let (word, tone) = match win.open {
        4.. => ("Wide open", 0),
        3 => ("Open", 0),
        2 => ("Closing", 1),
        _ => ("Shut", 2),
    };
    let data = win.room.iter().map(|(y, r)| (y.to_string(), vec![(r / 1e6).max(0.0)])).collect();
    let tops = Some(win.room.iter().map(|(_, r)| money(*r)).collect());
    let choice = ["Hold", "Push in", "Reload"][win.choice.min(2) as usize];
    let now = win.room.first().map_or(0.0, |x| x.1);
    Chase {
        code: "L1",
        name: "The window".into(),
        word: word.into(),
        tone,
        home: "Front office › Cap".into(),
        line: format!("{} of room this league year; {}.", money(now), lower_first(&win.words)),
        vis: Vis::Bars { names: vec!["Cap room".into()], data, tops },
        kv: vec![
            kv("QB", win.qb.clone(), if win.open >= 3 { 0 } else { 1 }),
            kv("Core", format!("{} · {} in dead money", win.core, money(win.dead)), 1),
            kv(
                "Your call",
                format!(
                    "{choice} this league year · {} first-round {}, {} in all next spring",
                    win.firsts,
                    if win.firsts == 1 { "pick" } else { "picks" },
                    win.picks
                ),
                1,
            ),
        ],
    }
}

fn l2(w: &World, t: usize) -> Chase {
    let mut log: Vec<(i32, u8, u32)> = w.legacy.nfl.qb.clone();
    log.sort_by_key(|x| x.0);
    let now = super::nfl::qb_rank(w, t);
    let mut pts: Vec<(String, f64)> = log.iter().rev().take(8).rev().map(|x| (x.0.to_string(), x.1 as f64)).collect();
    let mut ids: Vec<u32> = log.iter().rev().take(8).rev().map(|x| x.2).collect();
    if let Some((rank, i)) = now {
        pts.push(("now".into(), rank as f64));
        ids.push(w.people[i].player.id);
    }
    let marks: Vec<(usize, String)> = (1..ids.len())
        .filter(|&k| ids[k] != ids[k - 1])
        .filter_map(|k| person_name(w, ids[k]).map(|n| (k, format!("{} arrives", surname(&n)))))
        .collect();
    let found = now.is_some_and(|x| x.0 <= 12) && w.legacy.nfl.search_since.is_none();
    let (word, tone) = match (found, w.legacy.nfl.search_since) {
        (true, _) => ("Found".to_string(), 0),
        (false, Some(s)) => (format!("Searching since {s}"), 2),
        (false, None) => ("Searching".to_string(), 2),
    };
    let (how, next) = match now {
        Some((rank, i)) => {
            let p = &w.people[i];
            let how = match p.pro.as_ref().and_then(|x| x.draft) {
                Some((y, r, pk, team)) if team == t => {
                    format!("{}: your pick, round {r} ({pk} overall) in {y}", p.name)
                }
                Some((y, r, _, _)) => format!("{}: a round {r} pick in {y}, from another club", p.name),
                None => format!("{}: undrafted", p.name),
            };
            let age = p.age_on(w.today);
            let next = if rank <= 12 {
                format!("{} is {age} and ranks {rank} of 32; the next search starts when he slips", surname(&p.name))
            } else {
                format!("{} ranks {rank} of 32: a top-12 starter ends the search", surname(&p.name))
            };
            (how, next)
        }
        None => ("No starter named".into(), "Name a starter on the depth chart".into()),
    };
    Chase {
        code: "L2",
        name: "Quarterback search".into(),
        line: if found { format!("Found: {how}.") } else { format!("{word}. {next}.") },
        word,
        tone,
        home: "Squad › Depth chart".into(),
        vis: if pts.is_empty() {
            Vis::Empty { text: "Your starter's league rank shows here after your first season.".into() }
        } else {
            Vis::Line {
                pts,
                lo: 1.0,
                hi: 32.0,
                invert: true,
                marks,
                note: "Your starter's rank among the 32 starters each season; first is at the top".into(),
            }
        },
        kv: vec![
            kv("How", how, 1),
            kv("Paid", last_paid(w, "L2").unwrap_or_else(|| "Nothing yet: finding him pays into Evaluation".into()), 0),
            kv("Next", next, 1),
        ],
    }
}

/// Hit rates by round group, yours and the league's: (label, hits, picks,
/// the league's share).
fn hit_groups(w: &World, t: usize, since: i32) -> Vec<(String, u32, u32, f64)> {
    let groups: [(u32, u32, &str); 3] = [(1, 2, "rounds 1-2"), (3, 4, "rounds 3-4"), (5, 7, "rounds 5-7")];
    let mut mine = [(0u32, 0u32); 3];
    let mut all = [(0u32, 0u32); 3];
    for p in &w.people {
        let Some(pro) = p.pro.as_ref() else { continue };
        let Some((y, round, _, team)) = pro.draft else { continue };
        if y < since || y > w.year - 2 || !(1..=7).contains(&round) {
            continue;
        }
        let Some(g) = groups.iter().position(|x| round >= x.0 && round <= x.1) else { continue };
        let starts: u32 = pro.seasons.iter().filter(|s| s.year < y + 3).map(|s| s.starts).sum();
        let hit = starts >= 16 || pro.seasons.iter().any(|s| s.pro_bowl);
        all[g].1 += 1;
        all[g].0 += hit as u32;
        if team == t {
            mine[g].1 += 1;
            mine[g].0 += hit as u32;
        }
    }
    groups
        .iter()
        .enumerate()
        .map(|(g, x)| (x.2.to_string(), mine[g].0, mine[g].1, all[g].0 as f64 / all[g].1.max(1) as f64))
        .collect()
}

fn letter_tone(l: &str) -> u8 {
    match l {
        "A" | "B+" => 0,
        "B" | "C+" => 1,
        _ => 2,
    }
}

fn l3(w: &World, t: usize) -> Chase {
    let mut classes: Vec<&super::nfl::Class> = w.legacy.nfl.classes.iter().filter(|c| c.team as usize == t).collect();
    classes.sort_by_key(|c| c.year);
    let score = |s: f32| ((s as f64 + 0.5) * 100.0).clamp(4.0, 100.0);
    let mut rows: Vec<(String, f64, String, u8)> = Vec::new();
    for c in classes.iter().rev().take(3).rev() {
        rows.push((
            format!("{}, draft night", c.year),
            score(c.night),
            c.night_letter.clone(),
            letter_tone(&c.night_letter),
        ));
        if let Some(r) = c.regrade {
            rows.push((
                format!("{}, three years on", c.year),
                score(r),
                c.regrade_letter.clone(),
                letter_tone(&c.regrade_letter),
            ));
        }
    }
    let since = w.coaches.get(super::you(w).map_or(0, |x| x.0)).and_then(|c| c.job).map_or(w.year - 6, |j| j.since);
    for (label, h, n, league) in hit_groups(w, t, since) {
        if n == 0 {
            continue;
        }
        let mine = h as f64 / n as f64;
        rows.push((
            format!("Hits, {label}"),
            (mine * 100.0).max(3.0),
            format!("{h} of {n} · league {:.0}%", league * 100.0),
            if mine >= league { 0 } else { 2 },
        ));
    }
    let last = classes.last();
    let (word, tone) = match last {
        Some(c) => match c.regrade {
            Some(_) => (format!("{} three years on", c.regrade_letter), letter_tone(&c.regrade_letter)),
            None => (format!("{} on the night", c.night_letter), letter_tone(&c.night_letter)),
        },
        None => ("No class yet".into(), 1),
    };
    let next = classes.iter().find(|c| c.regrade.is_none()).map_or_else(
        || "Each class is re-graded three years on, from starts, Pro Bowls and second contracts".into(),
        |c| format!("The {} class is re-graded on Sep 1, {}, from starts, Pro Bowls and contracts", c.year, c.year + 3),
    );
    Chase {
        code: "L3",
        name: "Draft classes".into(),
        line: match last {
            Some(c) => format!(
                "{} class: {} on draft night{}.",
                c.year,
                c.night_letter,
                c.regrade.map_or(String::new(), |_| format!(", {} three years on", c.regrade_letter))
            ),
            None => "Your first class is graded on draft night.".into(),
        },
        word,
        tone,
        home: "Draft › Your classes".into(),
        vis: if rows.is_empty() {
            Vis::Empty { text: "Your classes show here from your first draft.".into() }
        } else {
            Vis::Rows { rows, max: 100.0 }
        },
        kv: vec![
            kv("Re-grade", next, 1),
            kv(
                "Late hits",
                super::guys::late_note(w).unwrap_or_else(|| "None of your late picks starts yet".into()),
                0,
            ),
            kv("Risk", "Busts stay on your record; late hits earn your say in the building".into(), 2),
        ],
    }
}

fn l4(w: &World, t: usize) -> Chase {
    let trust = super::boss_trust(w);
    let mut pts: Vec<(String, f64)> = w
        .legacy
        .nfl
        .trust
        .iter()
        .rev()
        .take(7)
        .rev()
        .map(|x| (x.0.to_string(), (x.1 * 100.0).round() as f64))
        .collect();
    pts.push(("now".into(), (trust * 100.0).round() as f64));
    let words = super::nfl::leash_words(trust);
    let tone = if trust >= 0.65 {
        0
    } else if trust >= 0.45 {
        1
    } else {
        2
    };
    let owner = crate::world::folk::boss::boss(w, crate::world::folk::Team::Nfl(t)).name;
    let last = w.legacy.nfl.asks.last().map(|a| {
        let what = if a.kind == 1 {
            "a quarterback in round one".to_string()
        } else {
            let who = a.coach.map(|c| w.coaches[c as usize].name.clone()).unwrap_or_else(|| "a coordinator".into());
            format!("{who} gone")
        };
        let said = match a.answer {
            0 => "Waiting on your answer.",
            1 => "You gave him it.",
            2 => "You argued your case.",
            _ => "You refused.",
        };
        format!("{}: {owner} wanted {what}. {said}", a.date.pretty())
    });
    let risk = if trust < 0.45 {
        "Another losing December and he steps in"
    } else if trust < 0.65 {
        "He wants to be asked before big moves"
    } else {
        "He leaves the football to you while you win"
    };
    Chase {
        code: "L4",
        name: "The owner's leash".into(),
        word: words.into(),
        tone,
        home: "Front office › Owner".into(),
        line: format!("{owner}: {}. Trust {} of 100.", words.to_lowercase(), (trust * 100.0).round()),
        vis: Vis::Line {
            pts,
            lo: 0.0,
            hi: 100.0,
            invert: false,
            marks: Vec::new(),
            note: "The owner's trust in you, 0 to 100, at each season's end".into(),
        },
        kv: vec![
            kv("The leash", words.into(), tone),
            kv("Last ask", last.unwrap_or_else(|| "He hasn't asked for anything yet".into()), 1),
            kv("Risk", risk.into(), if trust < 0.45 { 2 } else { 1 }),
        ],
    }
}

fn l5(w: &World, t: usize) -> Chase {
    let done: Vec<i32> = {
        let mut v: Vec<i32> = w.nfl_seasons.iter().map(|s| s.year).collect();
        v.sort();
        v.dedup();
        v
    };
    let last = done.last().copied().unwrap_or(w.year - 1);
    let div = crate::world::franchise::era_in(w, t, w.year).map(|e| e.div);
    let mates: Vec<usize> = (0..w.nfl.len())
        .filter(|&x| div.is_some() && crate::world::franchise::era_in(w, x, w.year).map(|e| e.div) == div)
        .collect();
    let years: Vec<i32> = done.iter().rev().take(10).copied().collect();
    let champs: Vec<Vec<usize>> = years.iter().map(|&y| super::nfl::div_champs(w, y)).collect();
    let mut rows: Vec<(String, f64, String, u8)> = mates
        .iter()
        .map(|&x| {
            let n = champs.iter().filter(|c| c.contains(&x)).count();
            let words = if n == 1 { "1 division title".to_string() } else { format!("{n} division titles") };
            (short(w, TeamKey::Nfl(x as u16)), n as f64, words, if x == t { 0 } else { 1 })
        })
        .collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let max = rows.iter().map(|r| r.1).fold(1.0, f64::max);
    let (po, dv, _, sb) = super::nfl::runs(w, t, last);
    let (word, tone) = if sb >= 2 {
        (format!("{sb} rings running"), 0)
    } else if dv >= 2 {
        (format!("{dv} titles running"), 0)
    } else if po >= 1 {
        (format!("{} running", count(po, "playoff year")), 1)
    } else {
        ("Not yet".to_string(), 1)
    };
    let best = super::nfl::league_best_div_run(w).map_or_else(
        || "No division run yet in this world".to_string(),
        |(x, n, y)| format!("{}: {n} division titles in a row, to {y}", short(w, TeamKey::Nfl(x as u16))),
    );
    Chase {
        code: "L5",
        name: "Beating parity".into(),
        line: format!("{} running; {} running.", count(po, "playoff year"), count(dv, "division title")),
        word,
        tone,
        home: "Standings".into(),
        vis: if rows.is_empty() {
            Vis::Empty { text: "Your division's runs show here.".into() }
        } else {
            Vis::Rows { rows, max }
        },
        kv: vec![
            kv("Runs", format!("Playoffs {} running · division titles {dv} running", count(po, "year")), 1),
            kv("League's best", best, 1),
            kv("Pays", "Each new length of a run pays once, into Motivation; dynasties feed the Hall".into(), 0),
        ],
    }
}

// ---------------------------------------------------------------------------
// College: L6-L10
// ---------------------------------------------------------------------------

fn l6(w: &World, p: usize) -> Chase {
    let state = w.programs[p].state;
    let mut fences: Vec<super::college::Fence> =
        w.legacy.college.fence.iter().filter(|f| f.program == p).cloned().collect();
    let class = crate::world::recruiting::next_class(w);
    if !fences.iter().any(|f| f.year == class) {
        fences.push(super::college::fence_of(w, p, class));
    }
    fences.sort_by_key(|f| f.year);
    let fences: Vec<super::college::Fence> = fences.into_iter().rev().take(6).rev().collect();
    let rival = fences.iter().rev().find_map(|f| f.rival.map(|r| r.0));
    let rival_name = rival.map_or_else(|| "The rest".to_string(), |r| w.program_name(r).to_string());
    let state_name = crate::world::folk::facts::state_name(&w.states[state].code);
    // Fences saved before update 2's counts kept no top size: read them as 25.
    let size = |f: &super::college::Fence| if f.n == 0 { 25 } else { f.n };
    let data = fences
        .iter()
        .map(|f| {
            let theirs = f.rival.filter(|r| Some(r.0) == rival).map_or(0, |r| r.1);
            let gone = if f.n == 0 { size(f).saturating_sub(f.ours + theirs) } else { f.gone.saturating_sub(theirs) };
            let open = if f.n == 0 { 0 } else { f.open() };
            (f.year.to_string(), vec![f.ours as f64, theirs as f64, gone as f64, open as f64])
        })
        .collect();
    let tops = Some(fences.iter().map(|f| format!("{}/{}", f.ours, size(f))).collect());
    let now = fences.last();
    let (ours, top) = now.map_or((0, 25), |f| (f.ours, size(f)));
    let decided = now.map_or(0, |f| f.ours + f.gone);
    let share = now.map_or(0.0, |f| f.share());
    let (word, tone) = if decided == 0 {
        ("Waiting", 1)
    } else if share >= 0.5 {
        ("Fenced", 0)
    } else if share >= 0.25 {
        ("Contested", 1)
    } else {
        ("Raided", 2)
    };
    let lost = now.and_then(|f| f.lost.clone()).map_or_else(
        || format!("Nobody from the state's top {top} has gone elsewhere yet"),
        |(n, r, to)| format!("{n}, the state's No. {r}, to {}", w.program_name(to)),
    );
    let clinic = w.legacy.scheme.clinic_coaches.len();
    Chase {
        code: "L6",
        name: format!("Fencing {state_name}"),
        line: format!(
            "{ours} of the state's top {top} with you in the {class} class{}.",
            rival.map_or(String::new(), |_| format!("; {rival_name} the rival"))
        ),
        word: word.into(),
        tone,
        home: "Recruiting › Board".into(),
        vis: Vis::Bars {
            names: vec![
                w.program_name(p).to_string(),
                rival_name,
                "Elsewhere".into(),
                format!("Still deciding: {state_name}'s top {top}"),
            ],
            data,
            tops,
        },
        kv: vec![
            kv(
                "Holds it",
                if clinic > 0 {
                    format!("{clinic} high school coaches heard you at clinics")
                } else {
                    "Clinics and visits keep the state's coaches close".into()
                },
                0,
            ),
            kv("Lost", lost, 2),
            kv("Pays", "Recruiting lesson points and the boosters' pride, each class you hold".into(), 0),
        ],
    }
}

fn l7(w: &World, p: usize) -> Chase {
    let prog = &w.programs[p];
    match super::college::brand(w, p) {
        Some(b) => {
            let rows = b
                .bars
                .iter()
                .map(|(k, mine, avg)| {
                    let words = format!("{:.0} · the {}'s {:.0}", mine * 100.0, b.target, avg * 100.0);
                    (k.clone(), (mine * 100.0).clamp(0.0, 100.0), words, if mine >= avg { 0 } else { 2 })
                })
                .collect();
            let invited = w.legacy.college.invited.as_ref();
            let (word, tone) = if invited.is_some() {
                ("Invited", 0)
            } else if b.target == prog.conference {
                ("Settled", 1)
            } else if b.interest {
                ("Interest", 0)
            } else {
                ("Climbing", 1)
            };
            Chase {
                code: "L7",
                name: "Conference climb".into(),
                line: format!("{} of {} in the {} by brand.", ordinal(b.rank), b.members, b.target),
                word: word.into(),
                tone,
                home: "Team › Program".into(),
                vis: Vis::Rows { rows, max: 100.0 },
                kv: vec![
                    kv(
                        "Rank",
                        format!(
                            "{} of {} by brand: money and TV the school would bring the {}",
                            ordinal(b.rank),
                            b.members,
                            b.target
                        ),
                        1,
                    ),
                    kv("Next", b.next, 1),
                    kv("Risk", b.risk, 2),
                ],
            }
        }
        None => match super::college::brand_home(w, p) {
            // At the top already: where you stand inside your own conference.
            Some(b) => {
                let rows = b
                    .bars
                    .iter()
                    .map(|(k, mine, avg)| {
                        let words = format!("{:.0} · the {}'s {:.0}", mine * 100.0, b.target, avg * 100.0);
                        (k.clone(), (mine * 100.0).clamp(0.0, 100.0), words, if mine >= avg { 0 } else { 2 })
                    })
                    .collect();
                let low = b.rank * 4 > b.members * 3;
                Chase {
                    code: "L7",
                    name: "Conference climb".into(),
                    word: if low { "Slipping" } else { "Settled" }.into(),
                    tone: if low { 2 } else { 1 },
                    home: "Team › Program".into(),
                    line: format!(
                        "{} of {} in the {} by brand: the climb now is staying on top.",
                        ordinal(b.rank),
                        b.members,
                        b.target
                    ),
                    vis: Vis::Rows { rows, max: 100.0 },
                    kv: vec![
                        kv("Here", "Staying on top: the brand holds as long as you win".into(), 1),
                        kv(
                            "Risk",
                            format!(
                                "The bottom of the {}'s list is what realignment leaves behind (Oregon State and Washington State, 2024)",
                                b.target
                            ),
                            2,
                        ),
                    ],
                }
            }
            None => Chase {
                code: "L7",
                name: "Conference climb".into(),
                word: "Independent".into(),
                tone: 1,
                home: "Team › Program".into(),
                line: format!("{} plays as an independent at the top level.", w.program_name(p)),
                vis: Vis::Empty {
                    text: format!(
                        "{} answers to no conference: there is no climb to make, only a schedule to fill.",
                        w.program_name(p)
                    ),
                },
                kv: vec![kv("Here", "Your brand carries the schedule and the TV deal on its own".into(), 1)],
            },
        },
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_lowercase().collect::<String>() + c.as_str())
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn ordinal(n: u32) -> String {
    let s = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

fn l8(w: &World, p: usize) -> Chase {
    let (g, leaders, holds) = super::college::position_u(w, p);
    let name = super::college::u_name(g);
    let mine = leaders.iter().position(|x| x.0 == p);
    let mut rows: Vec<(String, f64, String, u8)> = leaders
        .iter()
        .take(5)
        .map(|&(q, n)| (w.program_name(q).to_string(), n as f64, format!("{n} drafted"), if q == p { 0 } else { 1 }))
        .collect();
    if let Some(k) = mine
        && k >= 5
    {
        let n = leaders[k].1;
        let rank = 1 + leaders.iter().filter(|x| x.1 > n).count() as u32;
        rows.push((w.program_name(p).to_string(), n as f64, format!("{n} · {}", ordinal(rank)), 0));
    }
    let max = rows.iter().map(|r| r.1).fold(1.0, f64::max);
    let (word, tone) = if holds {
        (name.to_string(), 0)
    } else if mine.is_some_and(|k| k < 3) {
        ("Rising".to_string(), 0)
    } else {
        ("Not yet".to_string(), 1)
    };
    // The name's holder as the world reads it after each draft (a clear lead).
    let gi = Group::ALL.iter().position(|&x| x == g).unwrap_or(0) as u8;
    let holder = w
        .legacy
        .college
        .u_names
        .iter()
        .find(|x| x.1 == gi && x.0 as usize != p)
        .map(|x| w.program_name(x.0 as usize).to_string())
        .unwrap_or_default();
    let group = g.name().to_lowercase();
    let one = match g {
        Group::QB => "quarterback",
        Group::RB => "running back",
        Group::Receivers => "receiver",
        Group::OL => "offensive line",
        Group::DL => "defensive line",
        Group::LB => "linebacker",
        Group::DB => "defensive back",
        Group::Specialists => "kicker",
    };
    let my_n = mine.map_or(0, |k| leaders[k].1);
    let my_rank = 1 + leaders.iter().filter(|x| x.1 > my_n).count() as u32;
    let tied = leaders.iter().filter(|x| x.1 == my_n).count() > 1;
    let next = w
        .legacy
        .guys
        .iter()
        .filter(|x| {
            x.pick.is_none()
                && !x.gone
                && Group::of(x.pos) == g
                && matches!(x.team, TeamKey::College(q) if q as usize == p)
        })
        .max_by(|a, b| a.ovr_out.total_cmp(&b.ovr_out))
        .map_or_else(
            || format!("Your next {group} drafted adds to the count"),
            |x| format!("{} in the next draft would add one", x.name),
        );
    Chase {
        code: "L8",
        name: format!("{name}?"),
        line: format!(
            "{my_n} {group} drafted in ten years{}.",
            mine.map_or(String::new(), |_| format!(
                ": {}{} in the country",
                if tied { "tied " } else { "" },
                ordinal(my_rank)
            ))
        ),
        word,
        tone,
        home: "Team › Traditions".into(),
        vis: if rows.is_empty() {
            Vis::Empty { text: "Your drafted players by position show here.".into() }
        } else {
            Vis::Rows { rows, max }
        },
        kv: vec![
            kv(
                "The name",
                if holds {
                    format!("\"{name}\" is yours in the press")
                } else if holder.is_empty() {
                    format!("Nobody holds \"{name}\": no school leads by two")
                } else {
                    format!("\"{name}\" is {holder}'s in the press")
                },
                1,
            ),
            kv("Next", next, 0),
            kv("Pays", format!("Top {one} recruits weigh it; teaching lesson points when you claim it"), 0),
        ],
    }
}

fn l9(w: &World, p: usize) -> Chase {
    let prog = &w.programs[p];
    let budget = prog.collective.budget;
    // Your split, or the collective's usual one until you set yours.
    let split = w.legacy.college.split;
    let set = split.iter().sum::<f32>() > 0.0;
    let total: f32 = split.iter().sum::<f32>().max(0.01);
    let shares: [f32; 3] = if set { split.map(|x| x / total) } else { [0.4, 0.3, 0.3] };
    let labels = ["Your stars' deals", "Transfers", "The new class"];
    let mut rows: Vec<(String, f64, String, u8)> = labels
        .iter()
        .zip(shares)
        .map(|(l, s)| {
            let x = budget * s as f64;
            (l.to_string(), x / 1e6, format!("{} · {:.0}%", money(x), s * 100.0), 1)
        })
        .collect();
    let split_words = format!(
        "{}: {:.0}% your stars, {:.0}% transfers, {:.0}% the class",
        if set { "Yours this year" } else { "The collective's usual" },
        shares[0] * 100.0,
        shares[1] * 100.0,
        shares[2] * 100.0
    );
    for s in w.legacy.college.strings.iter().filter(|s| s.date.year() >= w.year - 1) {
        let words = match s.answer {
            0 => "strings: waiting",
            1 => "strings: you gave",
            _ => "refused",
        };
        rows.push((
            format!("From {}", s.donor),
            s.amount / 1e6,
            format!("{} · {words}", money(s.amount)),
            if s.answer == 0 { 2 } else { 1 },
        ));
    }
    let max = rows.iter().map(|r| r.1).fold(0.1, f64::max);
    let mut raised: Vec<(i32, f64)> = w.legacy.college.raised.clone();
    raised.sort_by_key(|x| x.0);
    let now = raised.last().map_or(budget, |x| x.1);
    let before = raised.len().checked_sub(2).map(|k| raised[k].1);
    let waiting: Vec<&super::college::StringAsk> = w.legacy.college.strings.iter().filter(|s| s.answer == 0).collect();
    let (word, tone) = if !waiting.is_empty() {
        ("Strings", 2)
    } else if budget >= 5e6 {
        ("Strong", 0)
    } else if budget > 0.0 {
        ("Building", 1)
    } else {
        ("None yet", 1)
    };
    let strings = if waiting.is_empty() {
        "No donor has asked for a say".to_string()
    } else {
        waiting
            .iter()
            .map(|s| {
                let what = match s.kind {
                    0 => "a say in a hire",
                    1 => "a player's playing time",
                    _ => "a seat at practice",
                };
                format!("{} wants {what}", s.donor)
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    Chase {
        code: "L9",
        name: "Your collective".into(),
        line: format!(
            "{} this year{}.",
            money(now),
            if waiting.is_empty() { String::new() } else { format!("; {} want a say", waiting.len()) }
        ),
        word: word.into(),
        tone,
        home: "Recruiting › NIL desk".into(),
        vis: if budget <= 0.0 && rows.len() <= 3 {
            Vis::Empty { text: "No collective money yet: NIL opens in July 2021.".into() }
        } else {
            Vis::Rows { rows, max }
        },
        kv: vec![
            kv(
                "Raised",
                before.map_or_else(
                    || format!("{} this year", money(now)),
                    |b| format!("{} this year, from {} the year before", money(now), money(b)),
                ),
                0,
            ),
            kv("Split", split_words, 1),
            kv("Strings", strings, if waiting.is_empty() { 1 } else { 2 }),
            kv("Risk", "A promise you can't keep becomes a grudge".into(), 2),
        ],
    }
}

fn l10(w: &World, p: usize) -> Chase {
    let r = super::college::resume(w, p);
    let pts: Vec<(String, f64)> = r
        .ranks
        .iter()
        .map(|(d, k)| {
            let (_, m, day) = d.parts();
            const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
            (format!("{} {day}", M[(m as usize).saturating_sub(1) % 12]), *k as f64)
        })
        .collect();
    let last = r.ranks.last().map(|x| x.1);
    let best = r.ranks.iter().map(|x| x.1).min();
    let marks: Vec<(usize, String)> = match best {
        Some(b) => r.ranks.iter().position(|x| x.1 == b).map(|k| vec![(k, format!("best: {b}"))]).unwrap_or_default(),
        None => Vec::new(),
    };
    let (word, tone) = match last {
        Some(k) if k <= 12 => ("Inside", 0),
        Some(_) => ("Outside", 2),
        None => ("Unranked", 1),
    };
    Chase {
        code: "L10",
        name: "Playoff résumé".into(),
        line: {
            let body = match last {
                Some(k) => format!("committee rank {k}; {} quality wins, {} bad losses.", r.quality_wins, r.bad_losses),
                None => format!("not in the committee's top 25; {} quality wins.", r.quality_wins),
            };
            if r.season < w.year { format!("Last season ({}): {body}", r.season) } else { upper_first(&body) }
        },
        word: word.into(),
        tone,
        home: "Schedule".into(),
        vis: if pts.is_empty() {
            Vis::Empty { text: "The committee's weekly ranking shows here from its first poll in November.".into() }
        } else {
            Vis::Line {
                pts,
                lo: 1.0,
                hi: 25.0,
                invert: true,
                marks,
                note: "The committee's ranking each week; first is at the top".into(),
            }
        },
        kv: vec![
            kv(
                "Résumé",
                format!(
                    "Opponents' win share {:.3} · {} quality wins · {} bad losses",
                    r.sos, r.quality_wins, r.bad_losses
                ),
                1,
            ),
            kv(
                "Booked",
                if r.booked.is_empty() {
                    "No future non-conference games booked yet".into()
                } else {
                    r.booked.join(" · ")
                },
                1,
            ),
            kv("Pays", "Playoff money and recruits who want playoff football".into(), 0),
        ],
    }
}

// ---------------------------------------------------------------------------
// High school: L11-L15
// ---------------------------------------------------------------------------

fn l11(w: &World, h: usize) -> Chase {
    let hs = &w.legacy.hs;
    let mut log = hs.support_log.clone();
    log.sort_by_key(|x| x.0);
    let log: Vec<(i32, f32)> = log.into_iter().rev().take(9).rev().collect();
    let pts: Vec<(String, f64)> = log.iter().map(|x| (x.0.to_string(), x.1.round() as f64)).collect();
    let marks: Vec<(usize, String)> = hs
        .votes
        .iter()
        .filter_map(|v| {
            let k = log.iter().position(|x| x.0 == v.0)?;
            Some((
                k,
                if v.2 {
                    format!("bond passes, {:.0}%", v.1 * 100.0)
                } else {
                    format!("bond fails, {:.0}%", v.1 * 100.0)
                },
            ))
        })
        .collect();
    let s = hs.support;
    let word = super::hs::support_words(s);
    let tone = if s >= 55.0 {
        0
    } else if s >= 40.0 {
        1
    } else {
        2
    };
    let next = match &hs.bond {
        Some(b) => match b.stage {
            0 | 1 => format!("{}: the board decides on July 15", b.what),
            2 => format!("{}: on the ballot, {}", b.what, b.vote.map_or_else(|| "in November".into(), |d| d.pretty())),
            3 => format!("{}: passed; it opens in {}", b.what, b.opens),
            4 => format!("{} opened in {}", b.what, b.opens),
            5 => format!("{}: the vote failed; a second try costs more", b.what),
            _ => format!("{}: the superintendent waits on your answer", b.what),
        },
        None => "A bond for a new stadium needs the town behind you".into(),
    };
    let rival = crate::world::folk::facts::rival(w, crate::world::folk::Team::High(h)).map(|r| {
        let rt = crate::world::storylines::team_key(r.0);
        let (wn, l, _) = super::record_vs(w, Some(rt), None);
        (short(w, rt), wn, l)
    });
    let gift =
        w.front_office.school.boosters.ledger.iter().rev().find(|x| x.1 == "The town's year-end giving").map(|x| x.2);
    Chase {
        code: "L11",
        name: "Your town".into(),
        line: format!("Support {:.0} of 100: {}. {next}.", s, word.to_lowercase()),
        word: word.into(),
        tone,
        home: "School office".into(),
        vis: if pts.is_empty() {
            Vis::Empty { text: "The town's support shows here after your first season.".into() }
        } else {
            Vis::Line {
                pts,
                lo: 0.0,
                hi: 100.0,
                invert: false,
                marks,
                note: "The town's support, 0 to 100, at each season's end".into(),
            }
        },
        kv: vec![
            kv(
                "Pays",
                gift.map_or_else(
                    || "Support over 40 brings the booster club's year-end giving".into(),
                    |g| format!("{} from the town's year-end giving", money(g)),
                ),
                0,
            ),
            kv("Next", next, 1),
            kv(
                "Risk",
                match rival {
                    Some((n, wn, l)) if l > wn => format!("{wn}-{l} against {n}: lose again and the board remembers"),
                    Some((n, wn, l)) => format!("{wn}-{l} against {n}: the town counts that game twice"),
                    None => "Turn on the town and it turns on you: below 25 the principal's trust falls".into(),
                },
                2,
            ),
        ],
    }
}

/// How far a high school season went: "—", "R1", "QF", "SF", "Final", "Won".
fn round_label(y: &Yr, rounds: usize) -> (f64, String) {
    if y.champion {
        return (rounds as f64 + 1.0, "Won".into());
    }
    if y.final_game {
        return (rounds as f64, "Final".into());
    }
    let p = y.playoff as usize;
    if p == 0 {
        return (0.0, "—".into());
    }
    let label = match rounds.saturating_sub(p) {
        1 => "SF".to_string(),
        2 => "QF".to_string(),
        _ => format!("R{p}"),
    };
    (p as f64, label)
}

fn l12(w: &World, h: usize) -> Chase {
    let team = TeamKey::Hs(h as u32);
    let seasons = super::seasons_at(w, team);
    let fmt = crate::world::hs_playoffs::format(&w.states[w.high_schools[h].state].code);
    let rounds =
        crate::world::hs_playoffs::rounds_for(fmt.field_of(w.high_schools[h].classification as usize) as usize);
    let shown: Vec<&Yr> = seasons.iter().rev().take(9).rev().collect();
    let labels: Vec<(f64, String)> = shown.iter().map(|y| round_label(y, rounds)).collect();
    let data = shown.iter().zip(&labels).map(|(y, l)| (y.year.to_string(), vec![l.0])).collect();
    let tops = Some(labels.iter().map(|l| l.1.clone()).collect());
    let r = super::hs::runs(w, h);
    let (word, tone) = if r.titles > 0 {
        (if r.titles == 1 { "1 state title".to_string() } else { format!("{} state titles", r.titles) }, 0)
    } else if r.playoffs_row >= 3 {
        ("Building".to_string(), 1)
    } else {
        ("Not yet".to_string(), 1)
    };
    let enroll = w.high_schools[h].enrollment;
    Chase {
        code: "L12",
        name: "State title run".into(),
        line: format!(
            "Playoffs {} running; {}, {}.",
            count(r.playoffs_row, "year"),
            count(r.semis, "semifinal"),
            count(r.finals, "final")
        ),
        word,
        tone,
        home: "Team › History".into(),
        vis: if shown.is_empty() {
            Vis::Empty { text: "Your playoff runs show here after your first season.".into() }
        } else {
            Vis::Bars { names: vec!["Rounds reached".into()], data, tops }
        },
        kv: vec![
            kv(
                "Streaks",
                format!(
                    "Playoffs {} running · district titles {} running · {} straight wins (best {})",
                    count(r.playoffs_row, "year"),
                    r.districts,
                    r.streak,
                    r.best_streak
                ),
                1,
            ),
            kv("Your class", format!("Enrollment {enroll}; classes are recounted on the state's own cycle"), 1),
            kv("Pays", "Banners, a statewide name, college coaches stopping by".into(), 0),
        ],
    }
}

fn l13(w: &World, h: usize) -> Chase {
    let mut log: Vec<(i32, [u16; 6])> = w.legacy.hs.placement.clone();
    let class = crate::world::recruiting::next_class(w);
    if !log.iter().any(|x| x.0 == class) {
        log.push((class, super::hs::signees(w, h, class)));
    }
    log.sort_by_key(|x| x.0);
    let shown: Vec<&(i32, [u16; 6])> = log.iter().rev().take(9).rev().collect();
    let data = shown
        .iter()
        .map(|(y, s)| (y.to_string(), vec![s[0] as f64, s[1] as f64, s[2] as f64, (s[3] + s[4]) as f64, s[5] as f64]))
        .collect();
    let all: u32 = log.iter().map(|x| x.1.iter().map(|&n| n as u32).sum::<u32>()).sum();
    let d1: u32 = log.iter().map(|x| x.1[0] as u32 + x.1[1] as u32).sum();
    let fbs: u32 = log.iter().map(|x| x.1[0] as u32).sum();
    let (word, tone) = if fbs >= 3 || d1 >= 6 {
        ("Trusted", 0)
    } else if all >= 5 {
        ("Growing", 1)
    } else {
        ("Unknown", 1)
    };
    let this = log.iter().find(|x| x.0 == class).map_or(0, |x| x.1.iter().map(|&n| n as u32).sum::<u32>());
    let last_fbs = super::hs::last_fbs_signee(w, h);
    Chase {
        code: "L13",
        name: "Sending kids up".into(),
        line: format!("{}, {d1} to Division I; {this} so far in the {class} class.", count(all, "signee")),
        word: word.into(),
        tone,
        home: "Scouting › High school side".into(),
        vis: Vis::Bars {
            names: vec![
                "FBS".into(),
                "FCS".into(),
                "Division II".into(),
                "D3 and NAIA".into(),
                "Junior college".into(),
            ],
            data,
            tops: None,
        },
        kv: vec![
            kv("Record", format!("{} under you, {d1} to Division I, {fbs} to the FBS", count(all, "signee")), 0),
            kv(
                "Last FBS",
                last_fbs.map_or_else(
                    || "Nobody from here has signed with an FBS school yet".into(),
                    |y| format!("The last FBS signee was the class of {y}"),
                ),
                1,
            ),
            kv("Pays", "College coaches stop by; your word on a kid carries more each time it holds".into(), 0),
        ],
    }
}

fn l14(w: &World, _h: usize) -> Chase {
    let f = &w.legacy.hs.feeders;
    let rows: Vec<(String, f64, String, u8)> = f
        .iter()
        .map(|x| {
            let (words, tone) = if x.open < 0.15 && x.adopt < 0.2 {
                ("coach says no", 2)
            } else if x.adopt >= 0.75 {
                ("runs your terms", 0)
            } else if x.adopt >= 0.5 {
                ("most of them", 0)
            } else if x.adopt >= 0.3 {
                ("about half", 1)
            } else {
                ("not yet", 1)
            };
            (x.name.clone(), (x.adopt * 100.0) as f64, words.to_string(), tone)
        })
        .collect();
    let adopted = f.iter().filter(|x| x.adopt >= 0.5).count();
    let (word, tone) = match adopted {
        3.. => ("Strong", 0),
        1.. => ("Growing", 1),
        _ => ("Not yet", 1),
    };
    let gave: Vec<String> =
        f.iter().filter(|x| x.playbook || x.clinic || x.assistant || x.equipment).map(|x| x.name.clone()).collect();
    Chase {
        code: "L14",
        name: "Your feeders".into(),
        line: format!("{adopted} of {} feeder teams run your system.", f.len()),
        word: word.into(),
        tone,
        home: "School and feeders".into(),
        vis: if rows.is_empty() {
            Vis::Empty { text: "Your feeder schools show here from June.".into() }
        } else {
            Vis::Rows { rows, max: 100.0 }
        },
        kv: vec![
            kv("Payoff", "Freshmen from a feeder on your terms know part of your playbook on day one".into(), 0),
            kv(
                "Given",
                if gave.is_empty() {
                    "Nothing yet: your playbook, a clinic, an assistant or equipment".into()
                } else {
                    format!("Your help so far: {}", super::and_list(&gave))
                },
                1,
            ),
            kv("Risk", "A coach who won't take it stays his own; help stopped fades in a year".into(), 2),
        ],
    }
}

fn l15(w: &World, h: usize) -> Chase {
    let r = super::hs::runs(w, h);
    let placed: u32 = w.legacy.hs.placement.iter().map(|x| x.1[0] as u32 + x.1[1] as u32).sum();
    let named = w.legacy.scheme.named.as_ref();
    let talks = w.legacy.hs.talks.len() as u32;
    let camps = w.legacy.hs.camps.len() as u32;
    let me = super::you(w).map_or(0, |x| x.0);
    let ties = (w.coaches[me].ability.network / 20.0 * 100.0).round();
    let rows = vec![
        (
            "State titles".to_string(),
            (r.titles as f64 * 34.0).clamp(3.0, 100.0),
            if r.titles == 0 { "none yet".to_string() } else { format!("{}", r.titles) },
            if r.titles == 0 { 2 } else { 0 },
        ),
        (
            "Placement".to_string(),
            (placed as f64 * 12.0).clamp(3.0, 100.0),
            format!("{placed} to Division I"),
            if placed >= 3 { 0 } else { 1 },
        ),
        (
            "Scheme".to_string(),
            if named.is_some() { 70.0 } else { 15.0 },
            named.map_or_else(|| "not named".to_string(), |n| format!("named in {}", n.year)),
            if named.is_some() { 0 } else { 1 },
        ),
        (
            "Clinics and camps".to_string(),
            ((talks + camps) as f64 * 15.0).clamp(3.0, 100.0),
            format!("{}, {}", count(talks, "talk"), count(camps, "camp job")),
            if talks + camps >= 2 { 0 } else { 1 },
        ),
        (
            "College ties".to_string(),
            ties.clamp(3.0, 100.0),
            match w.coaches[me].ability.network {
                x if x >= 16.0 => "wide".to_string(),
                x if x >= 12.0 => "good".to_string(),
                x if x >= 8.0 => "some".to_string(),
                _ => "few".to_string(),
            },
            1,
        ),
    ];
    let offers: Vec<String> = w.career.as_ref().map_or(Vec::new(), |c| {
        let mut v: Vec<String> = c
            .market
            .offers
            .iter()
            .filter(|o| matches!(o.employer, Employer::College(_)))
            .map(|o| {
                format!(
                    "{} {}, {}",
                    super::tname(w, super::team_of_employer(o.employer).unwrap_or(TeamKey::Other(0))),
                    o.role.name(),
                    o.made.year()
                )
            })
            .collect();
        v.extend(c.market.passed.iter().filter(|x| matches!(x.0, Employer::College(_))).map(|x| {
            format!(
                "{} {}, {} (declined)",
                super::tname(w, super::team_of_employer(x.0).unwrap_or(TeamKey::Other(0))),
                x.1.name(),
                x.2.year()
            )
        }));
        v
    });
    let (word, tone) = if !offers.is_empty() {
        ("Open door", 0)
    } else if talks + camps > 0 {
        ("Knocking", 1)
    } else {
        ("Closed", 1)
    };
    Chase {
        code: "L15",
        name: "The jump".into(),
        line: if offers.is_empty() {
            format!("{}, {}; no college offer yet.", count(talks, "clinic talk"), count(camps, "camp job"))
        } else {
            format!("College calls: {}.", offers.join("; "))
        },
        word: word.into(),
        tone,
        home: "Job market".into(),
        vis: Vis::Rows { rows, max: 100.0 },
        kv: vec![
            kv("Offers", if offers.is_empty() { "None yet".into() } else { offers.join(" · ") }, 1),
            kv("The choice", "A college start pays less and leaves your town and your kids".into(), 1),
            kv("Pays", "A college career; your high school years still count".into(), 0),
        ],
    }
}
