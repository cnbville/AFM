//! The NFL's five chases (L1-L5), on what the front office lane built:
//! contracts and the cap sheet, the draft, trades and comp picks, the
//! owner. L1 reads your contention window from the real books; L2 runs the
//! quarterback search; L3 grades your draft classes in public, on draft
//! night and three years on; L4 is the owner's leash, with the asks he
//! makes; L5 tracks the runs the league is built to stop. Nothing here
//! bends a result: the rewards pay for what the world's results did.
//! Sizes are proposed, not confirmed.

use super::{ACT_OWNER, Bank, key, letter, memo, pay};
use crate::calendar::Date;
use crate::positions::Position;
use crate::world::World;
use crate::world::desk::{Answer, Head};
use crate::world::stories::TeamKey;
use crate::world::storylines::catalogue::{K, Rating};

/// What the NFL chases keep.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Nfl {
    /// Your window choice each league year: 0 hold, 1 push in, 2 reload.
    pub choice: Vec<(i32, u8)>,
    /// Your starting quarterback's league rank each season: (year, rank, id).
    pub qb: Vec<(i32, u8, u32)>,
    /// The search's first season without a franchise quarterback.
    pub search_since: Option<i32>,
    /// Draft classes: night grades and the re-grades three years on.
    pub classes: Vec<Class>,
    /// The owner's trust at each season's end.
    pub trust: Vec<(i32, f32)>,
    /// The owner's asks and your answers.
    pub asks: Vec<Ask>,
}

/// One of your draft classes, judged in public.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Class {
    pub year: i32,
    pub team: u16,
    /// (player id, round, pick, public rank or 0 when off the boards).
    pub picks: Vec<(u32, u8, u16, u16)>,
    /// Draft night's grade: a score and its letter.
    pub night: f32,
    pub night_letter: String,
    /// Three years on.
    pub regrade: Option<f32>,
    pub regrade_letter: String,
}

/// An ask from the owner.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Ask {
    pub date: Date,
    /// The football year it was asked in.
    #[serde(default)]
    pub season: i32,
    /// 0 fire a coordinator, 1 a quarterback in round one.
    pub kind: u8,
    /// The coordinator (kind 0).
    pub coach: Option<u32>,
    /// 0 waiting, 1 complied, 2 argued, 3 refused.
    pub answer: u8,
    pub settled: bool,
}

fn team(w: &World) -> Option<usize> {
    match super::you(w)?.1 {
        TeamKey::Nfl(t) => Some(t as usize),
        _ => None,
    }
}

/// The league year whose books matter now (the new one from March).
fn book_year(w: &World) -> i32 {
    w.year
}

/// Your starting quarterback.
pub fn starter_qb(w: &World, t: usize) -> Option<usize> {
    w.clubs.get(t)?.depth.get(Position::QB as usize)?.first().copied()
}

/// Every club's starting quarterback ranked by how good he is: your QB's
/// place among them (1 the best).
pub fn qb_rank(w: &World, t: usize) -> Option<(u8, usize)> {
    let mine = starter_qb(w, t)?;
    let mut all: Vec<(f64, usize)> =
        (0..w.clubs.len()).filter_map(|c| starter_qb(w, c)).map(|i| (w.people[i].player.overall(), i)).collect();
    all.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let k = all.iter().position(|x| x.1 == mine)?;
    Some(((k + 1) as u8, mine))
}

/// A franchise quarterback: a top-12 starter.
pub(crate) fn franchise(rank: u8) -> bool {
    rank <= 12
}

/// The contention window, read from the books.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Window {
    /// "Wide open, two more years".
    pub words: String,
    pub open: u8,
    /// Cap room by league year: (year, dollars).
    pub room: Vec<(i32, f64)>,
    pub qb: String,
    pub core: String,
    pub dead: f64,
    pub firsts: u32,
    pub picks: u32,
    pub choice: u8,
}

/// The core: the 22 best on the roster, their average age.
fn core_age(w: &World, t: usize) -> f64 {
    let mut v: Vec<(f64, u32)> =
        w.clubs[t].roster.iter().map(|&i| (w.people[i].player.overall(), w.people[i].age_on(w.today))).collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.truncate(22);
    v.iter().map(|x| x.1 as f64).sum::<f64>() / v.len().max(1) as f64
}

