//! V15, the locker room: who's close to whom on your team, how each man
//! feels and why, who has a say in the room, where there's friction, and
//! how well each unit knows each other. Ties, moods, their dated reasons
//! and the captains are the world's own (section 6); the friction and the
//! say in the room are read from them, and the chemistry is the seasons
//! the starters have played together, as the engine reads it.

use std::collections::{HashMap, HashSet};

use afm_engine::calendar::Date;
use afm_engine::player::PlayingTime;
use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::charts::pair_chemistry;
use afm_engine::world::folk::ties::Kind;
use afm_engine::world::folk::{Team, life, rules, takeover, talks, ties};
use afm_engine::world::people::Why;
use serde_json::{Value, json};

use crate::Reply;
use crate::views;

/// The map shows this many men at most.
const SHOWN: usize = 18;

const GROUPS: [&str; 4] = ["Offensive line", "Backs and receivers", "Defense", "Kicking"];

/// Where a position sits on the map.
fn group(p: Position) -> usize {
    match p {
        Position::OT | Position::IOL => 0,
        Position::QB | Position::RB | Position::WR | Position::TE => 1,
        Position::K | Position::P | Position::LS => 3,
        _ => 2,
    }
}

/// The mood in five steps (1 very unhappy to 5 very happy), the bands the
/// player pages use.
fn mood_step(m: f32) -> u8 {
    match m {
        x if x > 0.5 => 5,
        x if x > 0.15 => 4,
        x if x > -0.15 => 3,
        x if x > -0.5 => 2,
        _ => 1,
    }
}

const MOOD: [&str; 5] = ["Very unhappy", "Unhappy", "Settled", "Happy", "Very happy"];
const SAY: [&str; 5] = ["little", "some", "fair", "a lot", "the most"];

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn short_date(d: Date) -> String {
    let t = views::date_text(d);
    t.rsplit_once(' ').map_or(t.clone(), |x| x.0.to_string())
}

fn surname(n: &str) -> &str {
    n.split(' ').next_back().unwrap_or(n)
}

/// One friction: the starter, the man who wants his place, the position,
/// and whether the starter took his place from him.
struct Friction {
    starter: usize,
    rival: usize,
    pos: Position,
    benched: bool,
}

impl Friction {
    /// The man who wants the place, against the starter's name.
    fn wants(&self, starter: &str) -> String {
        if self.benched { format!("lost his snaps to {starter}") } else { format!("wants {starter}'s snaps") }
    }

    /// The man who wants the place, as the starter's card names him.
    fn wants_his(&self) -> &'static str {
        if self.benched { "lost his snaps to him" } else { "wants his snaps" }
    }

    /// The starter, as the man who wants his place sees him.
    fn holder(&self) -> &'static str {
        if self.benched { "took his snaps" } else { "starts ahead of him" }
    }
}

