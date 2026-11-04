//! The Data section's running tallies (lane NB1) only watch: a world
//! counting them plays exactly as one that doesn't, every person on every
//! day, and each season's five files read back and add up to the stat book.

use crate::world::records::Stat;
use crate::world::tally::{self, Level, file, players};
use crate::world::wlog::fp;
use crate::world::{SPIN_UP_YEARS, World, WorldOptions};

fn prints(w: &World) -> (Vec<u64>, [u64; 7]) {
    (
        w.people.iter().map(fp::of).collect(),
        [
            fp::of(&w.rng),
            fp::of(&w.coaches),
            fp::of(&w.programs),
            fp::of(&w.clubs),
            fp::of(&w.alumni),
            fp::of(&w.news),
            book(w),
        ],
    )
}

/// The stat book's print. Its norms are saved as a list in the map's walk
/// order (`save::big_values`), which differs between two copies of one
/// world, so they are printed as a map here.
fn book(w: &World) -> u64 {
    let s = &w.stats;
    let norms: std::collections::HashMap<_, Vec<f32>> = s.norms.iter().map(|(k, v)| (*k, v.to_vec())).collect();
    fp::of(&(
        (&s.seasons, &s.by_player, &s.year_index, norms, &s.books, &s.broken, &s.honors),
        (&s.hall, &s.buzz, &s.streaks, &s.streak_from, &s.ended, &s.win_streak, &s.hs_streak),
    ))
}