/// The next draft's picks a club owns: (first-rounders, all).
fn picks_owned(w: &World, t: usize, year: i32) -> (u32, u32) {
    let (mut first, mut all) = (0, 0);
    for round in 1..=7u32 {
        for orig in 0..w.nfl.len() {
            if w.market.picks.forfeited.contains(&(year, round, orig)) {
                continue;
            }
            if w.market.picks.owner_of(year, round, orig) == t {
                all += 1;
                if round == 1 {
                    first += 1;
                }
            }
        }
    }
    (first, all)
}

/// L1: your window now.
pub fn window(w: &World, t: usize) -> Window {
    let y = book_year(w);
    let room: Vec<(i32, f64)> = (0..3).map(|k| (y + k, crate::world::pro::cap_room(w, t, y + k))).collect();
    let cap = crate::world::pro::money::price_cap(y).max(1.0);
    let (qb_words, qb_years, rookie) = match starter_qb(w, t) {
        Some(i) => {
            let p = &w.people[i];
            let c = p.pro.as_ref().and_then(|x| x.contract.as_ref());
            let left = c.map_or(0, |c| (c.last_year() - y).max(0));
            let hit = c.map_or(0.0, |c| c.cap_hit(y));
            let rookie = c.is_some_and(|c| c.kind == crate::world::pro::money::Kind::Rookie);
            let deal = if rookie { "on his rookie deal" } else { "on a veteran deal" };
            (
                format!(
                    "{} {deal}: {} this year, {}",
                    p.name,
                    crate::world::front_office::dollars(hit),
                    match left {
                        0 => "and it ends this year".to_string(),
                        1 => "1 more year".to_string(),
                        n => format!("{n} more years"),
                    }
                ),
                left,
                rookie,
            )
        }
        None => ("No starter named".into(), 0, false),
    };
    let age = core_age(w, t);
    let dead = w.clubs[t].dead_in(y);
    let (firsts, picks) = picks_owned(w, t, y + 1);
    let rank = qb_rank(w, t).map_or(32, |x| x.0);
    let room_now = room.first().map_or(0.0, |x| x.1) / cap;
    // How open: a franchise QB cheap or signed, a young enough core, room.
    let mut open = 0u8;
    if franchise(rank) {
        open += 2;
    }
    if rookie && franchise(rank) {
        open += 1;
    }
    if age <= 27.5 {
        open += 1;
    }
    if room_now >= 0.06 {
        open += 1;
    }
    if dead / cap > 0.12 {
        open = open.saturating_sub(1);
    }
    let years = if franchise(rank) { qb_years.clamp(1, 4) } else { 0 };
    let words = match open {
        4.. => format!("Wide open, {} more", plural_years(years)),
        3 => format!("Open, {} more", plural_years(years)),
        2 => "Closing".to_string(),
        _ => "Shut until the quarterback is found".to_string(),
    };
    let choice = w.legacy.nfl.choice.iter().rev().find(|x| x.0 == y).map_or(0, |x| x.1);
    Window {
        words,
        open,
        room,
        qb: qb_words,
        core: format!("The core's average age: {age:.1}"),
        dead,
        firsts,
        picks,
        choice,
    }
}

fn plural_years(n: i32) -> String {
    match n {
        1 => "one year".into(),
        2 => "two years".into(),
        3 => "three years".into(),
        n => format!("{n} years"),
    }
}

/// Your window choice for this league year.
pub fn set_choice(w: &mut World, c: u8) {
    let y = book_year(w);
    w.legacy.nfl.choice.retain(|x| x.0 != y);
    w.legacy.nfl.choice.push((y, c.min(2)));
}

/// Division champions of a season, from the standings.
pub fn div_champs(w: &World, year: i32) -> Vec<usize> {
    let rows: Vec<&crate::world::nfl_year::NflTeamSeason> = w.nfl_seasons.iter().filter(|s| s.year == year).collect();
    let mut best: Vec<(u8, f64, u32, usize)> = Vec::new();
    for s in &rows {
        let Some(e) = crate::world::franchise::era_in(w, s.team, year) else { continue };
        let pct = (s.wins as f64 + 0.5 * s.ties as f64) / (s.wins + s.losses + s.ties).max(1) as f64;
        match best.iter_mut().find(|b| b.0 == e.div) {
            Some(b) if pct > b.1 || (pct == b.1 && s.round > b.2) => *b = (e.div, pct, s.round, s.team),
            Some(_) => {}
            None => best.push((e.div, pct, s.round, s.team)),
        }
    }
    best.into_iter().map(|b| b.3).collect()
}