pub fn board(w: &World) -> Reply {
    let team = Team::seat(w).ok_or("Start a career first.")?;
    let tref = views::mine(w).ok_or("Start a career first.")?;
    let roster: Vec<usize> = team.roster(w);
    if roster.is_empty() {
        return Ok(json!({ "none": true, "team": views::team_brief(w, tref) }));
    }
    let on: HashMap<u32, usize> = roster.iter().map(|&i| (w.people[i].player.id, i)).collect();
    let captains: HashSet<usize> = takeover::captains_of(w, team).into_iter().collect();
    let council = &w.folk.rules.council;
    let starters = starters_of(w, &roster, &on);
    let starter: HashSet<usize> = starters.iter().flat_map(|(_, v)| v.iter().copied()).collect();

    // The say in the room: where his standing ranks on the roster
    // (captains at least "a lot").
    let mut ranked = roster.clone();
    ranked.sort_by(|&a, &b| takeover::standing(w, b).total_cmp(&takeover::standing(w, a)));
    let n = roster.len();
    let say: HashMap<usize, u8> = ranked
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let pct = k as f64 / n as f64;
            let s = match pct {
                x if x < 0.05 => 5,
                x if x < 0.2 => 4,
                x if x < 0.5 => 3,
                x if x < 0.8 => 2,
                _ => 1,
            };
            (i, if captains.contains(&i) { s.max(4) } else { s })
        })
        .collect();

    // Friction: a man behind a starter at his position, unhappy and saying
    // it's his role (his mood's own reason: a man who won't say why isn't
    // read as wanting a starter's snaps), and the starter at the bottom of
    // that position's order.
    let mut friction: Vec<Friction> = Vec::new();
    for (pos, list) in &starters {
        let Some(&last) = list.last() else { continue };
        let mut rivals: Vec<usize> = roster
            .iter()
            .copied()
            .filter(|&i| w.people[i].player.position == *pos && !list.contains(&i))
            .filter(|&i| w.people[i].mood < -0.15 && Why::from_u8(w.people[i].mood_why) == Why::Role)
            .collect();
        rivals.sort_by(|&a, &b| w.people[a].mood.total_cmp(&w.people[b].mood));
        if let Some(&r) = rivals.first() {
            let benched = talks::reasons(w, w.people[r].player.id).iter().any(|x| x.1 == "was benched");
            friction.push(Friction { starter: last, rival: r, pos: *pos, benched });
        }
    }
    friction.sort_by(|a, b| w.people[a.rival].mood.total_cmp(&w.people[b.rival].mood));
    friction.truncate(4);

    // Trouble: unhappy, sitting out, or something at home.
    let trouble = |i: usize| {
        let p = &w.people[i];
        p.mood <= -0.15 || rules::sitting(w, p.player.id, w.today) || life::open_event(w, p.player.id).is_some()
    };

    // Who the map shows: captains, the friction, the starters in trouble
    // and the most unhappy, then the starters by their say in the room.
    let mut pick: Vec<usize> = Vec::new();
    let push = |pick: &mut Vec<usize>, i: usize| {
        if !pick.contains(&i) && pick.len() < SHOWN {
            pick.push(i);
        }
    };
    for &i in &ranked {
        if captains.contains(&i) {
            push(&mut pick, i);
        }
    }
    for f in &friction {
        push(&mut pick, f.rival);
        push(&mut pick, f.starter);
    }
    let mut sad: Vec<usize> = roster.iter().copied().filter(|&i| trouble(i)).collect();
    sad.sort_by(|&a, &b| {
        (!starter.contains(&a)).cmp(&!starter.contains(&b)).then(w.people[a].mood.total_cmp(&w.people[b].mood))
    });
    for &i in sad.iter().take(4) {
        push(&mut pick, i);
    }
    for &i in &ranked {
        if starter.contains(&i) && group(w.people[i].player.position) < 3 {
            push(&mut pick, i);
        }
    }
    for &i in &ranked {
        push(&mut pick, i);
    }
    let at: HashMap<usize, usize> = pick.iter().enumerate().map(|(k, &i)| (i, k)).collect();

    // The links on the map, among the men it shows.
    let mut edges: Vec<Value> = Vec::new();
    for (ka, &a) in pick.iter().enumerate() {
        let id = w.people[a].player.id;
        // A tie of a man with himself (an old save's) is left out, and a
        // tie drawn twice shows once.
        for t in w.folk.ties.of_id(id).filter(|t| t.a == id && t.b != id && t.together) {
            let Some(&b) = on.get(&t.b) else { continue };
            let Some(&kb) = at.get(&b) else { continue };
            let (k, why) = tie_words(t.kind, t.strength, t.since);
            let e = json!({ "a": ka, "b": kb, "k": k, "why": why });
            if !edges.contains(&e) {
                edges.push(e);
            }
        }
    }
    for f in &friction {
        if let (Some(&a), Some(&b)) = (at.get(&f.rival), at.get(&f.starter)) {
            let (r, s) = (&w.people[f.rival].name, &w.people[f.starter].name);
            edges.push(json!({ "a": a, "b": b, "k": "t", "why": format!("{} {}", surname(r), f.wants(surname(s))) }));
        }
    }

    // Snaps this season.
    let snaps = snap_shares(w);
    let nodes: Vec<Value> =
        pick.iter().map(|&i| node(w, i, &on, &captains, &starter, &say, &friction, &snaps)).collect();

    // Chemistry: how well each unit's starters know each other (none kept
    // in another league).
    let unit = |f: &dyn Fn(Position) -> bool| -> Value {
        let set: Vec<usize> = starter.iter().copied().filter(|&i| f(w.people[i].player.position)).collect();
        mean_chemistry(w, &set)
    };
    let kept = roster.len() >= 2 && pair_chemistry(w, roster[0], roster[1]).is_some();
    let chem = if kept {
        json!([
            { "n": "Team", "v": unit(&|p| group(p) < 3), "of": "the starters together" },
            { "n": "Offense", "v": unit(&|p| group(p) < 2), "of": "the offense's starters" },
            { "n": "Defense", "v": unit(&|p| group(p) == 2), "of": "the defense's starters" },
            { "n": "O-line", "v": unit(&|p| group(p) == 0), "of": "the five up front" },
        ])
    } else {
        json!([])
    };

    // Worth knowing.
    let mut notes: Vec<Value> = Vec::new();
    for f in &friction {
        let (a, b) = (&w.people[f.starter], &w.people[f.rival]);
        notes.push(json!({
            "tag": "Friction",
            "t": format!("{} and {}", surname(&a.name), surname(&b.name)),
            "s": format!("{}: {} {}", f.pos.name(), b.name, f.wants(surname(&a.name))),
            "sel": at.get(&f.rival),
            "act": { "k": "talk", "id": b.player.id, "s": "role", "label": format!("Talk to {}", surname(&b.name)) },
            "warn": true,
        }));
    }
    let mut unhappy: Vec<usize> =
        pick.iter().copied().filter(|&i| w.people[i].mood <= -0.15 && !friction.iter().any(|f| f.rival == i)).collect();
    unhappy.sort_by(|&a, &b| w.people[a].mood.total_cmp(&w.people[b].mood));
    for &i in unhappy.iter().take(2) {
        let p = &w.people[i];
        let (subject, words) = talks::concern(w, i);
        // Low without one thing he points to: say so, not "settled".
        let words =
            if words.starts_with("nothing much") { "down, with no one thing he points to".into() } else { words };
        notes.push(json!({
            "tag": "Unhappy",
            "t": p.name,
            "s": cap(&words),
            "sel": at.get(&i),
            "act": { "k": "talk", "id": p.player.id, "s": subject.key(), "label": format!("Talk to {}", surname(&p.name)) },
            "warn": true,
        }));
    }
    // The strongest mentor tie of a man on the map, from the shown men's
    // own ties (not the world's whole list).
    let mentor = pick
        .iter()
        .flat_map(|&i| w.folk.ties.of_id(w.people[i].player.id))
        .filter(|t| t.kind == Kind::Mentor && t.together && t.a != t.b)
        .filter_map(|t| Some((on.get(&t.a)?, on.get(&t.b)?, t.strength)))
        .max_by_key(|x| x.2);
    if let Some((&a, &b, _)) = mentor {
        let (m, s) = (&w.people[a], &w.people[b]);
        notes.push(json!({
            "tag": "Mentor",
            "t": m.name,
            "s": format!("Teaching {}, {}", s.name, s.player.position.name()),
            "sel": at.get(&a).or(at.get(&b)),
            "act": { "k": "page", "id": m.player.id, "label": "His page" },
            "warn": false,
        }));
    }

    // Every man, for the groups, morale and leaders views.
    let rows: Vec<Value> = ranked
        .iter()
        .map(|&i| {
            let p = &w.people[i];
            let id = p.player.id;
            let why = talks::reasons(w, id)
                .into_iter()
                .next()
                .map(|x| cap(&x.1))
                .unwrap_or_else(|| cap(&talks::concern(w, i).1));
            let sc = snaps_cell(w, i, &snaps);
            json!({
                "id": id,
                "name": p.name,
                "pos": p.player.position.name(),
                "g": group(p.player.position),
                "cls": views::class_text(w, p),
                "m": mood_step(p.mood),
                "say": say[&i],
                "cap": captains.contains(&i),
                "council": council.contains(&id),
                "starter": starter.contains(&i),
                "friends": friends_here(w, id, &on),
                "why": why,
                "snaps": sc["v"],
                // The table's head says snaps: only last year's role needs a word.
                "snapsSub": if sc["sub"] == "last year" { sc["sub"].clone() } else { Value::Null },
                "shown": at.get(&i),
            })
        })
        .collect();

    // The rooms: each position's mood, ties inside it and its starters'
    // chemistry.
    let rooms: Vec<Value> = Position::ALL
        .iter()
        .filter_map(|&pos| {
            let men: Vec<usize> = roster.iter().copied().filter(|&i| w.people[i].player.position == pos).collect();
            if men.is_empty() {
                return None;
            }
            let mood = men.iter().map(|&i| w.people[i].mood).sum::<f32>() / men.len() as f32;
            let ids: HashSet<u32> = men.iter().map(|&i| w.people[i].player.id).collect();
            // Each pair's ties from its men's own lists, counted once (a
            // tie drawn twice counts once, as on the map).
            let mut pairs: HashSet<(u32, u32, u8)> = HashSet::new();
            for &id in &ids {
                for t in w.folk.ties.of_id(id).filter(|t| t.together && t.a != t.b) {
                    if ids.contains(&t.a) && ids.contains(&t.b) {
                        pairs.insert((t.a.min(t.b), t.a.max(t.b), t.kind as u8));
                    }
                }
            }
            let links = pairs.len();
            let st: Vec<usize> = men.iter().copied().filter(|i| starter.contains(i)).collect();
            let chem = mean_chemistry(w, &st);
            Some(json!({
                "pos": pos.name(),
                "g": group(pos),
                "n": men.len(),
                "mood": MOOD[mood_step(mood) as usize - 1],
                "low": mood <= -0.15,
                "unhappy": men.iter().filter(|&&i| w.people[i].mood <= -0.15).count(),
                "links": links,
                "chem": chem,
            }))
        })
        .collect();

    let mood = talks::team_mood(w, team);
    // The last game, when it was this past week and your team's (a game
    // from the job you just left isn't).
    let last = w.seat_team().and_then(|(level, t)| {
        let g = w.seat_games().last().filter(|g| g.date.days_to(w.today) <= 7 && g.level == level)?;
        let us = g.teams.iter().position(|&x| x == t)?;
        Some(g.names[1 - us].clone())
    });
    Ok(json!({
        "team": views::team_brief(w, tref),
        "date": short_date(w.today),
        "after": last,
        "mood": { "words": talks::mood_words(mood), "step": mood_step(mood) },
        "roster": n,
        "shown": pick.len(),
        "groups": GROUPS,
        "chem": chem,
        "nodes": nodes,
        "edges": edges,
        "notes": notes,
        "rows": rows,
        "rooms": rooms,
        "say": SAY,
        "moods": MOOD,
    }))
}

