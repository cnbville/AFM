//! V2, the needs map in the stadium: every starting spot now, next fall
//! and two falls out, by who stays (eligibility, graduation, contracts),
//! who likely leaves early (the draft) and who arrives (your commits), each
//! man's read carried forward by how players his age grow in this world.
//! Each spot gets a verdict against the level's starters, why it changes
//! and how to fill it. The staff's guess from its reads, never a ceiling.
//!
//! A college player in the transfer portal counts as gone (Cain
//! 2026-11-12): he is off every fall the map plans, and the map says the
//! place he would have had if he had stayed, from the same reads and the
//! same picking.

use std::collections::HashMap;

use afm_engine::league::depth_slots;
use afm_engine::player::PlayingTime;
use afm_engine::positions::Position;
use afm_engine::world::norms::{Marks, Rated};
use afm_engine::world::{Person, Stage, World};
use serde_json::{Value, json};

use super::{first_last, growth_at, growth_table, level_name, marks, mid, my_team, ordinal, own_read};
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
    id: u32,
    pos: Position,
    vals: [Option<f64>; 3],
    read: Read,
    commit: bool,
    gone: Option<(usize, String)>,
    draft_risk: bool,
    /// In the transfer portal: the map counts him as gone.
    portal: Option<InPortal>,
}

/// A man in the transfer portal: the reads he would have carried had he
/// stayed, why he would have gone anyway, the reason he gave, and whether
/// he plays his team's playoff run out before he goes.
struct InPortal {
    stay: [Option<f64>; 3],
    stay_gone: Option<(usize, String)>,
    why: &'static str,
    on_hold: bool,
}

/// A college player's portal entry and whether it is on hold: a playoff
/// team's entrant has no day yet and plays the run out (`release_held`).
/// Everyone else in the portal sits out the games left and is gone.
/// `season_on`: this fall's season has started and the year hasn't turned.
pub(super) fn portal_entry(w: &World, p: &Person, season_on: bool) -> Option<(&'static str, bool)> {
    let e = p.cal.portal.filter(|_| p.stage == Stage::College)?;
    Some((e.why, on_hold(e.held, e.entered.0, w.today.0, season_on)))
}

/// On hold for a playoff run: held, with no day yet (the engine's day for
/// a hold is far off, as the portal screen reads it). A hold never
/// outlives its season: once the year has turned he is in the portal like
/// anyone else, whatever his entry still says.
fn on_hold(held: bool, entered: i32, today: i32, season_on: bool) -> bool {
    season_on && held && entered > today + 3650
}

/// Counts a man in the portal as gone: from now, or from next fall when he
/// is on hold for the playoff run. Keeps what he would have been had he
/// stayed. Once he is in the portal he is no draft risk to plan around.
fn leave_through_portal(
    vals: &mut [Option<f64>; 3],
    gone: &mut Option<(usize, String)>,
    draft_risk: &mut bool,
    why: &'static str,
    on_hold: bool,
) -> InPortal {
    let kept = InPortal { stay: *vals, stay_gone: gone.take(), why, on_hold };
    let from = usize::from(on_hold);
    for v in vals.iter_mut().skip(from) {
        *v = None;
    }
    *draft_risk = false;
    *gone = Some((
        from,
        if on_hold { "means to enter the transfer portal after the playoff run" } else { "is in the transfer portal" }
            .to_string(),
    ));
    kept
}