/// The runs (L5) of a club through a season: (playoff seasons in a row,
/// division titles in a row, conference titles, Super Bowls in a row).
pub fn runs(w: &World, t: usize, through: i32) -> (u32, u32, u32, u32) {
    let mut seasons: Vec<&crate::world::nfl_year::NflTeamSeason> =
        w.nfl_seasons.iter().filter(|s| s.team == t && s.year <= through).collect();
    seasons.sort_by_key(|s| s.year);
    let (mut po, mut div, mut sb) = (0, 0, 0);
    for s in seasons.iter().rev() {
        if s.round > 0 { po += 1 } else { break }
    }
    for s in seasons.iter().rev() {
        if div_champs(w, s.year).contains(&t) { div += 1 } else { break }
    }
    for s in seasons.iter().rev() {
        if s.round == 5 { sb += 1 } else { break }
    }
    let conf = seasons.iter().filter(|s| s.round >= 4).count() as u32;
    (po, div, conf, sb)
}

/// The league's longest division title run, for the board.
pub fn league_best_div_run(w: &World) -> Option<(usize, u32, i32)> {
    let years: Vec<i32> = {
        let mut v: Vec<i32> = w.nfl_seasons.iter().map(|s| s.year).collect();
        v.sort();
        v.dedup();
        v
    };
    let mut cur: Vec<(usize, u32)> = Vec::new();
    let mut best: Option<(usize, u32, i32)> = None;
    for y in years {
        let champs = div_champs(w, y);
        cur.retain(|x| champs.contains(&x.0));
        for &c in &champs {
            match cur.iter_mut().find(|x| x.0 == c) {
                Some(x) => x.1 += 1,
                None => cur.push((c, 1)),
            }
        }
        for &(c, n) in &cur {
            if best.is_none_or(|b| n > b.1) {
                best = Some((c, n, y));
            }
        }
    }
    best
}

fn grade_letter(s: f32) -> &'static str {
    match s {
        x if x >= 0.3 => "A",
        x if x >= 0.15 => "B+",
        x if x >= 0.0 => "B",
        x if x >= -0.15 => "C+",
        x if x >= -0.3 => "C",
        _ => "D",
    }
}

/// Draft night: the media grade your class against the consensus.
fn grade_night(w: &mut World, t: usize) {
    let year = w.year;
    if w.legacy.nfl.classes.iter().any(|c| c.year == year && c.team as usize == t) {
        return;
    }
    let mut picks: Vec<(u32, u8, u16, u16)> = Vec::new();
    for p in &w.people {
        let Some(pro) = p.pro.as_ref() else { continue };
        let Some((y, round, pick, team)) = pro.draft else { continue };
        if y != year || team != t {
            continue;
        }
        let public = w
            .scouting
            .records
            .drafts
            .iter()
            .find(|g| g.year == year && g.id == p.player.id)
            .and_then(|g| g.public)
            .unwrap_or(0);
        picks.push((p.player.id, round as u8, pick as u16, public as u16));
    }
    if picks.is_empty() {
        return;
    }
    picks.sort_by_key(|x| x.2);
    // Each pick against the consensus: a steal when the public had him
    // higher than the slot, a reach when lower or off the boards.
    let score = picks
        .iter()
        .map(|&(_, _, pick, public)| {
            let pick = pick as f32;
            let public = if public == 0 { 300.0 } else { public as f32 };
            ((pick - public) / pick.max(12.0)).clamp(-1.0, 1.0)
        })
        .sum::<f32>()
        / picks.len() as f32;
    let letter = grade_letter(score).to_string();
    w.legacy.nfl.classes.push(Class {
        year,
        team: t as u16,
        picks,
        night: score,
        night_letter: letter.clone(),
        regrade: None,
        regrade_letter: String::new(),
    });
    super::story(
        w,
        K::DraftVerdict,
        year as u64,
        letter.clone(),
        format!("Draft night: the shows gave the class a {letter}."),
    );
    let mocked = score < -0.15;
    memo(
        w,
        "legacy-draft-night",
        format!("Draft night grades: {letter}"),
        vec![if mocked {
            "The draft shows are mocking the class: too many reaches on the boards they keep.".to_string()
        } else if score >= 0.15 {
            "The draft shows love the class: steals on every board they keep.".to_string()
        } else {
            "The draft shows call the class solid.".to_string()
        }],
    );
}

