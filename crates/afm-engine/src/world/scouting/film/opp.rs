//! Your next opponent's tape in the film room (SK54): the snaps that show
//! the tape's own hints (section 9) and the downs that decide games,
//! drawn as three schematic stills from what the kept film holds (the
//! look, the play, the coverage, where the ball went), with "what are they
//! running?" as the call. What you catch becomes the game plan's hints.
//! The kept film has no tracking, so the stills are a coach's diagram of
//! the snap, not his feet.

use super::tape::{Call, Dot, Game, Line, Role, Snap, Stat, Still};
use crate::world::season::key_uniform;
use crate::world::tape::{Snap as Kept, Tape};

const FAMILIES: [&str; 6] = ["Zone run", "Gap run", "Play-action shot", "Screen", "Quick game", "Dropback, downfield"];
const COVERS: [&str; 6] = ["Cover 0", "Cover 1", "Cover 2", "Cover 3", "Quarters", "Cover 6"];

fn scrimmage(s: &Kept) -> bool {
    s.kind <= 1 && !s.null
}

fn family(s: &Kept) -> &'static str {
    let p = s.play.to_lowercase();
    if s.kind == 0 {
        if p.contains("zone") || p.contains("stretch") { "Zone run" } else { "Gap run" }
    } else if s.play_action {
        "Play-action shot"
    } else if p.contains("screen") || s.throw.is_some_and(|t| t.air <= 0) {
        "Screen"
    } else if s.throw.is_some_and(|t| t.air <= 7) || p.contains("slant") || p.contains("quick") {
        "Quick game"
    } else {
        "Dropback, downfield"
    }
}

fn cover(s: &Kept) -> &'static str {
    let c = s.coverage.to_lowercase();
    if c.contains('0') {
        "Cover 0"
    } else if c.contains('1') || s.man {
        "Cover 1"
    } else if c.contains("quarter") || c.contains('4') {
        "Quarters"
    } else if c.contains('6') {
        "Cover 6"
    } else if c.contains('2') {
        "Cover 2"
    } else {
        "Cover 3"
    }
}

fn options(truth: &'static str, all: &[&'static str], salt: u64) -> (Vec<String>, u8) {
    let mut others: Vec<&str> = all.iter().copied().filter(|x| *x != truth).collect();
    others.sort_by_key(|x| key_uniform(salt, x.len() as u64 ^ (x.as_bytes()[0] as u64 * 131)).to_bits());
    let mut v = vec![truth, others[0], others[1]];
    let at = (key_uniform(salt ^ 0x0FF5, 7) * 3.0) as usize % 3;
    v.swap(0, at);
    (v.into_iter().map(String::from).collect(), at as u8)
}

fn ordinal(d: u8) -> &'static str {
    match d {
        1 => "1st",
        2 => "2nd",
        3 => "3rd",
        _ => "4th",
    }
}

fn clock(s: u16) -> String {
    format!("{}:{:02}", s / 60, s % 60)
}

