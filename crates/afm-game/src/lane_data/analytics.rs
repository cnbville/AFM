//! Player analytics (items 7 and 8, board 46): the percentile table by
//! position group and a player's card with the past seasons closest to his.
//!
//! A season's numbers are the group's card (`world::analytics`): the
//! charted card where your seat charts the level, the box score card where
//! it doesn't. Percentiles are among that season's qualifiers at the level
//! (a college division, the NFL, a state's high schools), so the seasons
//! like his compare men against their own seasons' fields, whatever the
//! era.

use std::collections::{BTreeSet, HashSet};

use afm_engine::positions::Position;
use afm_engine::world::analytics::{self as an, Cm, Fmt};
use afm_engine::world::data::Division;
use afm_engine::world::marks::{self, NowLine, Tier};
use afm_engine::world::records::{STAT_COUNT, Stat};
use afm_engine::world::{Stage, World, tally};
use serde_json::{Value, json};

use super::deep::{self, One, Src};
use super::{charted, division_arg, state_arg, team_json, tier_arg, tier_key};
use crate::views;

/// One player's season in a pool.
struct Row {
    id: u32,
    team: usize,
    pos: Position,
    games: u32,
    year: i32,
    vol: f64,
    vals: Vec<Option<f64>>,
}

/// Where a pool is drawn from.
#[derive(Clone, Copy)]
struct Place {
    tier: Tier,
    division: Option<Division>,
    state: Option<usize>,
}

fn place_of(w: &World, args: &Value) -> Place {
    let tier = tier_arg(w, args);
    Place {
        tier,
        division: (tier == Tier::College).then(|| division_arg(w, args)),
        state: (tier == Tier::Hs).then(|| state_arg(w, args)),
    }
}

fn group_arg(args: &Value) -> usize {
    match args.get("group") {
        Some(Value::String(g)) => an::GROUPS.iter().position(|x| x.eq_ignore_ascii_case(g)).unwrap_or(0),
        Some(Value::Number(n)) => (n.as_u64().unwrap_or(0) as usize).min(an::GROUPS.len() - 1),
        _ => 0,
    }
}

/// The view of a group's charted card the page asks for (items 4-11 and
/// 28; 0 the overview).
fn view_arg(args: &Value, group: usize) -> usize {
    args.get("view").and_then(Value::as_u64).map(|v| v as usize).filter(|&v| v < an::views(group).len()).unwrap_or(0)
}

/// The group's card at a place: the charted one (the view asked for) as
/// deep as your seat's charting goes there (Data overhaul item 2), or the
/// box score one.
fn card_of(w: &World, p: Place, group: usize, view: usize) -> (Vec<Cm>, Stat, &'static str, bool) {
    let ch = charted(w, p.tier, p.division);
    let views = an::views(group);
    let card = match super::charting(w, p.tier, p.division) {
        Some(c) => views[view.min(views.len() - 1)].1.iter().copied().filter(|&cm| an::depth(cm) <= c.depth).collect(),
        None => an::boxed(group).to_vec(),
    };
    let (vol, word) = if ch { an::volume(group) } else { an::box_volume(group) };
    (card, vol, word, ch)
}

/// The season in progress as your seat's charting has it: a charted
/// level's numbers stop at the last game charted, so the games of the
/// last `days` days come off each man's line (the box score card is whole
/// at once). The NFL's tracking is a day behind; your own staff's charting
/// follows your analytics head (item 2).
fn charted_now(w: &World, p: Place, now: Vec<NowLine>) -> (Vec<NowLine>, Option<afm_engine::calendar::Date>) {
    let Some(c) = super::charting(w, p.tier, p.division) else { return (now, None) };
    let level = if p.tier == Tier::Nfl { tally::Level::Nfl } else { tally::Level::College };
    let since = afm_engine::calendar::Date(w.today.0 - (c.days as i32 - 1));
    let recent = w.tally_games_since(level, since);
    if recent.is_empty() {
        return (now, None);
    }
    let mut off: std::collections::HashMap<u32, ([f32; STAT_COUNT], u32)> = Default::default();
    for g in &recent {
        for l in &g.lines {
            let e = off.entry(l.id).or_insert(([0.0; STAT_COUNT], 0));
            for (a, b) in e.0.iter_mut().zip(l.all()) {
                *a += b;
            }
            let all = l.all();
            e.1 += (all[Stat::SnapsOff as usize] + all[Stat::SnapsDef as usize] + all[Stat::SnapsSt as usize] > 0.0)
                as u32;
        }
    }
    let now = now
        .into_iter()
        .map(|mut l| {
            if let Some((o, g)) = off.get(&l.id) {
                for (a, b) in l.all.iter_mut().zip(o) {
                    *a -= b;
                }
                l.games = l.games.saturating_sub(*g);
            }
            l
        })
        .filter(|l| l.games > 0)
        .collect();
    (now, Some(since))
}

/// College's season in progress up to your staff's charting, every man's
/// line (lane NB11's prospect pages). The charting's delay comes off as
/// it does for your own division.
pub(super) fn college_now(w: &World) -> Vec<NowLine> {
    let division = match views::mine(w) {
        Some(views::TeamRef::College(p)) => w.programs[p].division,
        _ => Division::FBS,
    };
    let place = Place { tier: Tier::College, division: Some(division), state: None };
    charted_now(w, place, marks::season_now(w, Tier::College, None)).0
}

#[allow(clippy::too_many_arguments)]
fn row_from(
    id: u32,
    team: usize,
    pos: Position,
    games: u32,
    year: i32,
    all: &dyn Fn(Stat) -> f64,
    card: &[Cm],
    vol: Stat,
    one: Option<&One>,
) -> Row {
    let vals = card
        .iter()
        .map(|cm| if cm.deep() { one.and_then(|o| cm.value_d(&|c| o.get(c))) } else { cm.value(all, games as f64) })
        .collect();
    Row { id, team, pos, games, year, vol: all(vol), vals }
}

/// His NFL season's whole line, from the people in the world or the ones
/// who left it (`gone`: the second, by id, built when first needed).
fn pro_all(
    w: &World,
    id: u32,
    year: i32,
    gone: &mut Option<std::collections::HashMap<u32, usize>>,
) -> Option<[f32; STAT_COUNT]> {
    let career = match views::person_index(w, id) {
        Some(k) => w.people[k].pro.as_deref(),
        None => {
            let map = gone.get_or_insert_with(|| {
                w.alumni.iter().enumerate().filter_map(|(k, a)| a.player.as_ref().map(|p| (p.id, k))).collect()
            });
            map.get(&id).and_then(|&k| w.alumni[k].pro.as_deref())
        }
    }?;
    let s = career.seasons.iter().find(|s| s.year == year && s.games > 0)?;
    Some(afm_engine::world::records::from_line(&s.stats))
}