/// Three years on: how each pick turned out.
fn regrade(w: &mut World, t: usize) {
    let year = w.year;
    let due: Vec<usize> = (0..w.legacy.nfl.classes.len())
        .filter(|&k| {
            let c = &w.legacy.nfl.classes[k];
            c.team as usize == t && c.regrade.is_none() && c.year + 3 <= year
        })
        .collect();
    for k in due {
        let class = w.legacy.nfl.classes[k].clone();
        let mut score = 0.0f32;
        let mut hits_mocked = 0;
        for &(id, round, _, public) in &class.picks {
            let Some(p) = w.people.iter().find(|p| p.player.id == id) else {
                score -= 0.3;
                continue;
            };
            let pro = p.pro.as_ref();
            let starts: u32 = pro.map_or(0, |x| x.seasons.iter().map(|s| s.starts).sum());
            let bowls = pro.map_or(0, |x| x.seasons.iter().filter(|s| s.pro_bowl).count());
            // What a round is expected to give in three years.
            let expect = match round {
                1 => 30.0,
                2 => 20.0,
                3 => 12.0,
                4 => 6.0,
                _ => 3.0,
            };
            let s = ((starts as f32 + 12.0 * bowls as f32 - expect) / expect.max(8.0)).clamp(-1.0, 1.5);
            score += s;
            if (public == 0 || public as u32 > 32 * round as u32 + 32) && starts >= 16 {
                hits_mocked += 1;
            }
        }
        score /= class.picks.len().max(1) as f32;
        let letter = grade_letter(score).to_string();
        let c = &mut w.legacy.nfl.classes[k];
        c.regrade = Some(score);
        c.regrade_letter = letter.clone();
        let what = format!("The {} class re-graded: {letter} (draft night: {})", class.year, class.night_letter);
        let right = score >= class.night;
        super::story_end(w, K::DraftVerdict, class.year as u64, if right { "proved_right" } else { "proved_wrong" });
        if score >= 0.15 {
            pay(w, key("L3", class.year as u64, 1), "L3", what.clone(), 0.75, Bank::Rating(Rating::Evaluation), 1.0);
            super::boss_by(w, 0.05);
        } else if score < -0.3 {
            super::cost(w, key("L3", class.year as u64, 2), "L3", what.clone(), 0.5);
            super::boss_by(w, -0.05);
        }
        for h in 0..hits_mocked.min(3) {
            pay(
                w,
                key("L3", class.year as u64, 10 + h as u64),
                "L3",
                format!("A pick mocked on draft night in {} became a starter", class.year),
                0.5,
                Bank::Rating(Rating::Evaluation),
                0.5,
            );
        }
        memo(w, "legacy-regrade", format!("{} draft class: {letter}", class.year), vec![what]);
    }
}

/// Your picks' hit rate by round, and where it ranks among the league's
/// front offices over the same drafts.
pub fn hit_rates(w: &World, t: usize, since: i32) -> (Vec<(u8, u32, u32)>, u8) {
    let mut by_team: Vec<[(u32, u32); 7]> = vec![[(0, 0); 7]; w.nfl.len()];
    for p in &w.people {
        let Some(pro) = p.pro.as_ref() else { continue };
        let Some((y, round, _, team)) = pro.draft else { continue };
        if y < since || y > w.year - 2 || team >= by_team.len() || !(1..=7).contains(&round) {
            continue;
        }
        let starts: u32 = pro.seasons.iter().filter(|s| s.year < y + 3).map(|s| s.starts).sum();
        let hit = starts >= 16 || pro.seasons.iter().any(|s| s.pro_bowl);
        let r = &mut by_team[team][round as usize - 1];
        r.1 += 1;
        if hit {
            r.0 += 1;
        }
    }
    let rate = |a: &[(u32, u32); 7]| {
        let (h, n) = a.iter().fold((0, 0), |s, x| (s.0 + x.0, s.1 + x.1));
        h as f64 / n.max(1) as f64
    };
    let mine = by_team.get(t).map_or(0.0, rate);
    let rank = 1 + by_team.iter().enumerate().filter(|(k, a)| *k != t && rate(a) > mine).count() as u8;
    let rounds = by_team.get(t).map_or(Vec::new(), |a| {
        a.iter().enumerate().filter(|x| x.1.1 > 0).map(|(k, x)| ((k + 1) as u8, x.0, x.1)).collect()
    });
    (rounds, rank)
}