/// The mean chemistry of every pair in a set of men, 0-100: None for
/// fewer than two, or in another league.
fn mean_chemistry(w: &World, set: &[usize]) -> Value {
    let mut sum = 0.0;
    let mut c = 0;
    for x in 0..set.len() {
        for y in x + 1..set.len() {
            let Some(v) = pair_chemistry(w, set[x], set[y]) else { return Value::Null };
            sum += v as f64;
            c += 1;
        }
    }
    if c == 0 { Value::Null } else { json!((100.0 * sum / c as f64).round()) }
}

/// A tie's kind on the map and its words.
fn tie_words(kind: Kind, strength: u8, since: i16) -> (&'static str, String) {
    match kind {
        Kind::Family => ("c", "family".into()),
        Kind::Mentor => ("m", format!("teaching him since {since}")),
        Kind::Friend if strength >= 70 => ("c", format!("very close since {since}")),
        Kind::Friend if strength >= 45 => ("c", format!("close since {since}")),
        Kind::Friend => ("f", format!("friends since {since}")),
    }
}

/// His close friends on the team.
fn friends_here(w: &World, id: u32, on: &HashMap<u32, usize>) -> usize {
    ties::friends(w, id).iter().filter(|f| on.contains_key(f)).count()
}

/// The starters at each position: the game team's depth chart, else who
/// started last season.
fn starters_of(w: &World, roster: &[usize], on: &HashMap<u32, usize>) -> Vec<(Position, Vec<usize>)> {
    use afm_engine::game::roster::starters;
    if let Some(depth) = w.seat_game_depth() {
        return depth
            .into_iter()
            .map(|(pos, ids)| (pos, ids.iter().filter_map(|id| on.get(id).copied()).take(starters(pos)).collect()))
            .collect();
    }
    Position::ALL
        .iter()
        .map(|&pos| {
            let v: Vec<usize> = roster
                .iter()
                .copied()
                .filter(|&i| w.people[i].player.position == pos && w.people[i].last_role == Some(PlayingTime::Starter))
                .take(starters(pos))
                .collect();
            (pos, v)
        })
        .collect()
}