/// Their offense's look as dots: the line, the backs, the ends, the
/// receivers by the set; the defense in its shell.
fn formation(s: &Kept, los: u8, on_off: bool) -> Vec<Dot> {
    let look = s.look.to_lowercase();
    let pers: Vec<u8> = look.chars().take(2).filter_map(|c| c.to_digit(10).map(|d| d as u8)).collect();
    let (rb, te) = (pers.first().copied().unwrap_or(1).min(3), pers.get(1).copied().unwrap_or(1).min(3));
    let wr = 5u8.saturating_sub(rb + te);
    let o = if on_off { 1 } else { 0 };
    let d = 1 - o;
    let mut v = Vec::new();
    for (k, x) in [40u8, 45, 50, 55, 60].iter().enumerate() {
        v.push(Dot { x: *x, y: los + 2, k: o, l: if k == 2 { "C".into() } else { String::new() } });
    }
    let qb_y = if look.contains("gun") {
        los + 12
    } else if look.contains("pistol") {
        los + 9
    } else {
        los + 4
    };
    v.push(Dot { x: 50, y: qb_y.min(98), k: o, l: "QB".into() });
    let empty = look.contains("empty");
    for k in 0..rb {
        if empty {
            v.push(Dot { x: 30 + 6 * k, y: los + 2, k: o, l: "RB".into() });
        } else {
            v.push(Dot { x: 50 + if k == 0 { 6 } else { 0 }, y: (qb_y + 4 + 3 * k).min(98), k: o, l: "RB".into() });
        }
    }
    for k in 0..te {
        v.push(Dot { x: if k == 0 { 66 } else { 34 }, y: los + 2, k: o, l: "TE".into() });
    }
    let right: u8 = if look.contains("trips") || look.contains("bunch") { wr.min(3) } else { wr.div_ceil(2) };
    let xs_r = [92u8, 82, 74];
    let xs_l = [8u8, 18, 26];
    for k in 0..wr {
        let (x, y) = if k < right {
            (if look.contains("bunch") { 74 + 3 * k } else { xs_r[k as usize % 3] }, los + if k == 0 { 2 } else { 4 })
        } else {
            (xs_l[(k - right) as usize % 3], los + if k == right { 2 } else { 4 })
        };
        v.push(Dot { x, y, k: o, l: "WR".into() });
    }
    // The defense: the front, the backers, the corners and the shell.
    for x in [38u8, 46, 54, 62] {
        v.push(Dot { x, y: los.saturating_sub(3), k: d, l: String::new() });
    }
    for x in [40u8, 50, 60] {
        v.push(Dot { x, y: los.saturating_sub(12), k: d, l: String::new() });
    }
    for x in [9u8, 91] {
        v.push(Dot { x, y: los.saturating_sub(if s.man { 4 } else { 10 }), k: d, l: "CB".into() });
    }
    match cover(s) {
        "Cover 0" => {
            v.push(Dot { x: 30, y: los.saturating_sub(8), k: d, l: "S".into() });
            v.push(Dot { x: 70, y: los.saturating_sub(8), k: d, l: "S".into() });
        }
        "Cover 1" | "Cover 3" => {
            v.push(Dot { x: 50, y: los.saturating_sub(34), k: d, l: "S".into() });
            v.push(Dot { x: 66, y: los.saturating_sub(14), k: d, l: "S".into() });
        }
        _ => {
            v.push(Dot { x: 32, y: los.saturating_sub(30), k: d, l: "S".into() });
            v.push(Dot { x: 68, y: los.saturating_sub(30), k: d, l: "S".into() });
        }
    }
    v
}

/// The three stills of a kept snap.
fn stills(s: &Kept, on_off: bool) -> Vec<Still> {
    let los = 62u8;
    let dots = formation(s, los, on_off);
    let p = s.play.to_lowercase();
    let side_x: i32 = if p.contains("left") {
        -1
    } else if p.contains("right") {
        1
    } else {
        0
    };
    let up = |yd: i32| (los as i32 - yd * 2).clamp(2, 98) as u8;
    let snap_cap = format!("{}: {}", if on_off { "Their offense" } else { "Their defense" }, s.look);
    let mut moment = Still { caption: String::new(), dots: dots.clone(), lines: Vec::new(), ring: None, los };
    let mut end = Still { caption: String::new(), dots: dots.clone(), lines: Vec::new(), ring: None, los };
    let qb = dots.iter().find(|d| d.l == "QB").map_or((50, los + 12), |d| (d.x, d.y));
    if s.kind == 0 {
        let rb = dots.iter().find(|d| d.l == "RB").map_or(qb, |d| (d.x, d.y));
        let gap = (50 + side_x * 8).clamp(5, 95) as u8;
        moment.caption = format!("{}: the hole", s.play);
        moment.lines.push(Line { k: 2, pts: vec![rb, (gap, los + 1), (gap, up(2))] });
        moment.ring = Some((gap, los));
        let fin = (50 + side_x * 12).clamp(5, 95) as u8;
        end.caption = format!("{} for {}", s.play, s.yards);
        end.lines.push(Line { k: 2, pts: vec![rb, (gap, los + 1), (fin, up(s.yards as i32))] });
        end.ring = Some((fin, up(s.yards as i32)));
    } else {
        let (to_x, air) =
            s.throw.map_or((50u8, 8i32), |t| ((100.0 - t.to as f32 / 1.066).clamp(3.0, 97.0) as u8, t.air as i32));
        let drop = (qb.0, (qb.1 + if s.play_action { 6 } else { 3 }).min(98));
        moment.caption = if s.play_action {
            "The fake, and the shot".to_string()
        } else if s.blitz {
            format!("{}: the pressure comes", s.pressure)
        } else {
            format!("{} against {}", s.play, s.coverage)
        };
        moment.lines.push(Line { k: 0, pts: vec![qb, drop] });
        moment.lines.push(Line { k: 3, pts: vec![(to_x, los + 2), (to_x, up(air))] });
        moment.ring = Some(drop);
        end.caption = if s.sack {
            "Sacked".to_string()
        } else if s.complete {
            format!("Caught for {}", s.yards)
        } else if s.turnover {
            "Intercepted".to_string()
        } else {
            "Incomplete".to_string()
        };
        end.lines.push(Line { k: 2, pts: vec![drop, (to_x, up(air))] });
        if s.complete {
            end.lines.push(Line { k: 2, pts: vec![(to_x, up(air)), (to_x, up(s.yards as i32))] });
        }
        end.ring = Some((to_x, up(if s.complete { s.yards as i32 } else { air })));
    }
    vec![Still { caption: snap_cap, dots, lines: Vec::new(), ring: None, los }, moment, end]
}