/// The seasons a level keeps, newest first, and the one in progress
/// (`now`, from `marks::season_now`).
fn years(w: &World, p: Place, now: &[NowLine]) -> Vec<i32> {
    let mut v: Vec<i32> = match p.tier {
        Tier::College => w.stats.year_index.iter().map(|x| x.0).collect(),
        Tier::Nfl => {
            // The kept rows, and before they began (an older world file)
            // the players' own seasons: `pool` reads either.
            let mut y: BTreeSet<i32> = w.marks.pro_rows.iter().map(|r| r.year as i32).collect();
            let first = y.first().copied().unwrap_or(i32::MAX);
            y.extend(
                w.people
                    .iter()
                    .filter_map(|x| x.pro.as_ref())
                    .flat_map(|c| c.seasons.iter().filter(|s| s.year < first && s.games > 0).map(|s| s.year)),
            );
            y.into_iter().collect()
        }
        Tier::Hs => {
            let mut y: Vec<i32> = w.people.iter().flat_map(|x| x.hs_lines.iter().map(|l| l.year as i32)).collect();
            y.sort_unstable();
            y.dedup();
            y
        }
    };
    if !now.is_empty() {
        v.push(w.year);
    }
    v.sort_unstable();
    v.dedup();
    v.reverse();
    v
}