/// This season's snaps from your kept games: each man's on his side of the
/// ball, and the team's on each side (offense, defense, kicking).
struct Snaps {
    his: HashMap<u32, u32>,
    team: [u32; 3],
}

fn side(p: Position) -> usize {
    match group(p) {
        0 | 1 => 0,
        2 => 1,
        _ => 2,
    }
}

fn snap_shares(w: &World) -> Snaps {
    let mut s = Snaps { his: HashMap::new(), team: [0; 3] };
    let Some((level, t)) = w.seat_team() else { return s };
    let now = super::season_now(w, level);
    for g in w.seat_games().iter().filter(|g| g.level == level && super::season_of(g.level, g.date) == now) {
        let Some(us) = g.teams.iter().position(|&x| x == t) else { continue };
        let lines = &g.stats.players[us];
        let most = [
            lines.values().map(|l| l.snaps_off).max().unwrap_or(0),
            lines.values().map(|l| l.snaps_def).max().unwrap_or(0),
            lines.values().map(|l| l.snaps_st).max().unwrap_or(0),
        ];
        for (t, m) in s.team.iter_mut().zip(most) {
            *t += m;
        }
        for (&id, l) in lines {
            let Some(i) = views::person_index(w, id) else { continue };
            let n = [l.snaps_off, l.snaps_def, l.snaps_st][side(w.people[i].player.position)];
            *s.his.entry(id).or_default() += n;
        }
    }
    s
}