/// A cut-up of one kept tape on the team at `side`: the snaps behind each
/// hint (two a hint), then their third downs and red zone, six to eight
/// in all. The hints come back pointing at the cut-up's snaps.
pub fn team_cut(
    t: &Tape,
    side: u8,
    hints: &[(String, String, Vec<usize>)],
    salt: u64,
) -> (Vec<Game>, Vec<Snap>, super::session::Hints) {
    let mut picks: Vec<usize> = Vec::new();
    for (_, _, ks) in hints {
        for &k in ks.iter().filter(|&&k| k < t.snaps.len() && scrimmage(&t.snaps[k])).take(2) {
            if !picks.contains(&k) && picks.len() < 8 {
                picks.push(k);
            }
        }
    }
    let key: Vec<usize> = (0..t.snaps.len())
        .filter(|&k| {
            let s = &t.snaps[k];
            scrimmage(s) && s.off == side && (s.down == 3 || s.yl <= 20)
        })
        .collect();
    for k in key {
        if picks.len() >= 6 {
            break;
        }
        if !picks.contains(&k) {
            picks.push(k);
        }
    }
    picks.sort_unstable();
    let other = t.names[1 - side as usize].clone();
    let games =
        vec![Game { label: format!("{} vs {other}, {}", t.names[side as usize], t.date.pretty()), strength: 0.0 }];
    let mut snaps = Vec::new();
    for (n, &k) in picks.iter().enumerate() {
        let s = &t.snaps[k];
        let on_off = s.off == side;
        let sit = format!("Q{} {} · {} & {} at the {}", s.q, clock(s.clock), ordinal(s.down), s.togo, s.yl);
        let job =
            if on_off { format!("Their offense: {}", s.look) } else { format!("Their defense against {}", s.look) };
        let result = if s.kind == 0 {
            format!("{} for {}", s.play, s.yards)
        } else if s.sack {
            format!("Sacked for {}", s.yards)
        } else if s.complete {
            format!("{} complete for {}", s.play, s.yards)
        } else {
            format!("{} incomplete", s.play)
        };
        let ks = salt ^ (k as u64) << 16;
        let call = if on_off {
            let truth = family(s);
            let (opts, right) = options(truth, &FAMILIES, ks);
            Call { q: "What are they running?".into(), opts, right, truth: format!("{truth}: {}", s.play) }
        } else {
            let truth = cover(s);
            let (opts, right) = options(truth, &COVERS, ks);
            Call {
                q: "What's their coverage?".into(),
                opts,
                right,
                truth: format!("{truth}{}", if s.blitz { ", with a blitz" } else { "" }),
            }
        };
        snaps.push(Snap {
            game: 0,
            sit: sit.clone(),
            job: job.clone(),
            result: result.clone(),
            chart: format!("{sit}: {job}, {result}"),
            stills: stills(s, on_off),
            role: Role::Away,
            grade: 0,
            cues: Vec::new(),
            call: Some(call),
            after: None,
            surprise: None,
            speed: 0.0,
            third: s.down == 3,
            red: s.yl <= 20,
            pressure: s.blitz,
            special: false,
            stat: Stat::default(),
            others: Vec::new(),
        });
        let _ = n;
    }
    let mapped = hints
        .iter()
        .map(|(key, text, ks)| {
            let at: Vec<u8> = ks.iter().filter_map(|k| picks.iter().position(|p| p == k).map(|x| x as u8)).collect();
            (key.clone(), text.clone(), at)
        })
        .filter(|h| !h.2.is_empty())
        .collect();
    (games, snaps, mapped)
}
