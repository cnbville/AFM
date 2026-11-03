//! Speed and captains on a world file (minor bug 184): each spot's
//! expected forty and weight by level, the slowest FBS linebackers, and
//! who the FBS rooms voted captain (seasons, role, snaps, tackles). With a
//! program name, its captains and linebackers; `AFM_SHOW_BACKUPS=1` lists
//! every backup captain.
//! Usage: room_check <world file> [program name]

use afm_engine::attributes::Attr;
use afm_engine::body::{Drills, body_now};
use afm_engine::player::PlayingTime;
use afm_engine::positions::Position;
use afm_engine::world::data::Division;
use afm_engine::world::records::Stat;
use afm_engine::world::{Stage, World};
use std::path::Path;

fn pct(v: &mut [f64], q: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    if v.is_empty() { 0.0 } else { v[((v.len() - 1) as f64 * q).round() as usize] }
}

fn line(label: &str, rows: &[(f64, f64, f64, f64)]) {
    if rows.is_empty() {
        return;
    }
    let n = rows.len() as f64;
    let mean = |k: usize| rows.iter().map(|r| [r.0, r.1, r.2, r.3][k]).sum::<f64>() / n;
    let mut f: Vec<f64> = rows.iter().map(|r| r.3).collect();
    let slow = f.iter().filter(|&&x| x >= 5.0).count() as f64 / n;
    let slow2 = f.iter().filter(|&&x| x >= 5.15).count() as f64 / n;
    println!(
        "{label:<22} n {:>6}  lb {:>5.1}  spd {:>4.1}  acc {:>4.1}  forty {:.2} (p10 {:.2} p50 {:.2} p90 {:.2})  5.00+ {:>4.1}%  5.15+ {:>4.1}%",
        rows.len(),
        mean(0),
        mean(1),
        mean(2),
        mean(3),
        pct(&mut f, 0.1),
        pct(&mut f, 0.5),
        pct(&mut f, 0.9),
        100.0 * slow,
        100.0 * slow2
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let world: World = afm_engine::save::read_world(Path::new(&args[1])).expect("read the world");
    let w = &world;
    println!("world {:?}, season {}", w.today, w.year);
    let row = |i: usize| {
        let p = &w.people[i].player;
        let b = body_now(p);
        (b.weight, p.attributes.get(Attr::Speed), p.attributes.get(Attr::Acceleration), Drills::of(p).expected().forty)
    };
    let div = |i: usize| w.people[i].college.map(|k| w.programs[k].division);
    for pos in [Position::LB, Position::EDGE, Position::S, Position::RB, Position::TE] {
        let pick = |f: &dyn Fn(usize) -> bool| -> Vec<(f64, f64, f64, f64)> {
            (0..w.people.len()).filter(|&i| w.people[i].player.position == pos && f(i)).map(row).collect()
        };
        let hs = |age: u32| move |i: usize| w.people[i].stage == Stage::HighSchool && w.people[i].player.age == age;
        line(&format!("{} HS 17", pos.name()), &pick(&hs(17)));
        line(&format!("{} HS 18", pos.name()), &pick(&hs(18)));
        let col = |d: Division, role: Option<PlayingTime>| {
            move |i: usize| {
                w.people[i].stage == Stage::College
                    && div(i) == Some(d)
                    && role.is_none_or(|r| w.people[i].last_role == Some(r))
            }
        };
        line(&format!("{} FBS all", pos.name()), &pick(&col(Division::FBS, None)));
        line(&format!("{} FBS starters", pos.name()), &pick(&col(Division::FBS, Some(PlayingTime::Starter))));
        line(&format!("{} FBS backups", pos.name()), &pick(&col(Division::FBS, Some(PlayingTime::Backup))));
        line(&format!("{} FCS all", pos.name()), &pick(&col(Division::FCS, None)));
        line(
            &format!("{} NFL", pos.name()),
            &pick(&|i| w.people[i].stage == Stage::Pro && w.people[i].pro.as_ref().is_some_and(|c| c.team.is_some())),
        );
    }

    // The slowest FBS linebackers: who they are.
    let mut slow: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.player.position == Position::LB
                && p.stage == Stage::College
                && div(i) == Some(Division::FBS)
                && row(i).3 >= 5.08
        })
        .collect();
    slow.sort_by(|&a, &b| row(b).3.total_cmp(&row(a).3));
    for &i in slow.iter().take(25) {
        let p = &w.people[i];
        let r = row(i);
        println!(
            "  slow LB {} ({}) age {} stars {} ovr {:.1} role {:?} | {:.0} lb spd {:.1}/{:.1} acc {:.1}/{:.1} forty {:.2} | pot {:.0} (was {:.0}) frame {:.0} lb, positions {:?}",
            p.name,
            w.programs[p.college.unwrap()].real_name,
            p.player.age,
            p.stars,
            p.player.overall(),
            p.last_role,
            r.0,
            r.1,
            afm_engine::debug::ceilings(&p.player).get(Attr::Speed),
            r.2,
            afm_engine::debug::ceilings(&p.player).get(Attr::Acceleration),
            r.3,
            afm_engine::debug::potential(&p.player),
            afm_engine::debug::original_potential(&p.player),
            p.player.body.frame_weight,
            p.player.position_history.iter().map(|x| (x.0, x.1.name())).collect::<Vec<_>>(),
        );
    }

    // Captains at FBS programs.
    let tackles = |i: usize| -> (f64, u32) {
        let id = w.people[i].player.id;
        let c = w.stats.career(id);
        (c.iter().map(|r| r.get(Stat::Tackles)).sum(), c.iter().map(|r| r.games as u32).sum())
    };
    let (mut n, mut first, mut seniors, mut starters, mut backups, mut low_snaps, mut quiet) = (0, 0, 0, 0, 0, 0, 0);
    let mut by_seasons = [0u32; 7];
    for pr in w.programs.iter().filter(|p| p.division == Division::FBS) {
        for &i in &pr.captains {
            let p = &w.people[i];
            n += 1;
            by_seasons[(p.college_seasons as usize).min(6)] += 1;
            if p.college_seasons == 0 {
                first += 1;
            }
            if p.college_seasons >= 3 {
                seniors += 1;
            }
            match p.last_role {
                Some(PlayingTime::Starter) => starters += 1,
                Some(PlayingTime::Backup) | Some(PlayingTime::Redshirt) => backups += 1,
                _ => {}
            }
            if matches!(p.last_role, Some(PlayingTime::Backup) | Some(PlayingTime::Redshirt))
                && std::env::var_os("AFM_SHOW_BACKUPS").is_some()
            {
                let strict =
                    pr.head_coach.is_some_and(|c| afm_engine::debug::coach_ability(&w.coaches[c]).discipline >= 14.0);
                let t = &p.player.personality.traits;
                let mates = pr.roster.len();
                println!(
                    "  backup captain {} {} at {} ovr {:.1} seasons {} lead {:.0} prof {:.0} ego {:.0} strict {strict} roster {mates}",
                    p.name,
                    p.player.position.name(),
                    pr.real_name,
                    p.player.overall(),
                    p.college_seasons,
                    t.get(afm_engine::player::Trait::Leadership),
                    t.get(afm_engine::player::Trait::Professionalism),
                    t.get(afm_engine::player::Trait::Ego)
                );
            }
            if p.college_snaps < 300 {
                low_snaps += 1;
            }
            let def =
                matches!(p.player.position, Position::LB | Position::EDGE | Position::DT | Position::CB | Position::S);
            if def && tackles(i).0 < 10.0 && p.college_seasons >= 2 {
                quiet += 1;
            }
        }
    }
    let s = |x: u32| 100.0 * x as f64 / n.max(1) as f64;
    println!(
        "FBS captains {n}: first-years {:.1}%, 3+ seasons {:.1}%, last role starter {:.1}%, backup/redshirt {:.1}%, under 300 career snaps {:.1}%, defenders with 2+ seasons and under 10 tackles {:.1}%",
        s(first),
        s(seniors),
        s(starters),
        s(backups),
        s(low_snaps),
        s(quiet)
    );
    println!("  by college seasons 0-6+: {by_seasons:?}");

    if let Some(name) = args.get(2) {
        for pr in w.programs.iter().filter(|p| p.real_name.contains(name.as_str())) {
            println!("{} captains:", pr.real_name);
            for &i in &pr.captains {
                let p = &w.people[i];
                let t = &p.player.personality.traits;
                let (tk, g) = tackles(i);
                let r = row(i);
                println!(
                    "  {} {} ovr {:.1} seasons {} role {:?} snaps {} games {} tackles {:.0} lead {:.0} ego {:.0} | {:.0} lb spd {:.1} acc {:.1} forty {:.2} age {}",
                    p.name,
                    p.player.position.name(),
                    p.player.overall(),
                    p.college_seasons,
                    p.last_role,
                    p.college_snaps,
                    g,
                    tk,
                    t.get(afm_engine::player::Trait::Leadership),
                    t.get(afm_engine::player::Trait::Ego),
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    p.player.age
                );
            }
            // Its linebackers.
            for &i in pr.roster.iter().filter(|&&i| w.people[i].player.position == Position::LB) {
                let p = &w.people[i];
                let r = row(i);
                println!(
                    "  LB {} ovr {:.1} seasons {} role {:?} | {:.0} lb spd {:.1} acc {:.1} forty {:.2} age {} grow {:.2}",
                    p.name,
                    p.player.overall(),
                    p.college_seasons,
                    p.last_role,
                    r.0,
                    r.1,
                    r.2,
                    r.3,
                    p.player.age,
                    p.player.grow as f64
                );
            }
        }
    }
}
