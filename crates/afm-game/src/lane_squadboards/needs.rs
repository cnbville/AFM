//! V2, the needs map in the stadium: every starting spot now, next fall
//! and two falls out, by who stays (eligibility, graduation, contracts),
//! who likely leaves early (the draft) and who arrives (your commits), each
//! man's read carried forward by how players his age grow in this world.
//! Each spot gets a verdict against the level's starters, why it changes
//! and how to fill it. The staff's guess from its reads, never a ceiling.

use std::collections::HashMap;

use afm_engine::positions::Position;
use afm_engine::world::norms::{Marks, Rated};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::{first_last, growth_at, growth_table, level_name, marks, mid, my_team, own_read};
use crate::Reply;
use crate::reads::{self, Read};
use crate::views::{self, TeamRef};

/// The starting spots: key, position, words, and where each stands
/// (yards: depth from the line, across from the middle).
const SPOTS: [(&str, Position, &str, f64, f64); 24] = {
    use Position::*;
    [
        ("QB", QB, "Quarterback", -5.0, 0.0),
        ("RB", RB, "Running back", -7.5, 0.0),
        ("X", WR, "Wide receiver (X)", 0.0, -21.0),
        ("Z", WR, "Wide receiver (Z)", -1.0, 21.0),
        ("SLOT", WR, "Slot receiver", -1.8, 13.0),
        ("TE", TE, "Tight end", 0.0, 8.0),
        ("LT", OT, "Left tackle", 0.0, -4.4),
        ("LG", IOL, "Left guard", 0.0, -2.2),
        ("C", IOL, "Center", 0.0, 0.0),
        ("RG", IOL, "Right guard", 0.0, 2.2),
        ("RT", OT, "Right tackle", 0.0, 4.4),
        ("LE", EDGE, "Left end", 1.0, -5.5),
        ("DT1", DT, "Defensive tackle", 1.0, -1.5),
        ("DT2", DT, "Defensive tackle", 1.0, 1.5),
        ("RE", EDGE, "Right end", 1.0, 5.5),
        ("WILL", LB, "Weakside linebacker", 4.5, -5.0),
        ("MIKE", LB, "Middle linebacker", 5.0, 0.0),
        ("SAM", LB, "Strongside linebacker", 4.5, 5.0),
        ("CB1", CB, "Cornerback", 6.0, -19.0),
        ("CB2", CB, "Cornerback", 6.0, 19.0),
        ("FS", S, "Free safety", 13.0, -6.0),
        ("SS", S, "Strong safety", 10.0, 6.0),
        ("K", K, "Kicker", -10.0, -12.0),
        ("P", P, "Punter", -10.0, 12.0),
    ]
};

/// A man the map counts: his read carried to each of the three falls
/// (None: gone by then), and why he goes.
struct Man {
    i: usize,
    pos: Position,
    vals: [Option<f64>; 3],
    read: Read,
    commit: bool,
    gone: Option<(usize, String)>,
    draft_risk: bool,
}

/// A verdict against the level's marks at a position.
fn verdict(v: Option<f64>, m: Option<&Marks>) -> &'static str {
    let (Some(v), Some(m)) = (v, m) else { return if v.is_none() { "Hole" } else { "Average" } };
    if v >= (m.starter + m.elite) / 2.0 {
        "Strength"
    } else if v >= m.starter {
        "Solid"
    } else if v >= (m.backup + m.starter) / 2.0 {
        "Average"
    } else if v >= m.backup {
        "Thin"
    } else {
        "Hole"
    }
}

/// His class in a fall to come: "Jr", "RS So", "Sr"; a pro's age.
fn class_in(w: &World, i: usize, fall: i32) -> String {
    let p = &w.people[i];
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool if !is_commit(w, i) => {
            ["Sr", "Jr", "So", "Fr"][(p.class_of - fall - 1).clamp(0, 3) as usize].to_string()
        }
        Stage::HighSchool | Stage::PrepSchool | Stage::College => {
            let y = (fall - p.class_of).clamp(0, 4) as usize;
            if p.redshirt_used && y > 0 {
                ["Fr", "RS Fr", "RS So", "RS Jr", "RS Sr"][y].to_string()
            } else {
                ["Fr", "So", "Jr", "Sr", "Sr"][y].to_string()
            }
        }
        _ => format!("age {}", p.player.age as i32 + (fall - w.year)),
    }
}

