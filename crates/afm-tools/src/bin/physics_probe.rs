//! Temporary probe: where a boost changes a team's play.
use afm_engine::attributes::Attr;
use afm_engine::game::roster::depth_chart;
use afm_engine::game::synthetic::synthetic_team;
use afm_engine::game::{GameTeam, OffScheme, quick_game};
use afm_engine::player::Level;
use afm_engine::positions::Position;
use rayon::prelude::*;

fn boost(t: &GameTeam, pos: Position, n: usize, by: f64) -> GameTeam {
    boost_only(t, pos, n, by, &Attr::ALL)
}

fn boost_only(t: &GameTeam, pos: Position, n: usize, by: f64, only: &[Attr]) -> GameTeam {
    let mut t = t.clone();
    for p in t.players.iter_mut().filter(|p| p.position == pos).take(n) {
        for &a in only {
            p.attrs[a] = (p.attrs[a] + by).min(99.0);
        }
    }
    t
}

static TALLY: std::sync::Mutex<std::collections::BTreeMap<String, u32>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

fn summary(a: &GameTeam, b: &GameTeam, games: usize) {
    let d = depth_chart(a, 14.0);
    if std::env::var("AFM_DEPTH").is_ok() { for pos in Position::ALL {
        let ids: Vec<String> = d.list(pos).iter().map(|&i| format!("{}{:?}", a.players[i].id % 1000, a.players[i].position)).collect();
        print!("{:?}:{} ", pos, ids.join(","));
    } }
    println!("\n kr {:?} pr {}", d.kick_returners.iter().map(|&i| a.players[i].id % 1000).collect::<Vec<_>>(), a.players[d.punt_returner].id % 1000);
    let rs: Vec<_> = (0..games).into_par_iter().map(|g| {
        let r = quick_game(a, b, lvl(), 1_000_003 + g as u64);
        let mut v = [0.0f64; 22];
        let t = &r.stats.team[0];
        v[0] = (r.stats.score[0] - r.stats.score[1]) as f64;
        v[1] = t.rush_yards; v[2] = t.carries as f64; v[3] = t.pass_yards; v[4] = t.att as f64;
        v[5] = t.turnovers as f64; v[6] = t.fumbles_lost as f64; v[7] = t.plays as f64; v[8] = t.sacks as f64;
        let u = &r.stats.team[1];
        v[9] = u.points as f64; v[10] = u.plays as f64; v[11] = t.time_of_possession; v[12] = t.penalties as f64;
        v[13] = t.ints as f64; v[14] = u.turnovers as f64; v[15] = t.start_sum / (t.drives as f64).max(1.0);
        v[16] = t.points as f64; v[17] = t.yards; v[18] = t.red_zone_trips as f64; v[19] = t.red_zone_tds as f64; v[20] = t.punts as f64; v[21] = t.drives as f64;
        v
    }).collect();
    let n = rs.len() as f64;
    let m: Vec<f64> = (0..22).map(|k| rs.iter().map(|v| v[k]).sum::<f64>() / n).collect();
    let mut per: std::collections::BTreeMap<u32,[f64;6]> = Default::default();
    for g in 0..games.min(200) {
        let r = quick_game(a, b, lvl(), 1_000_003 + g as u64);
        for (id, l) in &r.stats.players[0] {
            let e = per.entry(*id).or_default();
            e[0] += l.snaps_off as f64; e[1] += l.rush as f64; e[2] += l.rush_yds; e[3] += l.targets as f64; e[4] += l.rec_yds; e[5] += l.snaps_st as f64;
        }
    }
    let gn = games.min(200) as f64;
    let mut bk = [0.0f64; 7];
    let mut ybc = 0.0; let mut nr = 0.0; let mut mt = 0.0;
    for g in 0..games.min(200) {
        let r = quick_game(a, b, lvl(), 1_000_003 + g as u64);
        for p in r.stats.plays.iter().filter(|p| p.offense == 0 && matches!(p.kind, afm_engine::game::PlayType::Run)) {
            let y = p.yards;
            let k = if y <= 0.0 {0} else if y < 4.0 {1} else if y < 10.0 {2} else if y < 20.0 {3} else if y < 40.0 {4} else {5};
            if k == 5 && std::env::var("AFM_LONG").is_ok() { println!("     LONG {:.0} {} vs {} box {} | {}", y, p.call, p.defense_call, p.box_count, p.why); }
            if std::env::var("AFM_STUFF").is_ok() {
                let tk: Vec<String> = p.tacklers.iter().map(|id| b.players.iter().find(|q| q.id == *id).map(|q| format!("{:?}", q.position)).unwrap_or("?".into())).collect();
                let key = format!("{} {}", if k == 0 { "stuff" } else if k == 2 { "4-9" } else if k >= 3 { "10+" } else { "1-3" }, tk.first().cloned().unwrap_or("none".into()));
                *TALLY.lock().unwrap().entry(key).or_insert(0) += 1;
            }
            bk[k] += 1.0; nr += 1.0; ybc += p.before_contact; mt += p.missed_tackles.len() as f64;
        }
    }
    if std::env::var("AFM_STUFF").is_ok() { for (k, v) in TALLY.lock().unwrap().iter() { println!("     {k}: {v}"); } TALLY.lock().unwrap().clear(); }
    println!("   runs <=0 {:.0}% 1-3 {:.0}% 4-9 {:.0}% 10-19 {:.0}% 20-39 {:.0}% 40+ {:.1}% ybc {:.2} missed/run {:.2}", 100.0*bk[0]/nr, 100.0*bk[1]/nr, 100.0*bk[2]/nr, 100.0*bk[3]/nr, 100.0*bk[4]/nr, 100.0*bk[5]/nr, ybc/nr, mt/nr);
    for (id, e) in &per { if e[1] > 0.5*gn || e[3] > gn {
        let p = a.players.iter().find(|p| p.id == *id).unwrap();
        println!("   {:>3}{:?} snaps {:.0} rush {:.1} ypc {:.2} tgt {:.1} recyds {:.1} st {:.1}", id%1000, p.position, e[0]/gn, e[1]/gn, e[2]/e[1].max(1.0), e[3]/gn, e[4]/gn, e[5]/gn); } }
    println!(" margin {:+.1} rush {:.0}/{:.1} ypc {:.2} pass {:.0}/{:.1} TO {:.2} fum {:.2} int {:.2} plays {:.1} sacksTaken? {:.2} oppPts {:.1} oppPlays {:.1} ToP {:.0} pen {:.1} oppTO {:.2} start {:.1}",
        m[0], m[1], m[2], m[1]/m[2], m[3], m[4], m[5], m[6], m[13], m[7], m[8], m[9], m[10], m[11], m[12], m[14], m[15]);
    println!(" pts {:.1} yds {:.0} yds/pt {:.1} rz {:.2} rzTD {:.2} punts {:.2} drives {:.2}", m[16], m[17], m[17]/m[16], m[18], m[19]/m[18], m[20], m[21]);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let a = synthetic_team("A", lvl(), 0.0, OffScheme::WestCoast, seed);
    let b = synthetic_team("B", lvl(), 0.0, OffScheme::WestCoast, seed + 1);
    if std::env::var("AFM_QBSPLIT").is_ok() {
        use Attr::*;
        summary(&a, &b, games);
        for (l, at) in [("accuracy", vec![ThrowAccuracy]), ("power", vec![ThrowPower]), ("mind", vec![Awareness, Vision, PlayRecognition]), ("poise", vec![Poise]), ("athletic", vec![Speed, Acceleration, Agility, Elusiveness, Strength])] {
            println!("== {l}");
            summary(&boost_only(&a, Position::QB, 1, 10.5, &at), &b, games);
        }
        return;
    }
    if std::env::var("AFM_SPLIT").is_ok() {
        use Attr::*;
        summary(&a, &b, games);
        for (l, at) in [("speed", vec![Speed, Acceleration]), ("agil/elus", vec![Agility, Elusiveness]), ("strength", vec![Strength]), ("vision", vec![Vision]), ("carry/catch", vec![Carrying, Catching, RouteRunning])] {
            println!("== {l}");
            summary(&boost_only(&a, Position::RB, 1, 10.5, &at), &b, games);
        }
        return;
    }
    summary(&a, &b, games);
    summary(&boost(&a, Position::RB, 1, 10.5), &b, games);
    summary(&boost(&a, Position::QB, 1, 10.5), &b, games);
}
#[allow(dead_code)]
pub fn one(a: &GameTeam, b: &GameTeam) {
    let r = quick_game(a, b, lvl(), 1_000_003);
    for p in r.stats.plays.iter().filter(|p| p.offense == 0).take(8) {
        println!("{:?} {} carrier {:?} on {:?}", p.kind, p.call, p.carrier.map(|c| c % 1000), p.on_field[0].iter().map(|c| c % 1000).collect::<Vec<_>>());
    }
}
fn lvl() -> Level {
    match std::env::var("AFM_LEVEL").as_deref() {
        Ok("nfl") => Level::NFL,
        Ok("hs") => Level::HS,
        _ => Level::FBS,
    }
}