/// L4: the leash in words, from the owner's trust.
pub fn leash_words(trust: f32) -> &'static str {
    match trust {
        x if x >= 0.85 => "Leaves football to you",
        x if x >= 0.65 => "Wants to be asked",
        x if x >= 0.45 => "Presses his own picks",
        x if x >= 0.3 => "Vetoes trades",
        _ => "On the hot seat",
    }
}

fn owner_name(w: &World, t: usize) -> String {
    crate::world::folk::boss::boss(w, crate::world::folk::Team::Nfl(t)).name
}

/// The owner's asks: after a bad year on one side of the ball, he wants
/// the coordinator gone; with no franchise quarterback, a quarterback in
/// round one. More often the more he meddles and the less he trusts you.
fn owner_asks(w: &mut World, t: usize, d: Date) {
    if w.legacy.nfl.asks.iter().any(|a| !a.settled) {
        return;
    }
    let boss = crate::world::folk::boss::boss(w, crate::world::folk::Team::Nfl(t));
    let trust = super::boss_trust(w);
    let urge = (boss.meddling / 20.0) as f32 * (1.0 - trust);
    let u = crate::world::season::key_uniform(w.options.seed as u64 ^ 0x0A5C, d.0 as u64 ^ (t as u64) << 32) as f32;
    let year = w.year;
    let owner = owner_name(w, t);
    // Before the draft: a quarterback in round one (late March, after free
    // agency's first wave).
    if d == Date::ymd(year, 3, 20) && w.legacy.nfl.search_since.is_some() && u < 0.3 + urge {
        let id = letter(
            w,
            &format!("legacy-owner-qb-{year}"),
            Head::Memo,
            (owner.clone(), "Owner".into()),
            "Take a quarterback in round one".into(),
            vec![format!("{owner} wants a quarterback with the first pick you have. He's tired of waiting.")],
            vec![
                Answer {
                    label: "Promise him".into(),
                    does: "Trust now; a broken promise costs more.".into(),
                    act: ACT_OWNER,
                    target: 1,
                },
                Answer {
                    label: "Argue for the board".into(),
                    does: "He listens if he trusts you.".into(),
                    act: ACT_OWNER,
                    target: 2,
                },
                Answer {
                    label: "Refuse".into(),
                    does: "Trust falls unless you're proved right.".into(),
                    act: ACT_OWNER,
                    target: 3,
                },
            ],
        );
        if id.is_some() {
            w.legacy.nfl.asks.push(Ask { date: d, season: year, kind: 1, coach: None, answer: 0, settled: false });
            leash_opens(w, "a quarterback in round one");
        }
        return;
    }
    // The day after Black Monday: a coordinator after a bad year.
    if d != w.dates.black_monday.add(1) {
        return;
    }
    let me = TeamKey::Nfl(t as u16);
    let (wins, losses) = w
        .legacy
        .games
        .iter()
        .filter(|g| g.season == year && g.team == me && !g.post)
        .fold((0u16, 0u16), |a, g| (a.0 + (g.us > g.them) as u16, a.1 + (g.us < g.them) as u16));
    let pct = if wins + losses == 0 { 0.5 } else { wins as f64 / (wins + losses) as f64 };
    if pct >= 0.45 || u >= 0.25 + urge {
        return;
    }
    let side_dc = crate::world::season::key_uniform(w.options.seed as u64 ^ 0x0D0C, year as u64) < 0.5;
    let coach = if side_dc { w.nfl[t].dc } else { w.nfl[t].oc };
    let Some(c) = coach else { return };
    let name = w.coaches[c].name.clone();
    let role = if side_dc { "defensive coordinator" } else { "offensive coordinator" };
    let id = letter(
        w,
        &format!("legacy-owner-fire-{year}"),
        Head::Memo,
        (owner.clone(), "Owner".into()),
        format!("Make a change at {role}"),
        vec![format!("{owner} wants {name} gone after a {wins}-{losses} year.")],
        vec![
            Answer {
                label: "Let him go".into(),
                does: format!("{name} is fired; the owner's trust rises."),
                act: ACT_OWNER,
                target: 1,
            },
            Answer {
                label: "Argue for him".into(),
                does: "He listens if he trusts you.".into(),
                act: ACT_OWNER,
                target: 2,
            },
            Answer {
                label: "Refuse".into(),
                does: "Trust falls unless next year proves you right.".into(),
                act: ACT_OWNER,
                target: 3,
            },
        ],
    );
    if id.is_some() {
        w.legacy.nfl.asks.push(Ask {
            date: d,
            season: year,
            kind: 0,
            coach: Some(c as u32),
            answer: 0,
            settled: false,
        });
        leash_opens(w, &format!("a change at {role}"));
    }
}