/// A season's pool at a place for a group (`now`: the season in progress,
/// read only for this year).
fn pool(w: &World, p: Place, year: i32, group: usize, view: usize, now: &[NowLine], src: &Src) -> Vec<Row> {
    let mut out = pool_all(w, p, year, group, view, now, src);
    // Returners: any man with a return.
    if group == an::RET {
        out.retain(|r| r.vol > 0.0);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn pool_all(w: &World, p: Place, year: i32, group: usize, view: usize, now: &[NowLine], src: &Src) -> Vec<Row> {
    let (card, vol, _, ch) = card_of(w, p, group, view);
    let in_group = |pos: Position| group == an::RET || an::group_of(pos) == Some(group);
    let mut out = Vec::new();
    if year == w.year && !now.is_empty() {
        for l in now.iter().filter(|l| in_group(l.pos)) {
            if p.tier == Tier::College && w.programs.get(l.team).map(|x| x.division) != p.division {
                continue;
            }
            let get = |s: Stat| l.all[s as usize] as f64;
            let one = src.one(l.id);
            out.push(row_from(l.id, l.team, l.pos, l.games, year, &get, &card, vol, one.as_ref()));
        }
        return out;
    }
    match p.tier {
        Tier::College => {
            for r in w.stats.year(year).iter().filter(|r| Some(r.division) == p.division && in_group(r.pos)) {
                let get = |s: Stat| r.get(s);
                let one = src.one(r.id);
                out.push(row_from(
                    r.id,
                    r.program as usize,
                    r.pos,
                    r.games as u32,
                    year,
                    &get,
                    &card,
                    vol,
                    one.as_ref(),
                ));
            }
        }
        Tier::Nfl => {
            let kept: Vec<&marks::ProRow> =
                w.marks.pro_rows.iter().filter(|r| r.year as i32 == year && in_group(r.pos)).collect();
            if !kept.is_empty() && ch && group != an::RET {
                let mut gone = None;
                for r in kept {
                    // Kept in the full charted card's order; a view's other
                    // numbers from his season's line and the depth counts.
                    let full = an::charted(group);
                    let one = src.one(r.id);
                    let mut line: Option<Option<[f32; STAT_COUNT]>> = None;
                    let vals = card
                        .iter()
                        .map(|cm| {
                            if cm.deep() {
                                return one.as_ref().and_then(|o| cm.value_d(&|c| o.get(c)));
                            }
                            if let Some(k) = full.iter().position(|x| x == cm) {
                                return r.vals.get(k).copied().filter(|v| v.is_finite()).map(|v| v as f64);
                            }
                            let all = line.get_or_insert_with(|| pro_all(w, r.id, year, &mut gone)).as_ref()?;
                            cm.value(&|k: Stat| all[k as usize] as f64, r.games as f64)
                        })
                        .collect();
                    out.push(Row {
                        id: r.id,
                        team: r.team as usize,
                        pos: r.pos,
                        games: r.games as u32,
                        year,
                        vol: r.vol as f64,
                        vals,
                    });
                }
            } else {
                // An older world file: the players' own season lines.
                for x in &w.people {
                    let Some(s) = x.pro.as_ref().and_then(|c| c.seasons.iter().find(|s| s.year == year)) else {
                        continue;
                    };
                    let (Some(team), true) = (s.team, s.games > 0 && in_group(x.player.position)) else { continue };
                    let all = afm_engine::world::records::from_line(&s.stats);
                    let get = |k: Stat| all[k as usize] as f64;
                    let one = src.one(x.player.id);
                    out.push(row_from(
                        x.player.id,
                        team,
                        x.player.position,
                        s.games,
                        year,
                        &get,
                        &card,
                        vol,
                        one.as_ref(),
                    ));
                }
            }
        }
        Tier::Hs => out = hs_pools(w, p, group, view, Some(year)).remove(&year).unwrap_or_default(),
    }
    out
}

/// A state's high school seasons for a group, by year, in one pass over
/// the players (`only`: one season).
fn hs_pools(
    w: &World,
    p: Place,
    group: usize,
    view: usize,
    only: Option<i32>,
) -> std::collections::BTreeMap<i32, Vec<Row>> {
    let (card, vol, _, _) = card_of(w, p, group, view);
    let mut out: std::collections::BTreeMap<i32, Vec<Row>> = Default::default();
    for x in &w.people {
        for l in &x.hs_lines {
            let year = l.year as i32;
            if only.is_some_and(|o| o != year) || (group != an::RET && an::group_of(l.pos) != Some(group)) {
                continue;
            }
            if w.high_schools.get(l.school as usize).map(|h| h.state) != p.state {
                continue;
            }
            let all: [f32; STAT_COUNT] = marks::hs_all(&l.s.map(|v| v as i32));
            let get = |k: Stat| all[k as usize] as f64;
            out.entry(year).or_default().push(row_from(
                x.player.id,
                l.school as usize,
                l.pos,
                l.games as u32,
                year,
                &get,
                &card,
                vol,
                None,
            ));
        }
    }
    if group == an::RET {
        for rows in out.values_mut() {
            rows.retain(|r| r.vol > 0.0);
        }
    }
    out
}

/// Every past season's pool at a place (the season in progress left out).
fn past_pools(w: &World, p: Place, group: usize, ys: &[i32]) -> Vec<(i32, Vec<Row>)> {
    if p.tier == Tier::Hs {
        return hs_pools(w, p, group, 0, None).into_iter().filter(|(y, _)| *y != w.year).collect();
    }
    ys.iter().filter(|&&y| y != w.year).map(|&y| (y, pool(w, p, y, group, 0, &[], &Src::None))).collect()
}

/// Each metric's percentile (0-100, 100 the best) among the qualifiers.
fn percentiles(rows: &[Row], qual: &[bool], card: &[Cm]) -> Vec<Vec<Option<f64>>> {
    let mut out = vec![vec![None; card.len()]; rows.len()];
    for (k, cm) in card.iter().enumerate() {
        // A number that only says how he plays shades high as more.
        let better = cm.about().3 || cm.neutral();
        let mut peers: Vec<f64> = rows.iter().zip(qual).filter(|(_, q)| **q).filter_map(|(r, _)| r.vals[k]).collect();
        peers.sort_by(f64::total_cmp);
        let n = peers.len();
        if n < 2 {
            continue;
        }
        for (i, r) in rows.iter().enumerate() {
            let Some(v) = r.vals[k] else { continue };
            let below = peers.partition_point(|&x| x < v) as f64;
            let same = (peers.partition_point(|&x| x <= v) as f64 - below).max(0.0);
            let mut pct = 100.0 * (below + 0.5 * same) / n as f64;
            if !better {
                pct = 100.0 - pct;
            }
            out[i][k] = Some(pct.clamp(0.0, 100.0));
        }
    }
    out
}

fn fmt_cell(cm: Cm, v: f64) -> String {
    match cm.about().2 {
        Fmt::Whole => format!("{v:.0}"),
        Fmt::One => format!("{v:.1}"),
        Fmt::Two => format!("{v:.2}"),
        Fmt::Pct => format!("{v:.1}%"),
        Fmt::Signed => format!("{v:+.2}"),
    }
}

fn name(w: &World, id: u32) -> String {
    marks::name_of(w, id).unwrap_or_else(|| "a former player".into())
}

/// Which way a number is better, for the page: "high", "low", or "none"
/// for one that only says how he plays.
fn better_word(cm: Cm) -> &'static str {
    if cm.neutral() {
        "none"
    } else if cm.about().3 {
        "high"
    } else {
        "low"
    }
}

/// Each column's label, its glossary entry (the hover on its name), the
/// middle value and the level average among the season's qualifiers
/// (item 3), which the hover shows beside the plain words, and whether
/// it has an era index and an opponent-adjusted reading.
fn columns(card: &[Cm], rows: &[Row], qual: &[bool]) -> Vec<Value> {
    let mids = middles(rows, qual, card);
    let avgs = means(rows, qual, card);
    card.iter()
        .zip(mids)
        .zip(avgs)
        .map(|((cm, mid), avg)| {
            let (short, long, f, _) = cm.about();
            json!({
                "label": short, "long": long, "better": better_word(*cm),
                "key": super::glossary::cm_key(*cm), "mid": mid.map(|v| fmt_cell(*cm, v)),
                "avg": avg.map(|v| fmt_cell(*cm, v)),
                "era": f != Fmt::Whole, "adj": an::opp_adjust(*cm).is_some(),
            })
        })
        .collect()
}

/// Each metric's average (the level average, item 3) among the qualifiers.
fn means(rows: &[Row], qual: &[bool], card: &[Cm]) -> Vec<Option<f64>> {
    (0..card.len())
        .map(|k| {
            let v: Vec<f64> = rows.iter().zip(qual).filter(|(_, q)| **q).filter_map(|(r, _)| r.vals[k]).collect();
            (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
        })
        .collect()
}

/// The era index (item 12): 100 the qualifiers' average that season and
/// 10 a standard deviation, higher better (a number that only says how he
/// plays reads higher as more), as Pro Football Reference's "+" stats
/// read. Counts stay as they are. Proposed, not confirmed: the scale.
fn era_vals(rows: &[Row], qual: &[bool], card: &[Cm]) -> Vec<Vec<Option<f64>>> {
    let mut out: Vec<Vec<Option<f64>>> = rows.iter().map(|r| r.vals.clone()).collect();
    for (k, cm) in card.iter().enumerate() {
        let (_, _, f, better) = cm.about();
        if f == Fmt::Whole {
            continue;
        }
        let v: Vec<f64> = rows.iter().zip(qual).filter(|(_, q)| **q).filter_map(|(r, _)| r.vals[k]).collect();
        let n = v.len() as f64;
        let mean = v.iter().sum::<f64>() / n.max(1.0);
        let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt();
        let sign = if better || cm.neutral() { 1.0 } else { -1.0 };
        for (i, r) in rows.iter().enumerate() {
            out[i][k] = if v.len() < 2 {
                None
            } else {
                r.vals[k].map(|x| if sd > 0.0 { 100.0 + 10.0 * sign * (x - mean) / sd } else { 100.0 })
            };
        }
    }
    out
}

/// The opponent-adjusted numbers (item 13, `an::opp_adjust`): each man's
/// opponents read from the season's games, the level's rate the average
/// his field faced, so the field's average stays put. Seasons since the
/// tallies began, college and the NFL.
#[allow(clippy::too_many_arguments)]
fn adj_vals(
    w: &World,
    p: Place,
    year: i32,
    group: usize,
    rows: &[Row],
    qual: &[bool],
    card: &[Cm],
    src: &Src,
) -> Vec<Vec<Option<f64>>> {
    let mut out: Vec<Vec<Option<f64>>> = rows.iter().map(|r| r.vals.clone()).collect();
    let sched = deep::sched(w, p.tier, year);
    let offense = an::offense_group(group);
    for (k, cm) in card.iter().enumerate() {
        let Some((rate, how)) = an::opp_adjust(*cm) else { continue };
        let faced: Vec<Option<f64>> = rows
            .iter()
            .map(|r| sched.as_ref().and_then(|s| src.one(r.id).and_then(|o| s.faced(&o, offense, rate))))
            .collect();
        let lv: Vec<f64> = faced.iter().zip(qual).filter(|(_, q)| **q).filter_map(|(f, _)| *f).collect();
        let lvl = (!lv.is_empty()).then(|| lv.iter().sum::<f64>() / lv.len() as f64);
        for (i, r) in rows.iter().enumerate() {
            out[i][k] = match (r.vals[k], faced[i], lvl) {
                (Some(v), Some(f), Some(l)) => match how {
                    None => (f > 0.0).then(|| v * l / f),
                    Some(m) => Some(v - m * (f - l)),
                },
                _ => None,
            };
        }
    }
    out
}

/// The page's reading: "raw", "era" or "adj".
fn mode_arg(args: &Value) -> &'static str {
    match args.get("mode").and_then(Value::as_str) {
        Some("era") => "era",
        Some("adj") => "adj",
        _ => "raw",
    }
}

/// A season's depth counts for a table at a place (the division's men at
/// college; `before`: this season's games your charting hasn't reached).
fn table_src<'a>(w: &'a World, p: Place, year: i32, before: Option<afm_engine::calendar::Date>) -> Src<'a> {
    let since = if year == w.tally_open_season() { before } else { None };
    let div = p.division;
    let scope = div.map_or("nfl", |d| d.name());
    deep::src(w, p.tier, year, since, scope, &|s| {
        div.is_none_or(|d| (s.team as usize) < w.programs.len() && super::season_place(w, s.team as usize, year).0 == d)
    })
}