#[test]
fn the_tallies_only_watch_and_add_up() {
    let opts = WorldOptions { scale: 0.002, history_years: 2, ..Default::default() };
    let dir = std::env::temp_dir().join(format!("afm-tallies-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut w = World::empty(opts.clone());
    for age in [17, 16, 15] {
        w.add_class_aged(age);
    }
    for _ in 0..SPIN_UP_YEARS {
        w.advance_year();
    }
    w.start_recording();
    let file_path = dir.with_extension("afmworld");
    crate::save::write_world(&w, &file_path, "test", "", None).unwrap();
    drop(w);
    let open = || {
        let mut w = crate::save::read_world(&file_path).unwrap();
        w.after_load();
        w
    };
    let (mut a, mut b) = (open(), open());
    let _ = std::fs::remove_file(&file_path);
    a.set_tallies_off(true);
    b.set_tallies_off(false);
    b.attach_tallies(&dir).unwrap();
    let end = a.year + opts.history_years as i32;
    let mut days = 0;
    while a.year < end {
        a.advance_day();
        b.advance_day();
        days += 1;
        let (pa, sa) = prints(&a);
        let (pb, sb) = prints(&b);
        if let Some(k) = (0..pa.len().max(pb.len())).find(|&k| pa.get(k) != pb.get(k)) {
            panic!("{}: person {k} ({}) differs with the tallies on", a.today, a.people[k].name);
        }
        assert_eq!(sa, sb, "{}: a store differs with the tallies on", a.today);
    }
    assert!(days > 700);
    assert!(a.tally_seasons().is_empty(), "the world with tallies off counted");
    let seasons = b.tally_seasons();
    let closed: Vec<i32> = seasons.iter().copied().filter(|&s| s < b.year).collect();
    assert_eq!(closed.len(), opts.history_years as usize, "{seasons:?}");
    // Each kind's file for each closed season, with records.
    for &s in &closed {
        for kind in tally::KINDS {
            let p = tally::path(&dir, kind, s);
            let mut f = file::open(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert_eq!(f.header.kind, kind);
            let mut n = 0;
            f.each(|_, _| n += 1).unwrap();
            assert!(n > 0, "no {kind} records in {s}");
            assert_eq!(n as u64, f.header.records);
        }
    }
    let s = closed[closed.len() - 1];
    // Players: a season's down splits add up to the stat book's season.
    let mut checked = 0;
    let mut by_id = std::collections::HashMap::new();
    b.tally_players(s, |p| {
        by_id.insert((p.level, p.id), p.clone());
    });
    for r in b.stats.year(s) {
        let Some(p) = by_id.get(&(Level::College as u8, r.id)) else { continue };
        if r.program as u32 != p.team {
            continue;
        }
        let downs = |v: &[(u8, [f32; 6])], k: usize| v.iter().filter(|x| x.0 < 4).map(|x| x.1[k] as f64).sum::<f64>();
        let rush = downs(&p.splits.rush, 0);
        if r.get(Stat::Rush) >= 20.0 {
            assert_eq!(rush, r.get(Stat::Rush), "player {} carries by down", r.id);
            assert!((downs(&p.splits.rush, 1) - r.get(Stat::RushYds)).abs() < 1.0, "player {} rush yards", r.id);
            checked += 1;
        }
        let tg = downs(&p.splits.recv, 0);
        if r.get(Stat::Targets) >= 10.0 {
            assert_eq!(tg, r.get(Stat::Targets), "player {} targets by down", r.id);
        }
        // His game lines in the games file add up to his season.
        if r.get(Stat::Rush) >= 20.0 && checked <= 3 {
            let mut yds = 0.0;
            let mut games = 0;
            b.tally_games(s, Some(&p.played), |_, g| {
                for l in g.lines.iter().filter(|l| l.id == r.id) {
                    yds += l.get(Stat::RushYds);
                    games += 1;
                }
            });
            assert_eq!(games, p.played.len(), "player {} games", r.id);
            assert!(
                (yds - r.get(Stat::RushYds)).abs() < 1.0,
                "player {} lines {yds} against {}",
                r.id,
                r.get(Stat::RushYds)
            );
        }
    }
    assert!(checked >= 5, "only {checked} runners checked");
    let qbs = by_id.values().filter(|p| !p.splits.pass.is_empty()).count();
    let nfl = by_id.keys().filter(|k| k.0 == Level::Nfl as u8).count();
    assert!(qbs > 10 && nfl > 500, "{qbs} passers, {nfl} NFL players");
    assert!(by_id.values().any(|p| p.starts > 0 && p.starts <= p.games));
    // Games: every level, with boxes; college and NFL with lines and the
    // win chance chart.
    let mut levels = [0usize; 3];
    let mut deep = 0;
    b.tally_games(s, None, |_, g| {
        levels[g.level as usize] += 1;
        assert!(g.boxes[0].plays + g.boxes[1].plays > 60, "a game with {} plays", g.boxes[0].plays + g.boxes[1].plays);
        if g.level != Level::Hs as u8 {
            assert!(!g.lines.is_empty() && g.wp.len() > 60);
            deep += g.adv[0].drives as usize;
        } else {
            assert!(g.lines.is_empty() && g.wp.is_empty());
        }
    });
    assert!(levels.iter().all(|&n| n > 0), "games by level {levels:?}");
    assert!(deep > 0);
    // Teams: both sides' keys, and each team's run and pass snaps match its
    // games' boxes.
    let mut teams = 0;
    b.tally_teams(s, |t| {
        teams += 1;
        let snaps = t.get("o|kind|run")[0] + t.get("o|kind|pass")[0];
        assert!(snaps > 0.0 && t.get("d|kind|pass")[0] > 0.0, "team {} {}", t.level, t.team);
        assert!(
            t.cells.keys().any(|k| k.starts_with("o|personnel|")) && t.cells.keys().any(|k| k.starts_with("d|box|"))
        );
        assert!(t.games > 0 && t.wins + t.losses <= t.games);
    });
    assert!(teams > 30, "{teams} teams");
    // People: coordinators and head coaches with their ranks.
    let mut units = 0;
    b.tally_units(s, |u| {
        units += 1;
        assert_eq!(u.vals.len(), tally::people::STATS.len());
        assert!(u.ranks.iter().all(|r| r.0 >= 1 && r.0 <= r.1));
    });
    assert!(units > 30, "{units} unit rows");
    // Classes: the drafts and the Division I signees, with outcomes counted.
    let c = b.tally_classes();
    // A small world drafts few of its own (the rest of the NFL is the
    // background pool), so each pick is checked against his career: his
    // slot, and his NFL games, which add up to the players file's seasons.
    assert!(!c.picks.is_empty(), "no picks");
    let games_in = |level: Level, id: u32, from: i32| -> u16 {
        closed.iter().filter(|&&y| y >= from).filter_map(|&y| b.tally_player(level, y, id)).map(|r| r.games).sum()
    };
    let mut slots = 0;
    for x in &c.picks {
        if let Some(d) = b.people.iter().find(|p| p.player.id == x.id).and_then(|p| p.pro.as_ref()?.draft) {
            assert_eq!(d, (x.year, x.round as u32, x.pick as u32, x.club as usize), "pick {}", x.id);
            slots += 1;
        }
        assert_eq!(x.out.nfl.games, games_in(Level::Nfl, x.id, x.year), "pick {} NFL games", x.id);
    }
    assert!(slots > 0, "no pick found in the world");
    assert!(c.signees.iter().any(|x| x.out.college.games > 0), "no signee's games counted");
    for x in &c.signees {
        assert_eq!(x.out.college.games, games_in(Level::College, x.id, x.class), "signee {} games", x.id);
    }
    // A save carries the season being counted; a reopened world reads the
    // closed ones from the folder.
    let open_rows = b.tallies.players.rows().len();
    let saved = dir.with_extension("saved.afmworld");
    crate::save::write_world(&b, &saved, "test", "", None).unwrap();
    let mut c = crate::save::read_world(&saved).unwrap();
    c.after_load();
    let _ = std::fs::remove_file(&saved);
    assert_eq!(c.tallies.players.rows().len(), open_rows);
    assert_eq!(c.tally_open_season(), b.tally_open_season());
    c.attach_tallies(&dir).unwrap();
    assert!(c.tally_player(Level::College, s, by_id.keys().find(|k| k.0 == 0).unwrap().1).is_some());
    // A quarterback's scrambles carry their EPA (minor bug 183).
    assert!(b.stats.year(s).iter().any(|r| r.get(Stat::ScrambleEpa) != 0.0));
    let _ = players::SPLITS;
    let _ = std::fs::remove_dir_all(&dir);
}