/// The owner's leash (a private story): it opens with an ask and ends
/// trusted (you got your way and were right, or gave him his) or overruled.
fn leash_opens(w: &mut World, ask: &str) {
    let year = w.year as u64;
    let Some((me, team)) = super::you(w) else { return };
    let owner = match team {
        TeamKey::Nfl(n) => owner_name(w, n as usize),
        _ => return,
    };
    let fact = format!("{owner} asked for {ask}.");
    let spot = crate::world::storylines::Spot::new(K::OwnerLeash, crate::world::storylines::subj(me as u64, year))
        .team(team)
        .lead(crate::world::storylines::Who::Coach(me as u32))
        .word(ask.to_string())
        .fact(fact, 30.0);
    crate::world::storylines::detect::tell(w, spot);
}

fn leash_ends(w: &mut World, ask: &Ask, trusted: bool) {
    let Some((me, _)) = super::you(w) else { return };
    let sub = crate::world::storylines::subj(me as u64, ask.season as u64);
    if let Some(id) = crate::world::storylines::open_id(w, K::OwnerLeash, sub) {
        crate::world::storylines::close(w, id, if trusted { "trusted" } else { "overruled" }, None);
    }
}

/// Your answer to the owner's ask.
pub(crate) fn answer(w: &mut World, target: u32) {
    let Some(t) = team(w) else { return };
    let Some(k) = w.legacy.nfl.asks.iter().rposition(|a| a.answer == 0) else { return };
    w.legacy.nfl.asks[k].answer = target.min(3) as u8;
    let ask = w.legacy.nfl.asks[k].clone();
    let trust = super::boss_trust(w);
    match target {
        1 => {
            super::boss_by(w, 0.04);
            if ask.kind == 0
                && let Some(c) = ask.coach
            {
                let c = c as usize;
                if let Some(j) = w.coaches[c].job
                    && j.employer == crate::world::coach::Employer::Nfl(t)
                {
                    crate::world::staff::leave(w, c, "fired");
                    crate::world::staff::fill_others(w, vec![(j.employer, j.role)]);
                }
                w.legacy.nfl.asks[k].settled = true;
                leash_ends(w, &ask, true);
            }
        }
        2 => super::boss_by(w, if trust >= 0.5 { 0.02 } else { -0.03 }),
        _ => super::boss_by(w, -0.06),
    }
}

/// Asks settle with the results: a QB promise kept or broken on draft
/// night; a refusal proved right or wrong by the next season.
fn settle(w: &mut World, t: usize) {
    let year = w.year;
    for k in 0..w.legacy.nfl.asks.len() {
        let a = w.legacy.nfl.asks[k].clone();
        if a.settled || a.answer == 0 {
            continue;
        }
        let drafted_qb = w.people.iter().any(|p| {
            p.player.position == Position::QB
                && p.pro.as_ref().and_then(|x| x.draft).is_some_and(|d| d.0 == year && d.1 == 1 && d.3 == t)
        });
        let season_done = w.legacy.seasons.iter().any(|s| s.year == year && s.team == TeamKey::Nfl(t as u16));
        match (a.kind, a.answer) {
            (1, 1) if season_done || drafted_qb => {
                super::boss_by(w, if drafted_qb { 0.06 } else { -0.08 });
                w.legacy.nfl.asks[k].settled = true;
                leash_ends(w, &a, drafted_qb);
            }
            (_, 2 | 3) if season_done && (a.season < year || a.kind == 1) => {
                let good = w
                    .legacy
                    .seasons
                    .iter()
                    .rev()
                    .find(|s| s.team == TeamKey::Nfl(t as u16))
                    .is_some_and(|s| s.playoff > 0);
                if a.answer == 3 {
                    super::boss_by(w, if good { 0.08 } else { -0.04 });
                }
                w.legacy.nfl.asks[k].settled = true;
                leash_ends(w, &a, good);
            }
            _ => {}
        }
    }
}