/// Each spot's starter in each of the three falls: now by the depth order
/// (yours, else your staff's), later by the carried reads. `val` is a
/// man's read in a fall (None: he isn't there).
fn pick_starters(
    men: &[Man],
    order: &[(Position, Vec<u32>)],
    val: &dyn Fn(usize, usize) -> Option<f64>,
) -> Vec<[Option<usize>; 3]> {
    let mut starters: Vec<[Option<usize>; 3]> = vec![[None; 3]; SPOTS.len()];
    #[allow(clippy::needless_range_loop)] // n is the year, read from every man too
    for n in 0..3 {
        let mut used: Vec<usize> = Vec::new();
        for (k, &(_, pos, _, _, _)) in SPOTS.iter().enumerate() {
            let mut cands: Vec<(f64, usize)> = men
                .iter()
                .enumerate()
                .filter(|(_, m)| m.pos == pos && !used.contains(&m.i))
                .filter_map(|(j, _)| val(j, n).map(|v| (v, j)))
                .collect();
            cands.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut pick = cands.first().map(|x| x.1);
            if n == 0
                && let Some((_, ids)) = order.iter().find(|x| x.0 == pos)
            {
                let by_order = ids.iter().find_map(|id| cands.iter().find(|c| men[c.1].id == *id).map(|c| c.1));
                pick = by_order.or(pick);
            }
            if let Some(j) = pick {
                used.push(men[j].i);
                starters[k][n] = Some(j);
            }
        }
    }
    starters
}

/// A man's place in a fall once the starters are picked.
struct Place {
    /// The spot he starts at.
    spot: Option<usize>,
    /// His place in line at his position: the starters first, in the map's
    /// order, then the rest by their reads.
    line: usize,
    /// How many play his position that fall.
    of: usize,
    role: PlayingTime,
}

/// Where man `j` stands in fall `n` (None: he isn't there). His role is the
/// game's own depth chart rule (`season::depth_chart`: the starters, then
/// the next man up rotates in, the rest back up), with every spot on the
/// map a starter.
fn place(
    men: &[Man],
    starters: &[[Option<usize>; 3]],
    j: usize,
    n: usize,
    val: &dyn Fn(usize, usize) -> Option<f64>,
) -> Option<Place> {
    let v = val(j, n)?;
    let pos = men[j].pos;
    let first: Vec<usize> =
        SPOTS.iter().zip(starters).filter(|(s, _)| s.1 == pos).filter_map(|(_, st)| st[n]).collect();
    let spot = (0..SPOTS.len()).find(|&k| starters[k][n] == Some(j));
    let at_pos = |m: usize| men[m].pos == pos && val(m, n).is_some();
    let of = (0..men.len()).filter(|&m| at_pos(m)).count();
    let line = match first.iter().position(|&m| m == j) {
        Some(k) => k + 1,
        None => {
            let better = |m: usize| val(m, n).is_some_and(|x| x > v || (x == v && m < j));
            first.len()
                + 1
                + (0..men.len()).filter(|&m| m != j && at_pos(m) && !first.contains(&m) && better(m)).count()
        }
    };
    let slots = depth_slots(pos);
    // A position the map has no spot for (the long snapper) starts as
    // many as the game's depth chart does.
    let spots = SPOTS.iter().filter(|s| s.1 == pos).count();
    let start = if spots == 0 { slots.starters } else { spots };
    let role = if spot.is_some() || (spots == 0 && line <= start) {
        PlayingTime::Starter
    } else if line <= start.max(slots.starters + slots.rotation.max(1)) {
        PlayingTime::Rotation
    } else {
        PlayingTime::Backup
    };
    Some(Place { spot, line, of, role })
}