fn is_commit(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    let Some(TeamRef::College(c)) = views::mine(w) else { return false };
    matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
        && (p.cal.signed.is_some_and(|s| s.program == c) || p.cal.commit.is_some_and(|x| x.0 == c))
}

/// Carries a read forward by the median growth at his age and level.
fn carry(table: &HashMap<(u32, u8), [f64; 5]>, start: f64, age: u32, tiers: &[u8]) -> Vec<f64> {
    let mut v = vec![start];
    let mut x = start;
    for (k, &tier) in tiers.iter().enumerate() {
        x = (x + growth_at(table, age + k as u32, tier)[2]).clamp(1.0, 99.0);
        v.push(x);
    }
    v
}

pub(crate) fn view(w: &World, args: &Value) -> Reply {
    let t = my_team(w)?;
    let f0 = w.year;
    let falls = [f0, f0 + 1, f0 + 2];
    let year = args.get("year").and_then(Value::as_u64).unwrap_or(0).min(2) as usize;
    let table = growth_table(w);
    let level_tier = match t {
        TeamRef::HighSchool(_) => 0,
        TeamRef::Nfl(_) => 2,
        _ => 1,
    };
    let five = w.changes.rules(f0).five_in_five;
    let limit: i32 = if five { 5 } else { 4 };
    // Who counts: your roster and, at a college, your commits.
    let mut people: Vec<(usize, bool)> = views::roster_people(w, t).into_iter().map(|i| (i, false)).collect();
    if let TeamRef::College(_) = t {
        for (i, _) in
            w.people.iter().enumerate().filter(|(_, p)| matches!(p.stage, Stage::HighSchool | Stage::PrepSchool))
        {
            if is_commit(w, i) && w.people[i].class_of <= f0 + 2 {
                people.push((i, true));
            }
        }
    }
    let mut men: Vec<Man> = Vec::new();
    for (i, commit) in people {
        let p = &w.people[i];
        let pos = p.player.position;
        let read = if commit { reads::overall(w, i) } else { own_read(w, t, i) };
        let Some(m0) = mid(read) else { continue };
        let age = p.player.age;
        // Each fall's level for his growth, and whether he plays then.
        let mut tiers = [level_tier; 2];
        let mut here = [true; 3];
        let mut gone: Option<(usize, String)> = None;
        let mut draft_risk = false;
        match p.stage {
            _ if commit => {
                for (n, fall) in falls.iter().enumerate() {
                    here[n] = p.class_of <= *fall;
                }
                for (n, tier) in tiers.iter_mut().enumerate() {
                    *tier = if p.class_of <= falls[n] { 1 } else { 0 };
                }
            }
            Stage::HighSchool | Stage::PrepSchool => {
                for (n, fall) in falls.iter().enumerate() {
                    here[n] = p.class_of > *fall;
                }
                if let Some(n) = here.iter().position(|h| !h) {
                    gone = Some((n, format!("graduates in {}", p.class_of)));
                }
            }
            Stage::College => {
                let left = (limit - p.college_seasons as i32).min(p.class_of + 5 - f0).max(0);
                for (n, h) in here.iter_mut().enumerate() {
                    *h = left > n as i32;
                }
                if let Some(n) = here.iter().position(|h| !h) {
                    gone = Some((n, format!("runs out of eligibility after {}", falls[n] - 1)));
                }
            }
            _ => {
                let through = p.pro.as_ref().and_then(|c| c.contract.as_ref()).map_or(f0 - 1, |k| k.last_year());
                for (n, fall) in falls.iter().enumerate() {
                    here[n] = through >= *fall;
                }
                if let Some(n) = here.iter().position(|h| !h) {
                    gone = Some((
                        n,
                        if through < f0 {
                            "has no contract".to_string()
                        } else {
                            format!("is under contract only through {through}")
                        },
                    ));
                }
            }
        }
        let path = carry(&table, m0, age, &tiers);
        let mut vals: [Option<f64>; 3] = [0, 1, 2].map(|n| here[n].then_some(path[n]));
        // A college player among the level's best leaves early for the
        // draft once three years out of high school.
        if p.stage == Stage::College
            && let Some(m) = marks(w, t, pos, Rated::Overall)
        {
            for n in 0..2 {
                let season = falls[n];
                if vals[n].is_some_and(|v| v >= m.elite) && vals[n + 1].is_some() && season >= p.class_of + 2 {
                    for v in vals.iter_mut().skip(n + 1) {
                        *v = None;
                    }
                    draft_risk = true;
                    gone = Some((n + 1, format!("is likely to declare for the draft after {season}")));
                    break;
                }
            }
        }
        men.push(Man { i, pos, vals, read, commit, gone, draft_risk });
    }
    // Starters each fall: now by the depth order (yours, else your
    // staff's), later by the carried reads.
    let order: Vec<(Position, Vec<u32>)> = {
        let mut o = w.seat_game_depth().unwrap_or_default();
        for (p, ids) in &w.seat_depth {
            o.retain(|x| x.0 != *p);
            o.push((*p, ids.clone()));
        }
        o
    };
    let mut starters: Vec<[Option<usize>; 3]> = vec![[None; 3]; SPOTS.len()];
    #[allow(clippy::needless_range_loop)] // n is the year, read from every man too
    for n in 0..3 {
        let mut used: Vec<usize> = Vec::new();
        for (k, &(_, pos, _, _, _)) in SPOTS.iter().enumerate() {
            let mut cands: Vec<(f64, usize)> = men
                .iter()
                .enumerate()
                .filter(|(_, m)| m.pos == pos && !used.contains(&m.i))
                .filter_map(|(j, m)| m.vals[n].map(|v| (v, j)))
                .collect();
            cands.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut pick = cands.first().map(|x| x.1);
            if n == 0
                && let Some((_, ids)) = order.iter().find(|x| x.0 == pos)
            {
                let by_order =
                    ids.iter().find_map(|id| cands.iter().find(|c| w.people[men[c.1].i].player.id == *id).map(|c| c.1));
                pick = by_order.or(pick);
            }
            if let Some(j) = pick {
                used.push(men[j].i);
                starters[k][n] = Some(j);
            }
        }
    }
    let marks_at: HashMap<Position, Option<Marks>> =
        SPOTS.iter().map(|s| (s.1, marks(w, t, s.1, Rated::Overall))).collect();
    let name_of = |j: usize| first_last(&w.people[men[j].i].name).1;
    let full_of = |j: usize| w.people[men[j].i].name.clone();
    let mut spots: Vec<Value> = Vec::new();
    for (k, &(key, pos, words, x, y)) in SPOTS.iter().enumerate() {
        let m = marks_at.get(&pos).and_then(|x| x.as_ref());
        let vals: Vec<Option<f64>> = (0..3).map(|n| starters[k][n].and_then(|j| men[j].vals[n])).collect();
        let verdicts: Vec<&str> = vals.iter().map(|v| verdict(*v, m)).collect();
        let names: Vec<Value> = (0..3)
            .map(|n| {
                starters[k][n].map_or(Value::Null, |j| {
                    json!({ "id": w.people[men[j].i].player.id, "name": full_of(j), "class": class_in(w, men[j].i, falls[n]), "commit": men[j].commit })
                })
            })
            .collect();
        // Why it changes.
        let mut why: Vec<String> = Vec::new();
        if let Some(j) = starters[k][0] {
            if let Some((n, reason)) = &men[j].gone {
                why.push(format!("{} {reason}.", full_of(j)));
                if let Some(j2) = starters[k][*n] {
                    let mj = &men[j2];
                    let who = if mj.commit {
                        let st = stars(w, mj.i);
                        format!("{} ({} {st} commit)", full_of(j2), if st.starts_with('u') { "an" } else { "a" })
                    } else {
                        full_of(j2)
                    };
                    why.push(format!("{who} takes over in {}.", falls[*n]));
                } else {
                    why.push(format!("Nobody on the roster is ready to take over in {}.", falls[*n]));
                }
            } else {
                let g = vals[2].zip(vals[0]).map(|(b, a)| (b - a) / 2.0);
                match g {
                    Some(g) if g >= 1.5 => {
                        why.push(format!("{} should keep growing, about {g:.0} a year at his age.", full_of(j)))
                    }
                    Some(g) if g <= -1.0 => {
                        why.push(format!("{} is past his best years: about {:.0} a year down.", full_of(j), -g))
                    }
                    _ => why.push(format!("{} holds the spot all three falls.", full_of(j))),
                }
            }
        } else {
            why.push("Nobody plays here now.".into());
        }
        // How to fill it.
        let mut fix: Vec<String> = Vec::new();
        let weak_later = verdicts.iter().skip(1).any(|v| matches!(*v, "Hole" | "Thin"));
        let commits: Vec<usize> = (0..men.len()).filter(|&j| men[j].commit && men[j].pos == pos).collect();
        for &j in commits.iter().take(2) {
            let class = w.people[men[j].i].class_of;
            let st = stars(w, men[j].i);
            let a = if st.starts_with('u') { "an" } else { "a" };
            fix.push(format!("{}, {a} {st} commit, arrives in fall {class}.", full_of(j)));
        }
        if let Some(j) = starters[k][0]
            && w.people[men[j].i].mood <= -0.15
            && men[j].vals[1].is_some()
        {
            fix.push(match t {
                TeamRef::College(_) => {
                    format!("Talk with {}: he's unhappy, and the portal opens in December.", name_of(j))
                }
                _ => format!("Talk with {}: he's unhappy.", name_of(j)),
            });
        }
        if weak_later {
            // A man at another position your staff reads as a fit here.
            if let Some((who, lo, hi, from)) = best_move(w, t, &men, &starters, pos, m) {
                fix.push(format!("Move {who} from {from}: your staff reads him {lo}-{hi} here."));
            }
            fix.push(match t {
                TeamRef::College(_) if commits.is_empty() => format!(
                    "No {} commits yet for the class of {}. The portal fills a spot at once: a transfer with two years left.",
                    pos.name(),
                    f0 + 1
                ),
                TeamRef::College(_) => "The portal fills a spot at once: a transfer with two years left.".into(),
                TeamRef::Nfl(_) => {
                    format!("The draft or free agency: plan a {} for {}.", pos.name(), falls[1])
                }
                _ => "Next year's freshmen come up in August: your staff sees them then.".into(),
            });
        }
        if fix.is_empty() {
            fix.push("Nothing to do.".into());
        }
        spots.push(json!({
            "key": key, "pos": pos.name(), "full": words, "x": x, "y": y,
            "vals": vals.iter().map(|v| v.map(f64::round)).collect::<Vec<_>>(),
            "verdicts": verdicts, "names": names, "why": why.join(" "), "fix": fix.join(" "),
        }));
    }
    // The three falls in a line each.
    let years: Vec<Value> = (0..3)
        .map(|n| {
            let vs: Vec<f64> = spots.iter().filter_map(|s| s["vals"][n].as_f64()).collect();
            let avg = if vs.is_empty() { None } else { Some((vs.iter().sum::<f64>() / vs.len() as f64).round()) };
            let holes = spots.iter().filter(|s| s["verdicts"][n] == "Hole").count();
            let thin = spots.iter().filter(|s| s["verdicts"][n] == "Thin").count();
            json!({ "fall": falls[n], "label": (["Now", "Next fall", "Two falls out"][n]), "avg": avg, "holes": holes, "thin": thin })
        })
        .collect();
    // Who leaves after this season, who may, who arrives.
    let names_list = |v: &[usize]| v.iter().take(6).map(|&j| full_of(j)).collect::<Vec<_>>();
    let last: Vec<usize> = (0..men.len())
        .filter(|&j| !men[j].commit && men[j].gone.as_ref().is_some_and(|g| g.0 == 1) && !men[j].draft_risk)
        .collect();
    let draft: Vec<usize> = (0..men.len()).filter(|&j| men[j].draft_risk).collect();
    let portal: Vec<usize> = (0..men.len())
        .filter(|&j| {
            matches!(t, TeamRef::College(_))
                && !men[j].commit
                && w.people[men[j].i].mood <= -0.15
                && men[j].vals[1].is_some()
        })
        .collect();
    let arriving: Vec<usize> =
        (0..men.len()).filter(|&j| men[j].commit && w.people[men[j].i].class_of == f0 + 1).collect();
    let leaving_label = match t {
        TeamRef::HighSchool(_) => "Seniors",
        TeamRef::Nfl(_) => "Contracts ending",
        _ => "Out of eligibility",
    };
    let mut leaving = vec![json!({ "label": leaving_label, "n": last.len(), "names": names_list(&last) })];
    if let TeamRef::College(_) = t {
        leaving.push(json!({ "label": "Draft risks", "n": draft.len(), "names": names_list(&draft) }));
        leaving.push(json!({ "label": "Portal risks", "n": portal.len(), "names": names_list(&portal) }));
        let label = format!("Commits for {}", f0 + 1);
        leaving.push(json!({ "label": label, "n": arriving.len(), "names": names_list(&arriving) }));
    }
    let roster_n = views::roster_people(w, t).len();
    let roster_line = match t {
        TeamRef::College(c) => {
            let (lim, _) = afm_engine::world::ncaa::roster_limits(&w.programs[c], f0);
            let commits = men.iter().filter(|m| m.commit).count();
            format!("{roster_n} on the roster of {lim} · {commits} commit{}", if commits == 1 { "" } else { "s" })
        }
        TeamRef::Nfl(n) => {
            // The front office's own count: the men on the roster, not the
            // practice squad or the injured list.
            use afm_engine::world::front_office::roster;
            let on = roster::count(w, n, afm_engine::world::pro::Status::Roster);
            format!("{on} on the roster of {}", roster::roster_limit(w))
        }
        _ => format!("{roster_n} on the roster"),
    };
    let now_note = match (w.today >= in_season_from(w, t), views::next_game(w, t).is_some()) {
        (false, _) => "now, before the fall",
        (true, true) => "now, in the season",
        (true, false) => "now, the season over",
    };
    Ok(json!({
        "team": views::team_brief(w, t),
        "level": level_name(w, t),
        "kind": match t { TeamRef::HighSchool(_) => "hs", TeamRef::Nfl(_) => "nfl", _ => "college" },
        "year": year,
        "falls": falls,
        "notes": [now_note, if matches!(t, TeamRef::College(_)) { "next fall, with commits" } else { "next fall" }, if matches!(t, TeamRef::College(_)) { "two falls out, with commits" } else { "two falls out" }],
        "roster_line": roster_line,
        "spots": spots,
        "years": years,
        "leaving": leaving,
        "draft_note": "A draft risk is a player among the level's best a year from being eligible: the staff counts him gone.",
    }))
}