/// The snaps cell: this season's share, else his role last year.
fn snaps_cell(w: &World, i: usize, s: &Snaps) -> Value {
    let t = snaps_text(w, i, s);
    match t.strip_suffix(" last year") {
        Some(role) => json!({ "k": "Snaps", "v": cap(role), "sub": "last year" }),
        None if t.ends_with('%') => json!({ "k": "Snaps", "v": t, "sub": "of the team's" }),
        None => json!({ "k": "Snaps", "v": "None yet" }),
    }
}

fn snaps_text(w: &World, i: usize, s: &Snaps) -> String {
    let p = &w.people[i];
    let k = side(p.player.position);
    if s.team[k] == 0 {
        return match p.last_role {
            Some(PlayingTime::Starter) => "started last year".into(),
            Some(PlayingTime::Rotation) => "rotation last year".into(),
            Some(PlayingTime::Backup) => "backup last year".into(),
            _ => "no snaps yet".into(),
        };
    }
    let his = s.his.get(&p.player.id).copied().unwrap_or(0);
    format!("{}%", (100.0 * his as f64 / s.team[k] as f64).round().min(100.0))
}

/// One man on the map, with his card.
#[allow(clippy::too_many_arguments)]
fn node(
    w: &World,
    i: usize,
    on: &HashMap<u32, usize>,
    captains: &HashSet<usize>,
    starter: &HashSet<usize>,
    say: &HashMap<usize, u8>,
    friction: &[Friction],
    snaps: &Snaps,
) -> Value {
    let p = &w.people[i];
    let id = p.player.id;
    let m = mood_step(p.mood);
    let reasons = talks::reasons(w, id);
    // The trend: what moved him these last two weeks.
    let recent: Vec<&(Date, String, f32)> = reasons.iter().filter(|x| x.0.days_to(w.today) <= 14).collect();
    let net: f32 = recent.iter().map(|x| x.2).sum();
    let trend = if net > 0.05 {
        recent.iter().find(|x| x.2 > 0.0).map_or("rising".into(), |x| format!("rising since {}", short_date(x.0)))
    } else if net < -0.05 {
        recent.iter().find(|x| x.2 < 0.0).map_or("falling".into(), |x| format!("falling since {}", short_date(x.0)))
    } else {
        "steady".to_string()
    };
    // Why: the dated reasons first, then what anyone can see.
    let mut why: Vec<(bool, String)> = reasons.iter().take(3).map(|x| (x.2 > 0.0, cap(&x.1))).collect();
    if captains.contains(&i) {
        why.push((true, "A captain: the room listens to him".into()));
    }
    if starter.contains(&i) {
        why.push((true, format!("Starting at {}", p.player.position.name())));
    }
    if let Some(mt) = ties::mentor_of(w, id).filter(|&m| m != id).and_then(|x| on.get(&x)) {
        why.push((true, format!("{} is teaching him", w.people[*mt].name)));
    }
    let friends = friends_here(w, id, on);
    if friends >= 2 {
        why.push((true, format!("{friends} close friends on the team")));
    }
    if rules::sitting(w, id, w.today) {
        why.push((false, "Sitting out under your team rules".into()));
    }
    if p.mood <= -0.15 {
        let c = talks::concern(w, i).1;
        if !c.starts_with("nothing much") {
            why.push((false, cap(&c)));
        }
    }
    for f in friction.iter().filter(|f| f.rival == i) {
        why.push((false, cap(&f.wants(&w.people[f.starter].name))));
    }
    let mut seen = HashSet::new();
    why.retain(|x| seen.insert(x.1.clone()));
    why.truncate(5);
    // His links: friction, mentors, close, friends.
    let mut links: Vec<(u8, Value)> = Vec::new();
    for t in w.folk.ties.of_id(id).filter(|t| t.together && t.a != t.b) {
        let other = if t.a == id { t.b } else { t.a };
        let Some(&j) = on.get(&other) else { continue };
        let (k, words) = tie_words(t.kind, t.strength, t.since);
        let (rank, kind) = match (t.kind, k) {
            (Kind::Mentor, _) if t.a == id => (1, "mentoring"),
            (Kind::Mentor, _) => (1, "mentor"),
            (Kind::Family, _) => (2, "family"),
            (_, "c") => (2, "close"),
            _ => (3, "friends"),
        };
        let words = if kind == "mentoring" { words.replace("teaching him", "teaching") } else { words };
        let l = (rank, json!({ "id": other, "name": w.people[j].name, "why": words, "k": kind }));
        if !links.contains(&l) {
            links.push(l);
        }
    }
    for f in friction.iter().filter(|f| f.rival == i || f.starter == i) {
        let o = if f.rival == i { f.starter } else { f.rival };
        let words = if f.rival == i {
            f.holder().to_string()
        } else {
            format!("{} {}", surname(&w.people[f.rival].name), f.wants_his())
        };
        links
            .push((0, json!({ "id": w.people[o].player.id, "name": w.people[o].name, "why": words, "k": "friction" })));
    }
    links.sort_by_key(|x| x.0);
    let s = say.get(&i).copied().unwrap_or(1);
    json!({
        "id": id,
        "name": p.name,
        "pos": p.player.position.name(),
        "g": group(p.player.position),
        "cls": views::class_text(w, p),
        "home": views::hometown(w, p),
        "say": s,
        "m": m,
        "mood": MOOD[m as usize - 1],
        "trend": trend,
        "cap": captains.contains(&i),
        "starter": starter.contains(&i),
        "cells": [
            snaps_cell(w, i, snaps),
            { "k": "Friends here", "v": friends.to_string() },
            { "k": "Say", "v": SAY[s as usize - 1] },
        ],
        "why": why.iter().map(|(up, t)| json!({ "s": if *up { "+" } else { "−" }, "t": t })).collect::<Vec<_>>(),
        "links": links.into_iter().take(4).map(|x| x.1).collect::<Vec<_>>(),
    })
}