/// A place in plain words: "Starter at left tackle", "Second running back".
fn place_words(pos: Position, pl: &Place) -> String {
    let one = crate::lane_ratings::pos_words(pos).0;
    match (pl.role, pl.spot) {
        (PlayingTime::Starter, Some(k)) => {
            let full = SPOTS[k].2;
            let mut c = full.chars();
            let low = c.next().map_or(String::new(), |f| f.to_lowercase().collect::<String>() + c.as_str());
            format!("Starter at {low}")
        }
        (PlayingTime::Starter, None) => format!("Starter at {one}"),
        _ => {
            const WORDS: [&str; 10] =
                ["First", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth", "Ninth", "Tenth"];
            match WORDS.get(pl.line.wrapping_sub(1)) {
                Some(word) => format!("{word} {one}"),
                None => format!("{} {one}", ordinal(pl.line as u32)),
            }
        }
    }
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
pub(super) fn class_in(w: &World, i: usize, fall: i32) -> String {
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

pub(super) fn is_commit(w: &World, i: usize) -> bool {
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
    // This fall's season has started (camp has opened) and the year hasn't
    // turned: the next fall to start is the one after.
    let season_on = w.today >= in_season_from(w, t);
    let level_tier = match t {
        TeamRef::HighSchool(_) => 0,
        TeamRef::Nfl(_) => 2,
        _ => 1,
    };
    // Seasons each player has left at this team's division (minor bug B9).
    let division = match t {
        TeamRef::College(j) => Some(w.programs[j].division),
        _ => None,
    };
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
                let div = division.unwrap_or_else(|| afm_engine::world::eligibility::division_of(p, &w.programs));
                let left = afm_engine::world::eligibility::seasons_left(p, div, f0) as i32;
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
        // A man in the transfer portal is gone, whatever his eligibility
        // says: the map plans without him and keeps what he would have been.
        let portal = portal_entry(w, p, season_on)
            .filter(|_| !commit)
            .map(|(why, on_hold)| leave_through_portal(&mut vals, &mut gone, &mut draft_risk, why, on_hold));
        men.push(Man { i, id: p.player.id, pos, vals, read, commit, gone, draft_risk, portal });
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
    let starters = pick_starters(&men, &order, &|j, n| men[j].vals[n]);
    let marks_at: HashMap<Position, Option<Marks>> =
        SPOTS.iter().map(|s| (s.1, marks(w, t, s.1, Rated::Overall))).collect();
    let name_of = |j: usize| first_last(&w.people[men[j].i].name).1;
    let full_of = |j: usize| w.people[men[j].i].name.clone();
    // The next fall to start: this one until camp opens, then the next.
    let coming = usize::from(season_on);
    // Your players in the portal: the place each would have had if he
    // alone had stayed, from the same reads and the same picking. A spot
    // he would have started at says so (spot, fall, man).
    let mut in_portal: Vec<Value> = Vec::new();
    let mut would_start: Vec<(usize, usize, usize)> = Vec::new();
    for (j, man) in men.iter().enumerate() {
        let Some(ip) = &man.portal else { continue };
        let val = |m: usize, n: usize| if m == j { ip.stay[n] } else { men[m].vals[n] };
        let with_him = pick_starters(&men, &order, &val);
        // The map's marks, or his position's own where the map has no spot
        // (the long snapper).
        let off_map;
        let m = match marks_at.get(&man.pos) {
            Some(x) => x.as_ref(),
            None => {
                off_map = marks(w, t, man.pos, Rated::Overall);
                off_map.as_ref()
            }
        };
        let mut first_start = None;
        let stayed: Vec<Value> = (0..3)
            .map(|n| {
                let Some(pl) = place(&men, &with_him, j, n, &val) else { return Value::Null };
                // The first fall still to start that he would have started in.
                if first_start.is_none() && n >= coming {
                    first_start = pl.spot.map(|k| (k, n, j));
                }
                json!({
                    "fall": falls[n], "role": views::role_words(pl.role),
                    "spot": pl.spot.map(|k| SPOTS[k].0), "spot_full": pl.spot.map(|k| SPOTS[k].2),
                    "line": pl.line, "of": pl.of, "val": ip.stay[n].map(f64::round),
                    "verdict": verdict(ip.stay[n], m), "class": class_in(w, man.i, falls[n]),
                    "words": place_words(man.pos, &pl),
                })
            })
            .collect();
        // A man on hold still starts now, and his spot says when he goes.
        would_start.extend(first_start.filter(|_| !ip.on_hold));
        in_portal.push(json!({
            "id": man.id, "name": full_of(j), "pos": man.pos.name(), "class": class_in(w, man.i, falls[coming]),
            "why": ip.why, "on_hold": ip.on_hold,
            "stay_gone": ip.stay_gone.as_ref().map(|g| json!({ "at": g.0, "why": g.1 })),
            "stayed": stayed,
        }));
    }
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
        for &(_, n, j) in would_start.iter().filter(|x| x.0 == k) {
            why.push(format!(
                "{} is in the transfer portal and counts as gone: he would have started here in {}.",
                full_of(j),
                falls[n]
            ));
        }
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
                    // The real next window (minor bug B12).
                    let when = match afm_engine::world::front_office::college::next_window(w) {
                        Some((a, b, _)) if a <= w.today => format!("the portal is open until {}", views::date_text(b)),
                        Some((a, _, _)) => format!("the portal opens {}", views::date_text(a)),
                        None => "he can leave through the portal".to_string(),
                    };
                    format!("Talk with {}: he's unhappy, and {when}.", name_of(j))
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
        .filter(|&j| {
            !men[j].commit
                && men[j].gone.as_ref().is_some_and(|g| g.0 == 1)
                && !men[j].draft_risk
                && men[j].portal.is_none()
        })
        .collect();
    let entered: Vec<usize> = (0..men.len()).filter(|&j| men[j].portal.is_some()).collect();
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
        leaving.push(json!({ "label": "In the portal", "n": entered.len(), "names": names_list(&entered) }));
        leaving.push(json!({ "label": "Portal risks", "n": portal.len(), "names": names_list(&portal) }));
        let label = format!("Commits for {}", f0 + 1);
        leaving.push(json!({ "label": label, "n": arriving.len(), "names": names_list(&arriving) }));
    }
    let roster_n = views::roster_people(w, t).len();
    let roster_line = match t {
        TeamRef::College(c) => {
            let (lim, _) = afm_engine::world::ncaa::roster_limits(&w.programs[c], f0);
            let commits = men.iter().filter(|m| m.commit).count();
            // The men in the portal are off the count; one on hold for the
            // playoff run still plays.
            let away = views::roster_people(w, t)
                .into_iter()
                .filter(|&i| portal_entry(w, &w.people[i], season_on).is_some_and(|e| !e.1))
                .count();
            let left = if away > 0 { format!(" · {away} in the portal") } else { String::new() };
            let s = if commits == 1 { "" } else { "s" };
            format!("{} on the roster of {lim}{left} · {commits} commit{s}", roster_n - away)
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
    // Every man the map counts, for the screens that show a group's depth:
    // his read and class each fall he is there, the spot he starts at, and
    // why he goes.
    let men_rows: Vec<Value> = men
        .iter()
        .enumerate()
        .map(|(j, m)| {
            json!({
                "id": m.id, "name": full_of(j), "pos": m.pos.name(), "commit": m.commit, "portal": m.portal.is_some(),
                "classes": (0..3).map(|n| m.vals[n].map(|_| class_in(w, m.i, falls[n]))).collect::<Vec<_>>(),
                "vals": m.vals.iter().map(|v| v.map(f64::round)).collect::<Vec<_>>(),
                "spots": (0..3)
                    .map(|n| (0..SPOTS.len()).find(|&k| starters[k][n] == Some(j)).map(|k| SPOTS[k].0))
                    .collect::<Vec<_>>(),
                "gone": m.gone.as_ref().map(|g| json!({ "at": g.0, "why": g.1 })),
                "draft_risk": m.draft_risk,
            })
        })
        .collect();
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
        "coming": coming,
        "portal": in_portal,
        "men": men_rows,
        "draft_note": "A draft risk is a player among the level's best a year from being eligible: the staff counts him gone.",
    }))
}

/// The positions the map calls weak when the next season's work starts (a
/// Hole or Thin spot in the fall still to come): position name to the
/// weakest spot there (in words) and its verdict. The inbox's portal-day
/// message asks it which entrants "fit what we need".
pub(crate) fn weak_positions(w: &World) -> HashMap<String, (String, &'static str)> {
    let mut out: HashMap<String, (String, &'static str)> = HashMap::new();
    let Ok(v) = view(w, &json!({})) else { return out };
    let coming = v["coming"].as_u64().unwrap_or(0) as usize;
    for s in v["spots"].as_array().into_iter().flatten() {
        let verdict = match s["verdicts"][coming].as_str() {
            Some("Hole") => "Hole",
            Some("Thin") => "Thin",
            _ => continue,
        };
        let pos = s["pos"].as_str().unwrap_or_default().to_string();
        let full = s["full"].as_str().unwrap_or_default().to_lowercase();
        let worse = out.get(&pos).is_none_or(|old| verdict == "Hole" && old.1 == "Thin");
        if worse {
            out.insert(pos, (full, verdict));
        }
    }
    out
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

    fn man(i: usize, pos: Position, vals: [Option<f64>; 3]) -> Man {
        Man {
            i,
            id: 100 + i as u32,
            pos,
            vals,
            read: Read::Range(60, 70),
            commit: false,
            gone: None,
            draft_risk: false,
            portal: None,
        }
    }

    fn all(v: f64) -> [Option<f64>; 3] {
        [Some(v), Some(v + 1.0), Some(v + 2.0)]
    }

    /// Puts man `j` in the portal, as the view does.
    fn enters(men: &mut [Man], j: usize, on_hold: bool) {
        let m = &mut men[j];
        m.portal = Some(leave_through_portal(&mut m.vals, &mut m.gone, &mut m.draft_risk, "playing time", on_hold));
    }

    fn spot(key: &str) -> usize {
        SPOTS.iter().position(|s| s.0 == key).unwrap()
    }

    /// The place man `j` would have had if he alone had stayed.
    fn stayed(men: &[Man], order: &[(Position, Vec<u32>)], j: usize, n: usize) -> Option<Place> {
        let ip = men[j].portal.as_ref().unwrap();
        let val = |m: usize, n: usize| if m == j { ip.stay[n] } else { men[m].vals[n] };
        let with_him = pick_starters(men, order, &val);
        place(men, &with_him, j, n, &val)
    }

    #[test]
    fn a_man_in_the_portal_counts_as_gone() {
        use Position::*;
        // Two quarterbacks; the better one, a likely early entrant, enters the portal.
        let mut men = vec![man(0, QB, all(80.0)), man(1, QB, all(70.0)), man(2, RB, all(75.0))];
        men[0].draft_risk = true;
        men[0].vals[2] = None;
        men[0].gone = Some((2, "is likely to declare for the draft after 2027".into()));
        let before = pick_starters(&men, &[], &|j, n| men[j].vals[n]);
        assert_eq!(before[spot("QB")], [Some(0), Some(0), Some(1)]);
        enters(&mut men, 0, false);
        assert_eq!(men[0].vals, [None; 3], "off every fall the map plans");
        assert_eq!(men[0].gone, Some((0, "is in the transfer portal".to_string())));
        assert!(!men[0].draft_risk, "in the portal, not a draft risk to plan around");
        let after = pick_starters(&men, &[], &|j, n| men[j].vals[n]);
        assert_eq!(after[spot("QB")], [Some(1); 3], "the next man starts every fall");
        assert_eq!(after[spot("RB")], [Some(2); 3], "the other spots stand");
        // If he had stayed: the starter this fall and next, then off to the draft as before.
        for n in 0..2 {
            let pl = stayed(&men, &[], 0, n).unwrap();
            assert_eq!((pl.spot, pl.line, pl.of, pl.role), (Some(spot("QB")), 1, 2, PlayingTime::Starter));
            assert_eq!(place_words(QB, &pl), "Starter at quarterback");
        }
        assert!(stayed(&men, &[], 0, 2).is_none(), "gone by then anyway");
        let kept = men[0].portal.as_ref().unwrap();
        assert_eq!(kept.stay_gone.as_ref().map(|g| g.0), Some(2));
        assert_eq!(kept.why, "playing time");
    }

    #[test]
    fn an_entrant_on_hold_plays_the_run_out() {
        use Position::*;
        let mut men = vec![man(0, QB, all(80.0)), man(1, QB, all(70.0))];
        enters(&mut men, 0, true);
        assert_eq!(men[0].vals, [Some(80.0), None, None]);
        assert_eq!(men[0].gone.as_ref().map(|g| g.0), Some(1));
        let st = pick_starters(&men, &[], &|j, n| men[j].vals[n]);
        assert_eq!(st[spot("QB")], [Some(0), Some(1), Some(1)]);
    }

    #[test]
    fn a_hold_never_outlives_its_season() {
        let (today, far) = (10_000, i32::MAX / 2);
        assert!(on_hold(true, far, today, true), "a playoff team's entrant, the run still on");
        assert!(!on_hold(true, today + 3, today, true), "released: he has his day");
        assert!(!on_hold(false, today + 3, today, true), "an entry dated inside the window");
        assert!(!on_hold(true, far, today, false), "the year has turned: he is in the portal");
    }

    #[test]
    fn the_depth_order_skips_a_man_in_the_portal() {
        use Position::*;
        // Your order starts the lesser back; he enters the portal.
        let mut men = vec![man(0, RB, all(60.0)), man(1, RB, all(72.0)), man(2, RB, all(66.0))];
        let order = vec![(RB, vec![100, 102, 101])];
        let st = pick_starters(&men, &order, &|j, n| men[j].vals[n]);
        assert_eq!(st[spot("RB")], [Some(0), Some(1), Some(1)], "now by your order, later by the reads");
        enters(&mut men, 0, false);
        let st = pick_starters(&men, &order, &|j, n| men[j].vals[n]);
        assert_eq!(st[spot("RB")], [Some(2), Some(1), Some(1)], "the next man in your order starts now");
        // Had he stayed he starts now by your order; next fall he is third by the reads.
        let now = stayed(&men, &order, 0, 0).unwrap();
        assert_eq!((now.spot, now.line, now.role), (Some(spot("RB")), 1, PlayingTime::Starter));
        let next = stayed(&men, &order, 0, 1).unwrap();
        assert_eq!((next.spot, next.line, next.of, next.role), (None, 3, 3, PlayingTime::Backup));
        assert_eq!(place_words(RB, &next), "Third running back");
    }

    #[test]
    fn the_place_he_would_have_had_follows_the_depth_chart_rule() {
        use Position::*;
        // Five receivers, four linebackers, two quarterbacks and a long
        // snapper stay; one more of each is in the portal.
        let mut men: Vec<Man> = Vec::new();
        for (k, v) in [80.0, 78.0, 76.0, 72.0, 70.0].into_iter().enumerate() {
            men.push(man(k, WR, all(v)));
        }
        for (k, v) in [80.0, 78.0, 76.0, 70.0].into_iter().enumerate() {
            men.push(man(10 + k, LB, all(v)));
        }
        men.push(man(20, QB, all(80.0)));
        men.push(man(21, QB, all(60.0)));
        men.push(man(30, LS, all(60.0)));
        let role = |pos: Position, v: f64| {
            let mut men: Vec<Man> = men.iter().map(|m| man(m.i, m.pos, m.vals)).collect();
            men.push(man(99, pos, all(v)));
            let j = men.len() - 1;
            enters(&mut men, j, false);
            let pl = stayed(&men, &[], j, 1).unwrap();
            (pl.role, pl.line, pl.spot.map(|k| SPOTS[k].0), place_words(pos, &pl))
        };
        assert_eq!(role(WR, 79.0), (PlayingTime::Starter, 2, Some("Z"), "Starter at wide receiver (Z)".into()));
        assert_eq!(role(WR, 75.0), (PlayingTime::Rotation, 4, None, "Fourth wide receiver".into()));
        assert_eq!(role(WR, 71.5), (PlayingTime::Backup, 5, None, "Fifth wide receiver".into()));
        // Every spot on the map starts: the third linebacker too.
        assert_eq!(role(LB, 77.5), (PlayingTime::Starter, 3, Some("SAM"), "Starter at strongside linebacker".into()));
        assert_eq!(role(LB, 75.0), (PlayingTime::Backup, 4, None, "Fourth linebacker".into()));
        // The game's depth chart calls the next quarterback rotation.
        assert_eq!(role(QB, 70.0), (PlayingTime::Rotation, 2, None, "Second quarterback".into()));
        assert_eq!(role(QB, 50.0), (PlayingTime::Backup, 3, None, "Third quarterback".into()));
        // No spot on the map for the long snapper: the depth chart's one starter.
        assert_eq!(role(LS, 65.0), (PlayingTime::Starter, 1, None, "Starter at long snapper".into()));
        assert_eq!(role(LS, 55.0), (PlayingTime::Rotation, 2, None, "Second long snapper".into()));
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