/// The exact spots a group's filter offers (item 14): (code, name), a
/// defensive spot's code 100 on.
fn spots_for(group: usize) -> Vec<(usize, &'static str)> {
    let o = an::SPOTS_O.iter().enumerate().map(|(k, n)| (k, *n));
    let d = an::SPOTS_D.iter().enumerate().map(|(k, n)| (100 + k, *n));
    match group {
        an::RET => o.skip(1).chain(d).collect(),
        g if an::offense_group(g) => o.collect(),
        _ => d.collect(),
    }
}

/// The spot a man played most that season, as `spots_for` codes.
fn spot_code(one: &One, group: usize) -> Option<usize> {
    let d = |c: u16| one.get(c);
    spots_for(group)
        .into_iter()
        .map(|(c, _)| (c, if c >= 100 { an::spot_snaps(&d, false, c - 100) } else { an::spot_snaps(&d, true, c) }))
        .filter(|x| x.1 > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0)
}

/// A team's conference that season: a college program's, or the NFL's
/// AFC or NFC.
fn conference_of(w: &World, tier: Tier, team: usize, year: i32) -> Option<String> {
    match tier {
        Tier::College => (team < w.programs.len())
            .then(|| super::season_place(w, team, year).1.to_string())
            .filter(|c| !c.is_empty()),
        Tier::Nfl => afm_engine::world::franchise::era_in(w, team, year)
            .map(|e| if e.div < 4 { "AFC".to_string() } else { "NFC".to_string() }),
        Tier::Hs => None,
    }
}