/// The best man at another position your staff reads as a fit at `pos`
/// (not a starter at his own spot that fall): name, his read there, and
/// where he plays now.
fn best_move(
    w: &World,
    t: TeamRef,
    men: &[Man],
    starters: &[[Option<usize>; 3]],
    pos: Position,
    m: Option<&Marks>,
) -> Option<(String, u8, u8, &'static str)> {
    let m = m?;
    let reader = views::reader_key(w);
    let starting: Vec<usize> = starters.iter().filter_map(|s| s[1]).collect();
    men.iter()
        .enumerate()
        .filter(|(j, x)| !x.commit && x.pos != pos && x.vals[1].is_some() && !starting.contains(j))
        .filter(|(_, x)| !x.pos.is_specialist() && !pos.is_specialist())
        .filter_map(|(_, x)| {
            let p = &w.people[x.i];
            let eye = views::eye_for(w, t, p.player.position);
            let half = 2.0 + (20.0 - eye.clamp(1.0, 20.0)) * 0.12;
            let v = afm_engine::positions::overall_for(pos, &p.player.attributes);
            match reads::range(v, half + 1.0, reader, p.player.id, 300 + pos.index()) {
                Read::Range(lo, hi)
                    if (lo as f64 + hi as f64) / 2.0 >= m.backup.max(mid(x.read).unwrap_or(0.0) - 6.0) =>
                {
                    Some(((lo as f64 + hi as f64) / 2.0, p.name.clone(), lo, hi, x.pos.name()))
                }
                _ => None,
            }
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|x| (x.1, x.2, x.3, x.4))
}

fn stars(w: &World, i: usize) -> String {
    let s = w.people[i].stars;
    if s > 0 { format!("{s}-star") } else { "unrated".into() }
}

/// The day the season's work starts at your level (camp, report day, the
/// school year).
fn in_season_from(w: &World, t: TeamRef) -> afm_engine::calendar::Date {
    match t {
        TeamRef::Nfl(_) => w.dates.veterans_report,
        TeamRef::HighSchool(_) => w.dates.hs_school_start,
        _ => w.dates.fall_camp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks_of(backup: f64, starter: f64, elite: f64) -> Marks {
        Marks {
            pool: afm_engine::world::norms::Pool::Nfl,
            pos: Position::QB,
            backup,
            starter,
            elite,
            starters: vec![],
            backups: 0,
            teams: 0,
        }
    }

    #[test]
    fn verdicts_follow_the_marks() {
        let m = marks_of(60.0, 70.0, 80.0);
        assert_eq!(verdict(Some(77.0), Some(&m)), "Strength");
        assert_eq!(verdict(Some(71.0), Some(&m)), "Solid");
        assert_eq!(verdict(Some(66.0), Some(&m)), "Average");
        assert_eq!(verdict(Some(61.0), Some(&m)), "Thin");
        assert_eq!(verdict(Some(50.0), Some(&m)), "Hole");
        assert_eq!(verdict(None, Some(&m)), "Hole");
    }
}