/// Every day at your NFL club.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(t) = team(w) else { return };
    if !super::head(w) {
        return;
    }
    owner_asks(w, t, d);
    // The day after the draft: the night grades.
    let drafted =
        w.people.iter().any(|p| p.pro.as_ref().and_then(|x| x.draft).is_some_and(|x| x.0 == w.year && x.3 == t));
    if drafted && d.weekday() == crate::calendar::Weekday::Mon {
        grade_night(w, t);
    }
    if d == Date::ymd(w.year, 9, 1) {
        regrade(w, t);
    }
}

/// After your season: the quarterback search, the window, the runs and the
/// owner's trust.
pub(crate) fn season(w: &mut World) {
    let Some(t) = team(w) else { return };
    let year = w.year;
    let trust = super::boss_trust(w);
    w.legacy.nfl.trust.retain(|x| x.0 != year);
    w.legacy.nfl.trust.push((year, trust));
    settle(w, t);
    // L2: the search.
    if let Some((rank, qb)) = qb_rank(w, t) {
        let id = w.people[qb].player.id;
        w.legacy.nfl.qb.retain(|x| x.0 != year);
        w.legacy.nfl.qb.push((year, rank, id));
        let p = &w.people[qb];
        if franchise(rank) {
            if let Some(since) = w.legacy.nfl.search_since.take() {
                let draft = p.pro.as_ref().and_then(|x| x.draft);
                let late = draft.is_some_and(|d| d.1 >= 3);
                let name = p.name.clone();
                let how = match draft {
                    Some((y, r, pk, team)) if team == t => format!("your pick, round {r} ({pk} overall) in {y}"),
                    Some(_) => "from another club".to_string(),
                    None => "undrafted".to_string(),
                };
                let lesson = if late { 1.0 } else { 0.75 };
                pay(
                    w,
                    key("L2", id as u64, since as u64),
                    "L2",
                    format!("Found him: {name}, {how}, after {}", super::seasons_words(year - since)),
                    lesson,
                    Bank::Rating(Rating::Evaluation),
                    1.5,
                );
                pay(
                    w,
                    key("L2", id as u64, 2),
                    "L2",
                    format!("{name} became a franchise quarterback"),
                    0.5,
                    Bank::Teach(crate::world::coach::Group::of(Position::QB) as usize),
                    0.0,
                );
            }
        } else if w.legacy.nfl.search_since.is_none() {
            w.legacy.nfl.search_since = Some(year);
        }
    }
    // A top-three pick at quarterback who missed: three years, no answer.
    let missed = w.people.iter().any(|p| {
        p.player.position == Position::QB
            && p.pro.as_ref().and_then(|x| x.draft).is_some_and(|d| d.3 == t && d.2 <= 3 && d.0 == year - 3)
            && !w.legacy.nfl.qb.iter().any(|q| q.2 == p.player.id && franchise(q.1))
    });
    if missed {
        super::cost(w, key("L2", year as u64, 9), "L2", "A top-three quarterback who missed".into(), 0.5);
        super::boss_by(w, -0.1);
    }
    // L1: a ring with the window open; a push that paid.
    let win = window(w, t);
    let s = w.legacy.seasons.iter().rev().find(|s| s.year == year && s.team == TeamKey::Nfl(t as u16)).cloned();
    if let Some(s) = s {
        if s.champion && win.open >= 3 {
            pay(
                w,
                key("L1", year as u64, 1),
                "L1",
                "A ring with the window open".into(),
                1.0,
                Bank::Rating(Rating::Negotiating),
                2.0,
            );
        }
        if win.choice == 1 && s.playoff > 0 {
            pay(
                w,
                key("L1", year as u64, 2),
                "L1",
                "You pushed in and it paid: a playoff year".into(),
                0.5,
                Bank::Rating(Rating::Negotiating),
                0.5,
            );
        } else if win.choice == 1 {
            // A push that missed: the owner paid for a run that didn't come.
            super::boss_by(w, -0.04);
        } else if win.choice == 2 && s.losses > s.wins {
            // A reload he signed off on: a losing year he expected.
            super::boss_by(w, 0.02);
        }
    }
    // L5: each new length of a run pays once.
    let (po, div, _, sb) = runs(w, t, year);
    for (n, what, lesson) in [
        (3u32, "playoff seasons in a row", 0.5f32),
        (5, "playoff seasons in a row", 0.75),
        (8, "playoff seasons in a row", 1.0),
    ] {
        if po >= n {
            pay(w, key("L5", 1, n as u64), "L5", format!("{n} {what}"), lesson, Bank::Rating(Rating::Motivation), 1.0);
        }
    }
    for (n, lesson) in [(2u32, 0.25f32), (3, 0.5), (5, 0.75), (8, 1.0)] {
        if div >= n {
            pay(
                w,
                key("L5", 2, n as u64),
                "L5",
                format!("{n} division titles in a row"),
                lesson,
                Bank::Rating(Rating::Motivation),
                1.0,
            );
        }
    }
    if sb >= 2 {
        pay(
            w,
            key("L5", 3, sb as u64),
            "L5",
            format!("{sb} Super Bowls in a row"),
            1.0,
            Bank::Rating(Rating::GameManagement),
            3.0,
        );
    }
}