/// Each metric's middle value (the median) among the qualifiers.
fn middles(rows: &[Row], qual: &[bool], card: &[Cm]) -> Vec<Option<f64>> {
    (0..card.len())
        .map(|k| {
            let mut v: Vec<f64> = rows.iter().zip(qual).filter(|(_, q)| **q).filter_map(|(r, _)| r.vals[k]).collect();
            if v.is_empty() {
                return None;
            }
            v.sort_by(f64::total_cmp);
            let n = v.len();
            Some(if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn base(
    w: &World,
    p: Place,
    group: usize,
    year: i32,
    years: &[i32],
    min: f64,
    word: &str,
    ch: bool,
) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("level".into(), json!(tier_key(p.tier)));
    m.insert("division".into(), json!(p.division.map(|d| d.name())));
    m.insert("state".into(), json!(p.state.and_then(|s| w.states.get(s)).map(|s| s.code.as_str())));
    m.insert("group".into(), json!(an::GROUPS[group]));
    m.insert("groups".into(), json!(an::GROUPS));
    m.insert("year".into(), json!(year));
    m.insert("years".into(), json!(years));
    m.insert("min".into(), json!(min));
    m.insert("volume".into(), json!(word));
    m.insert("charted".into(), json!(ch));
    m.insert("divisions".into(), json!(Division::ALL.iter().map(|d| d.name()).collect::<Vec<_>>()));
    m.insert("source".into(), super::sources(w)["line"].clone());
    m.insert(
        "charting".into(),
        super::charting(w, p.tier, p.division).as_ref().map_or(Value::Null, super::charting_json),
    );
    m
}

/// The minimum a season needs to qualify: the page's, or two thirds of a
/// starter's volume at the level, cut down while a season is young.
fn min_for(w: &World, p: Place, group: usize, year: i32, ch: bool, args: &Value, now: &[NowLine]) -> f64 {
    if let Some(m) = args.get("min").and_then(Value::as_f64) {
        return m.max(0.0);
    }
    let full = an::default_min(group, ch, p.tier == Tier::Nfl);
    if year != w.year {
        return full;
    }
    // A season in progress: its share played so far, by the most games
    // anyone in the pool has.
    let played = now.iter().map(|l| l.games).max().unwrap_or(0) as f64;
    let total = marks::season_games(w, p.tier, year) as f64;
    if played > 0.0 { (full * (played / total).min(1.0)).round() } else { full }
}

/// What the class filter reads of a player: his high school class, birth
/// date, college years and pro career, from the people still in the world
/// or the ones who left it.
struct Bio<'a> {
    class_of: i32,
    born: afm_engine::calendar::Date,
    staff: &'a [(i32, usize, Option<usize>, Option<usize>)],
    pro: Option<&'a afm_engine::world::pro::ProCareer>,
    stars: u8,
    home: usize,
}

/// The table's filter by class (Cain, 2026-11-02): his class that season
/// (college Fr to Sr, by seasons played; high school by grade; the NFL by
/// year in the league, 5 for a fifth year or more), redshirted or not, and
/// his age on 31 August of the season, as the world counts it.
#[derive(Default)]
struct Filter {
    class: Option<u8>,
    redshirt: Option<bool>,
    age: (Option<u32>, Option<u32>),
    // Item 14: his team or conference that season, the spot he played most,
    // draft eligible next spring, a transfer, his recruiting stars, his
    // home state.
    team: Option<usize>,
    conference: Option<String>,
    spot: Option<usize>,
    eligible: Option<bool>,
    transfer: Option<bool>,
    stars: Option<u8>,
    home: Option<usize>,
}

impl Filter {
    fn of(args: &Value, tier: Tier) -> Filter {
        let class = match args.get("class") {
            Some(Value::String(c)) => match (tier, c.as_str()) {
                (Tier::Nfl, _) => c.trim_end_matches('+').parse::<u8>().ok().filter(|&n| (1..=5).contains(&n)),
                (_, "Fr") => Some(1),
                (_, "So") => Some(2),
                (_, "Jr") => Some(3),
                (_, "Sr") => Some(4),
                _ => None,
            },
            Some(Value::Number(n)) => n.as_u64().map(|n| n.clamp(1, 5) as u8),
            _ => None,
        };
        let redshirt = match args.get("rs").and_then(Value::as_str) {
            Some("yes") if tier == Tier::College => Some(true),
            Some("no") if tier == Tier::College => Some(false),
            _ => None,
        };
        let age = |k: &str| args.get(k).and_then(Value::as_u64).map(|a| a as u32);
        let yes_no = |k: &str| match args.get(k).and_then(Value::as_str) {
            Some("yes") if tier == Tier::College => Some(true),
            Some("no") if tier == Tier::College => Some(false),
            _ => None,
        };
        let num = |k: &str| match args.get(k) {
            Some(Value::Number(n)) => n.as_u64(),
            Some(Value::String(s)) => s.parse().ok(),
            _ => None,
        };
        Filter {
            class,
            redshirt,
            age: (age("age_min"), age("age_max")),
            team: num("team").map(|t| t as usize),
            conference: args.get("conference").and_then(Value::as_str).filter(|c| !c.is_empty()).map(String::from),
            spot: num("spot").map(|t| t as usize),
            eligible: yes_no("eligible"),
            transfer: yes_no("transfer"),
            stars: num("stars").filter(|&s| s <= 5).map(|s| s as u8),
            home: None,
        }
    }

    fn on(&self) -> bool {
        self.class.is_some()
            || self.redshirt.is_some()
            || self.age.0.is_some()
            || self.age.1.is_some()
            || self.team.is_some()
            || self.conference.is_some()
            || self.spot.is_some()
            || self.eligible.is_some()
            || self.transfer.is_some()
            || self.stars.is_some()
            || self.home.is_some()
    }

    /// Whether anything asks about the man himself (his class, age, stars,
    /// home), which needs his file.
    fn bio(&self) -> bool {
        self.class.is_some()
            || self.redshirt.is_some()
            || self.age.0.is_some()
            || self.age.1.is_some()
            || self.eligible.is_some()
            || self.transfer.is_some()
            || self.stars.is_some()
            || self.home.is_some()
    }
}

/// His class in a season, as a roster lists it: (class, redshirted).
/// College: Fr to Sr by seasons of eligibility used before this one, plus
/// this one; a year at school that didn't count (a redshirt: no games, or
/// four or fewer from 2018) makes him a redshirt from then on. Under the
/// five-year model (Division I from 2026) every year counts. High school by
/// grade; the NFL by year in the league.
fn class_in(w: &World, tier: Tier, id: u32, b: &Bio, year: i32) -> (u8, bool) {
    match tier {
        Tier::College => {
            let book = w.stats.career(id);
            let mut years: BTreeSet<i32> = b.staff.iter().map(|s| s.0).filter(|&y| y < year).collect();
            years.extend(book.iter().map(|r| r.year as i32).filter(|&y| y < year));
            let (mut used, mut rs) = (0u32, false);
            for y in years {
                let games: u32 = book.iter().filter(|r| r.year as i32 == y).map(|r| r.games as u32).sum();
                let division = book.iter().find(|r| r.year as i32 == y).map(|r| r.division).or_else(|| {
                    b.staff.iter().find(|s| s.0 == y).and_then(|s| w.programs.get(s.1)).map(|p| p.division)
                });
                let five = division.is_some_and(|d| afm_engine::world::eligibility::five_in_five(y, d));
                if !five && !rs && (games == 0 || y >= 2018 && games <= 4) {
                    rs = true;
                } else {
                    used += 1;
                }
            }
            ((used + 1).min(4) as u8, rs)
        }
        Tier::Hs => {
            let c = match b.class_of - year {
                ..=1 => 4,
                2 => 3,
                3 => 2,
                _ => 1,
            };
            (c, false)
        }
        Tier::Nfl => {
            let before = b
                .pro
                .map_or(0, |c| c.seasons.iter().filter(|s| s.year < year && (s.games > 0 || s.team.is_some())).count());
            ((before + 1).min(5) as u8, false)
        }
    }
}

/// His age on 31 August of a season (from his high school class when the
/// world has no birth date for him).
fn age_in(b: &Bio, year: i32) -> u32 {
    if b.born == afm_engine::calendar::born::UNKNOWN {
        (year - b.class_of + 18).max(0) as u32
    } else {
        afm_engine::calendar::born::age_on(b.born, afm_engine::calendar::Date::ymd(year, 8, 31))
    }
}

/// Which of the rows pass the filter (a player the world no longer knows
/// passes only when nothing is asked of him).
fn passes(w: &World, tier: Tier, rows: &[Row], f: &Filter, spot: &dyn Fn(u32) -> Option<usize>) -> Vec<bool> {
    if !f.on() {
        return vec![true; rows.len()];
    }
    // The ones who left football, by id, built only when a row needs them.
    let mut gone: Option<std::collections::HashMap<u32, usize>> = None;
    rows.iter()
        .map(|r| {
            if f.team.is_some_and(|t| t != r.team)
                || f.conference.as_ref().is_some_and(|c| conference_of(w, tier, r.team, r.year).as_ref() != Some(c))
                || f.spot.is_some_and(|s| spot(r.id) != Some(s))
            {
                return false;
            }
            if !f.bio() {
                return true;
            }
            let b = match views::person_index(w, r.id) {
                Some(k) => {
                    let x = &w.people[k];
                    Bio {
                        class_of: x.class_of,
                        born: x.born,
                        staff: &x.college_staff,
                        pro: x.pro.as_deref(),
                        stars: x.stars,
                        home: x.home_state,
                    }
                }
                None => {
                    let map = gone.get_or_insert_with(|| {
                        w.alumni.iter().enumerate().filter_map(|(k, a)| a.player.as_ref().map(|p| (p.id, k))).collect()
                    });
                    let Some(&k) = map.get(&r.id) else { return false };
                    let a = &w.alumni[k];
                    Bio {
                        class_of: a.class_of,
                        born: a.born,
                        staff: &a.college_staff,
                        pro: a.pro.as_deref(),
                        stars: a.stars,
                        home: a.home_state,
                    }
                }
            };
            let (class, rs) = class_in(w, tier, r.id, &b, r.year);
            let age = age_in(&b, r.year);
            // Draft eligible next spring: three years out of high school
            // (the NFL's rule). A transfer: more than one school by then.
            let eligible = r.year + 1 - b.class_of >= 3;
            let mut schools: Vec<usize> = b.staff.iter().filter(|s| s.0 <= r.year).map(|s| s.1).collect();
            schools.sort_unstable();
            schools.dedup();
            f.class.is_none_or(|c| c == class)
                && f.redshirt.is_none_or(|x| x == rs)
                && f.age.0.is_none_or(|a| age >= a)
                && f.age.1.is_none_or(|a| age <= a)
                && f.eligible.is_none_or(|x| x == eligible)
                && f.transfer.is_none_or(|x| x == (schools.len() > 1))
                && f.stars.is_none_or(|x| x == b.stars)
                && f.home.is_none_or(|x| x == b.home)
        })
        .collect()
}

pub fn table(w: &World, args: &Value) -> Value {
    let p = place_of(w, args);
    let group = group_arg(args);
    let view = view_arg(args, group);
    let (card, _, word, ch) = card_of(w, p, group, view);
    // The season in progress, built once for the whole table, as far as
    // your charting has it.
    let (now, before) = charted_now(w, p, marks::season_now(w, p.tier, p.state));
    let ys = years(w, p, &now);
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| ys.contains(y))
        .or(ys.first().copied())
        .unwrap_or(w.year);
    let mode = mode_arg(args);
    let mut filter = Filter::of(args, p.tier);
    filter.home = match args.get("home").and_then(Value::as_str) {
        Some(c) if !c.is_empty() => w.states.iter().position(|s| s.code.eq_ignore_ascii_case(c)),
        _ => None,
    };
    // The depth counts, when the card, the reading or the filter needs them.
    let src = if card.iter().any(|c| c.deep()) || mode == "adj" || filter.spot.is_some() {
        table_src(w, p, year, before)
    } else {
        Src::None
    };
    let min = min_for(w, p, group, year, ch, args, &now);
    let rows = pool(w, p, year, group, view, &now, &src);
    let qual: Vec<bool> = rows.iter().map(|r| r.vol >= min).collect();
    // Percentiles stay among every qualifier: a freshman's shade says
    // where he stands in the whole field, not among freshmen.
    let pcts = percentiles(&rows, &qual, &card);
    let spot = |id: u32| src.one(id).and_then(|o| spot_code(&o, group));
    let pass = passes(w, p.tier, &rows, &filter, &spot);
    let sort = args.get("sort").and_then(Value::as_u64).map(|k| k as usize).filter(|&k| k < card.len()).unwrap_or(0);
    let dir = args.get("dir").and_then(Value::as_i64).unwrap_or(-1);
    let qualified = qual.iter().filter(|&&q| q).count();
    let mut order: Vec<usize> = (0..rows.len()).filter(|&i| qual[i] && pass[i]).collect();
    // Sorted by percentile, so "best first" is the same for every metric.
    order.sort_by(|&a, &b| {
        // A card with no numbers (the box score line's blockers and
        // punters) keeps the pool's order.
        let at = |i: usize| pcts[i].get(sort).copied().flatten().unwrap_or(-1.0);
        let (x, y) = (at(a), at(b));
        let o = y.total_cmp(&x).then(rows[a].id.cmp(&rows[b].id));
        if dir > 0 { o.reverse() } else { o }
    });
    // The era index and the opponent-adjusted numbers (items 12 and 13),
    // shown beside the raw ones.
    let shown = match mode {
        "era" => era_vals(&rows, &qual, &card),
        "adj" => adj_vals(w, p, year, group, &rows, &qual, &card, &src),
        _ => Vec::new(),
    };
    let cell = |i: usize, k: usize| -> Option<String> {
        let cm = card[k];
        match mode {
            "era" if cm.about().2 != Fmt::Whole => shown[i][k].map(|v| format!("{v:.0}")),
            "adj" if an::opp_adjust(cm).is_some() => shown[i][k].map(|v| fmt_cell(cm, v)),
            _ => rows[i].vals[k].map(|v| fmt_cell(cm, v)),
        }
    };
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(150).min(500) as usize;
    let out: Vec<Value> = order
        .iter()
        .take(limit)
        .enumerate()
        .map(|(n, &i)| {
            let r = &rows[i];
            let mut row = json!({
                "rank": n + 1,
                "id": r.id,
                "name": name(w, r.id),
                "linked": views::person_index(w, r.id).is_some(),
                "pos": r.pos.name(),
                "team": team_json(w, p.tier, r.team),
                "games": r.games,
                "vol": r.vol.round() as i64,
                "cells": (0..card.len()).map(|k| cell(i, k)).collect::<Vec<_>>(),
                "pct": pcts[i].iter().map(|x| x.map(|x| x.round() as i64)).collect::<Vec<_>>(),
            });
            if mode != "raw" {
                row["raw"] =
                    json!(r.vals.iter().zip(&card).map(|(v, cm)| v.map(|v| fmt_cell(*cm, v))).collect::<Vec<_>>());
            }
            row
        })
        .collect();
    let mut m = base(w, p, group, year, &ys, min, word, ch);
    if year == w.year {
        m.insert("charted_before".into(), json!(before.map(|d| d.to_string())));
    }
    m.insert("columns".into(), json!(columns(&card, &rows, &qual)));
    m.insert("sort".into(), json!(sort));
    m.insert("total".into(), json!(order.len()));
    m.insert("qualified".into(), json!(qualified));
    m.insert("pool".into(), json!(rows.len()));
    // The card's views (items 4-11 and 28) and the reading (12, 13).
    m.insert(
        "views".into(),
        json!(if ch { an::views(group).iter().map(|v| v.0).collect::<Vec<_>>() } else { Vec::new() }),
    );
    m.insert("view".into(), json!(view));
    m.insert("mode".into(), json!(mode));
    m.insert("deep_from".into(), json!(w.tally_seasons().first()));
    // The filter's choices at this level, and what is picked.
    let classes: Vec<(&str, &str)> = match p.tier {
        Tier::Nfl => {
            vec![("1", "Rookie"), ("2", "2nd year"), ("3", "3rd year"), ("4", "4th year"), ("5", "5th year +")]
        }
        _ => vec![("Fr", "Freshman"), ("So", "Sophomore"), ("Jr", "Junior"), ("Sr", "Senior")],
    };
    let picked = filter.class.and_then(|c| classes.get(c as usize - 1)).map(|c| c.0);
    m.insert("classes".into(), json!(classes.iter().map(|c| [c.0, c.1]).collect::<Vec<_>>()));
    m.insert("class".into(), json!(picked));
    m.insert("rs".into(), json!(filter.redshirt.map(|r| if r { "yes" } else { "no" })));
    m.insert("age_min".into(), json!(filter.age.0));
    m.insert("age_max".into(), json!(filter.age.1));
    // Item 14's choices: the teams and conferences in the pool, the spots,
    // the home states.
    let mut teams: Vec<usize> = rows.iter().zip(&qual).filter(|(_, q)| **q).map(|(r, _)| r.team).collect();
    teams.sort_unstable();
    teams.dedup();
    let mut team_list: Vec<(usize, String)> = teams
        .iter()
        .map(|&t| {
            let n = team_json(w, p.tier, t).get("name").and_then(Value::as_str).unwrap_or("").to_string();
            (t, n)
        })
        .collect();
    team_list.sort_by(|a, b| a.1.cmp(&b.1));
    let mut confs: Vec<String> = teams.iter().filter_map(|&t| conference_of(w, p.tier, t, year)).collect();
    confs.sort();
    confs.dedup();
    m.insert("teams".into(), json!(team_list.iter().map(|t| json!([t.0, t.1])).collect::<Vec<_>>()));
    m.insert("conferences".into(), json!(confs));
    m.insert(
        "spots".into(),
        json!(if src.is_some() || ch {
            spots_for(group).iter().map(|s| json!([s.0, s.1])).collect::<Vec<_>>()
        } else {
            Vec::new()
        }),
    );
    m.insert("team".into(), json!(filter.team));
    m.insert("conference".into(), json!(filter.conference));
    m.insert("spot".into(), json!(filter.spot));
    m.insert("eligible".into(), json!(filter.eligible.map(|r| if r { "yes" } else { "no" })));
    m.insert("transfer".into(), json!(filter.transfer.map(|r| if r { "yes" } else { "no" })));
    m.insert("stars".into(), json!(filter.stars));
    m.insert("home".into(), json!(filter.home.and_then(|h| w.states.get(h)).map(|s| s.code.as_str())));
    m.insert("filtered".into(), json!(filter.on()));
    m.insert("rows".into(), json!(out));
    Value::Object(m)
}

/// Where a player is now, for "seasons like his".
fn now_text(w: &World, id: u32) -> String {
    let Some(k) = views::person_index(w, id) else { return "Out of football".into() };
    let x = &w.people[k];
    match x.stage {
        Stage::HighSchool => format!("High school · {}", w.school_name(x.hs)),
        Stage::College | Stage::PrepSchool | Stage::Away => {
            x.college.map_or_else(|| "College".to_string(), |c| format!("College · {}", w.program_name(c)))
        }
        Stage::Declared => "In the draft pool".into(),
        Stage::Pro => x
            .pro
            .as_ref()
            .and_then(|c| c.team)
            .map_or_else(|| "Pro football".to_string(), |t| format!("NFL · {}", w.nfl_team_name(t))),
        Stage::Done => "Out of football".into(),
    }
}

/// His listed position in one season at a place, read where `pool` reads
/// that season, else (no line there, or an older NFL file's seasons, which
/// keep none) his position now.
fn season_pos(w: &World, k: usize, p: Place, year: i32, now: &[NowLine]) -> Position {
    let x = &w.people[k];
    let id = x.player.id;
    let listed = if year == w.year && !now.is_empty() {
        now.iter().find(|l| l.id == id && now_in(w, p, l)).map(|l| l.pos)
    } else {
        match p.tier {
            Tier::College => w
                .stats
                .career(id)
                .iter()
                .find(|r| r.year as i32 == year && Some(r.division) == p.division)
                .map(|r| r.pos),
            Tier::Nfl => w.marks.pro_rows.iter().find(|r| r.id == id && r.year as i32 == year).map(|r| r.pos),
            Tier::Hs => x
                .hs_lines
                .iter()
                .find(|l| l.year as i32 == year && w.high_schools.get(l.school as usize).map(|h| h.state) == p.state)
                .map(|l| l.pos),
        }
    };
    listed.unwrap_or(x.player.position)
}

/// Whether a line of the season in progress is at the card's place (a
/// college line in its division; the NFL's and a state's are already).
fn now_in(w: &World, p: Place, l: &NowLine) -> bool {
    p.tier != Tier::College || w.programs.get(l.team).map(|q| q.division) == p.division
}

/// A player's card (item 7) and the past seasons closest to his (item 8).
/// The table's `year` and `division` pick the season it was opened from.
pub fn card(w: &World, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_u64).map(|x| x as u32) else {
        return json!({ "error": "No player." });
    };
    let Some(k) = views::person_index(w, id) else { return json!({ "error": "No such player." }) };
    let x = &w.people[k];
    // His level: the one asked for, else where he plays now.
    let tier = match args.get("level").and_then(Value::as_str) {
        Some(_) => tier_arg(w, args),
        None => match x.stage {
            Stage::Pro => Tier::Nfl,
            Stage::HighSchool => Tier::Hs,
            _ => Tier::College,
        },
    };
    // The division asked for (the table's), else his program's now, else
    // his last season's.
    let p = Place {
        tier,
        division: (tier == Tier::College)
            .then(|| {
                args.get("division")
                    .and_then(Value::as_str)
                    .and_then(Division::parse)
                    .or_else(|| x.college.map(|c| w.programs[c].division))
                    .or_else(|| w.stats.career(id).last().map(|r| r.division))
            })
            .flatten(),
        // The state asked for (the table's), else his school's.
        state: (tier == Tier::Hs)
            .then(|| if args.get("state").is_some() { state_arg(w, args) } else { w.high_schools[x.hs].state }),
    };
    // The season in progress, built once for the whole card, as far as
    // your charting has it.
    let (now, _before) = charted_now(w, p, marks::season_now(w, tier, p.state));
    // His seasons at the level, newest first.
    let mut mine: Vec<i32> = match tier {
        Tier::College => {
            w.stats.career(id).iter().filter(|r| Some(r.division) == p.division).map(|r| r.year as i32).collect()
        }
        Tier::Nfl => {
            x.pro.as_ref().map_or(Vec::new(), |c| c.seasons.iter().filter(|s| s.games > 0).map(|s| s.year).collect())
        }
        Tier::Hs => x
            .hs_lines
            .iter()
            .filter(|l| w.high_schools.get(l.school as usize).map(|h| h.state) == p.state)
            .map(|l| l.year as i32)
            .collect(),
    };
    if now.iter().any(|l| l.id == id && now_in(w, p, l)) {
        mine.push(w.year);
    }
    mine.sort_unstable();
    mine.dedup();
    mine.reverse();
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| mine.contains(y))
        .or(mine.first().copied());
    let Some(year) = year else {
        let Some(group) = an::group_of(x.player.position) else {
            return json!({ "error": "No card for long snappers." });
        };
        let (_, _, word, ch) = card_of(w, p, group, 0);
        let mut m = base(w, p, group, w.year, &mine, 0.0, word, ch);
        m.insert("id".into(), json!(id));
        m.insert("name".into(), json!(x.name));
        m.insert("empty".into(), json!("No season at this level yet."));
        return Value::Object(m);
    };
    // The season's group: a pool keeps a season by the position he was
    // listed at that season, which can differ from his position now.
    // The table's group when it is the returners', else his season's.
    let asked = (group_arg(args) == an::RET && args.get("group").is_some()).then_some(an::RET);
    let Some(group) = asked.or_else(|| an::group_of(season_pos(w, k, p, year, &now))) else {
        return json!({ "error": "No card for long snappers." });
    };
    let view = view_arg(args, group);
    let (card, _, word, ch) = card_of(w, p, group, view);
    let min = min_for(w, p, group, year, ch, &Value::Null, &now);
    let (_, before) = charted_now(w, p, Vec::new());
    let src = if card.iter().any(|c| c.deep()) { table_src(w, p, year, before) } else { Src::None };
    let rows = pool(w, p, year, group, view, &now, &src);
    let qual: Vec<bool> = rows.iter().map(|r| r.vol >= min).collect();
    let pcts = percentiles(&rows, &qual, &card);
    let Some(me) = rows.iter().position(|r| r.id == id) else {
        return json!({ "error": "No line that season." });
    };
    let r = &rows[me];
    let mids = middles(&rows, &qual, &card);
    let metrics: Vec<Value> = card
        .iter()
        .enumerate()
        .map(|(k, cm)| {
            let (short, long, _, _) = cm.about();
            json!({
                "label": short, "long": long, "better": better_word(*cm),
                "key": super::glossary::cm_key(*cm), "mid": mids[k].map(|v| fmt_cell(*cm, v)),
                "value": r.vals[k].map(|v| fmt_cell(*cm, v)),
                "pct": pcts[me][k].map(|x| x.round() as i64),
            })
        })
        .collect();
    // Seasons like his: every other season's field at the level, each read
    // against its own field (proposed, not confirmed: the closeness is the
    // mean gap in percentiles over the numbers both seasons have, at least
    // half the card).
    // Read on the overview card's stat book numbers (the depth counts
    // begin with the tallies, so older seasons have none).
    let my_pct = &pcts[me];
    let book: Vec<bool> = card.iter().map(|c| !c.deep()).collect();
    let book_n = book.iter().filter(|b| **b).count();
    let mut like: Vec<(f64, u32, i32, usize, u32)> = Vec::new();
    let ys = years(w, p, &now);
    let past = if view == 0 { past_pools(w, p, group, &ys) } else { Vec::new() };
    for (y, rs) in past {
        if y == year {
            continue;
        }
        let q: Vec<bool> = rs.iter().map(|r| r.vol >= an::default_min(group, ch, tier == Tier::Nfl)).collect();
        let ps = percentiles(&rs, &q, &card);
        for (i, o) in rs.iter().enumerate() {
            if !q[i] || o.id == id {
                continue;
            }
            let pairs: Vec<f64> = my_pct
                .iter()
                .zip(&ps[i])
                .zip(&book)
                .filter(|(_, b)| **b)
                .filter_map(|((a, b), _)| Some((a.as_ref()? - b.as_ref()?).abs()))
                .collect();
            if pairs.is_empty() || pairs.len() * 2 < book_n.max(1) {
                continue;
            }
            let gap = pairs.iter().sum::<f64>() / pairs.len() as f64;
            like.push((gap, o.id, o.year, o.team, o.games));
        }
    }
    like.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    // One season a player: his closest.
    let mut seen = HashSet::new();
    like.retain(|l| seen.insert(l.1));
    let like: Vec<Value> = like
        .iter()
        .take(5)
        .map(|&(gap, oid, y, team, games)| {
            json!({
                "id": oid,
                "name": name(w, oid),
                "linked": views::person_index(w, oid).is_some(),
                "year": y,
                "team": team_json(w, tier, team),
                "games": games,
                "close": (100.0 - gap).round() as i64,
                "now": now_text(w, oid),
            })
        })
        .collect();
    let mut m = base(w, p, group, year, &ys, min, word, ch);
    m.insert("id".into(), json!(id));
    m.insert("name".into(), json!(x.name));
    m.insert("pos".into(), json!(r.pos.name()));
    m.insert("team".into(), team_json(w, tier, r.team));
    m.insert("games".into(), json!(r.games));
    m.insert("vol".into(), json!(r.vol.round() as i64));
    m.insert("qualified".into(), json!(qual[me]));
    m.insert("field".into(), json!(qual.iter().filter(|q| **q).count()));
    m.insert("seasons".into(), json!(mine));
    m.insert("metrics".into(), json!(metrics));
    m.insert("like".into(), json!(like));
    m.insert(
        "views".into(),
        json!(if ch { an::views(group).iter().map(|v| v.0).collect::<Vec<_>>() } else { Vec::new() }),
    );
    m.insert("view".into(), json!(view));
    Value::Object(m)
}