/// The Crowds ambition's steps.
pub(crate) fn crowd_steps(w: &World, t: usize, from: i32) -> [bool; 5] {
    let fin = w.finance.nfl.get(t);
    let seasons: Vec<&crate::world::finance::nfl::Season> =
        fin.map_or(Vec::new(), |f| f.seasons.iter().filter(|s| s.year >= from).collect());
    let full = |s: &&crate::world::finance::nfl::Season| s.home_games > 0 && s.sellouts >= s.home_games;
    let streak = seasons.iter().rev().take_while(|s| full(s)).count();
    let home_playoff = w
        .legacy
        .games
        .iter()
        .any(|g| g.team == TeamKey::Nfl(t as u16) && g.playoff && g.season >= from && g.date < w.dates.super_bowl)
        && w.legacy
            .seasons
            .iter()
            .any(|s| s.team == TeamKey::Nfl(t as u16) && s.year >= from && s.title && s.playoff > 0);
    [
        seasons.iter().any(|s| s.sellouts > 0),
        seasons.iter().any(|s| s.waitlist > 0),
        seasons.iter().any(full) && w.legacy.seasons.iter().any(|s| s.year >= from && s.wins >= 10),
        home_playoff,
        streak >= 3,
    ]
}

/// Why the Crowds ambition matters at your club.
pub(crate) fn crowds_why(w: &World, t: usize) -> String {
    match w.finance.nfl.get(t).and_then(|f| f.seasons.last()) {
        Some(s) if s.home_games > 0 => {
            let wait = if s.waitlist > 0 {
                format!("; {} on the season ticket waitlist", s.waitlist)
            } else {
                "; no waitlist".to_string()
            };
            format!("{} of {} home games sold out last season{wait}.", s.sellouts, s.home_games)
        }
        _ => "Crowds follow winners, and a full building in December is earned.".into(),
    }
}

/// Why a say in your own extension matters.
pub(crate) fn contract_why(w: &World) -> String {
    let trust = super::boss_trust(w);
    format!(
        "The owner today: {}. Owners give a coach a say once he has earned their trust.",
        leash_words(trust).to_lowercase()
    )
}

/// The Reach ambition's steps at an NFL club.
pub(crate) fn reach_steps(w: &World, t: usize, set: Date) -> [bool; 5] {
    let trust = super::boss_trust(w);
    let here = super::seasons_at(w, TeamKey::Nfl(t as u16));
    let years = super::ambitions::years_at(w, TeamKey::Nfl(t as u16));
    [
        years >= 2,
        trust >= 0.6,
        trust >= 0.75,
        here.iter().any(|s| s.playoff > 0 && s.year >= set.year()),
        trust >= 0.9 && years >= 3,
    ]
}
