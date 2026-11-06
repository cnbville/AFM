//! The world build over two machines (NB13): a world stopped between two
//! history seasons, written to a file and read back, carries on exactly as
//! one that never stopped: every person, every store, every season's
//! tallies. Stopped twice here: at the spin-up's end (before recorded
//! history starts) and after the first recorded season.

use crate::world::wlog::fp;
use crate::world::{SPIN_UP_YEARS, World, WorldOptions};

fn prints(w: &World) -> (Vec<u64>, Vec<(&'static str, u64)>) {
    let s = &w.stats;
    // The stat book's norms are saved as a list in the map's walk order
    // (`save::big_values`), which differs between two copies of one world,
    // so they are printed as a map here.
    let norms: std::collections::HashMap<_, Vec<f32>> = s.norms.iter().map(|(k, v)| (*k, v.to_vec())).collect();
    let book = fp::of(&(
        (&s.seasons, &s.by_player, &s.year_index, norms, &s.books, &s.broken, &s.honors),
        (&s.hall, &s.buzz, &s.streaks, &s.streak_from, &s.ended, &s.win_streak, &s.hs_streak),
    ));
    (
        w.people.iter().map(fp::of).collect(),
        vec![
            ("rng", fp::of(&w.rng)),
            ("coaches", fp::of(&w.coaches)),
            ("programs", fp::of(&w.programs)),
            ("high schools", fp::of(&w.high_schools)),
            ("clubs", fp::of(&w.clubs)),
            ("league", fp::of(&w.nfl)),
            ("alumni", fp::of(&w.alumni)),
            ("news", fp::of(&w.news)),
            ("stat book", book),
            ("nfl book", fp::of(&w.nfl_book)),
            ("market", fp::of(&w.market)),
            ("scouting", fp::of(&w.scouting)),
            ("folk", fp::of(&w.folk)),
            ("finance", fp::of(&w.finance)),
            ("voters", fp::of(&w.voters)),
            ("calendar", fp::of(&w.calendar.events)),
            ("tallies", fp::of(&w.tallies)),
            ("year", w.year as u64),
            ("today", fp::of(&w.today)),
        ],
    )
}

/// Every closed season's tally files in `dir` (a folder a kind, a file a
/// season), by kind and season.
fn tally_files(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut v = Vec::new();
    for kind in std::fs::read_dir(dir).unwrap() {
        let kind = kind.unwrap().path();
        for f in std::fs::read_dir(&kind).unwrap() {
            let f = f.unwrap().path();
            let name = f.strip_prefix(dir).unwrap().to_string_lossy().into_owned();
            v.push((name, std::fs::read(&f).unwrap()));
        }
    }
    v.sort();
    v
}

#[test]
fn a_world_stopped_between_seasons_carries_on_as_one_that_never_stopped() {
    let opts = WorldOptions { scale: 0.004, history_years: 2, ..Default::default() };
    let base = std::env::temp_dir().join(format!("afm-split-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (dir_a, dir_b) = (base.join("whole"), base.join("split"));

    let whole = World::new_recorded(opts.clone(), None, Some(&dir_a), |_, _| true).unwrap();

    // Stopped after the spin-up's last season and after the first recorded
    // one, each time written, dropped and read back.
    let file = base.join("half.afmworld");
    let mut w = World::begin_recorded(opts.clone(), None, Some(&dir_b));
    let mut stops = [SPIN_UP_YEARS, SPIN_UP_YEARS + 1].into_iter().peekable();
    let mut played = 0;
    loop {
        let done = w.play_history(|_, _| {
            played += 1;
            stops.next_if_eq(&played).is_none()
        });
        if done {
            break;
        }
        assert!(w.history_left() > 0);
        crate::save::write_world(&w, &file, "test", "", None).unwrap();
        drop(w);
        w = crate::save::read_world(&file).unwrap();
        w.attach_tallies(&dir_b).unwrap();
    }
    assert_eq!(played, SPIN_UP_YEARS + opts.history_years);
    assert!(stops.next().is_none(), "the build never stopped where asked");

    let (pa, sa) = prints(&whole);
    let (pb, sb) = prints(&w);
    if let Some(k) = (0..pa.len().max(pb.len())).find(|&k| pa.get(k) != pb.get(k)) {
        panic!("person {k} ({}) differs in the build that stopped", whole.people[k].name);
    }
    for ((name, a), (_, b)) in sa.iter().zip(&sb) {
        assert_eq!(a, b, "{name} differs in the build that stopped");
    }
    let (ta, tb) = (tally_files(&dir_a), tally_files(&dir_b));
    assert!(!ta.is_empty());
    assert_eq!(ta.iter().map(|t| &t.0).collect::<Vec<_>>(), tb.iter().map(|t| &t.0).collect::<Vec<_>>());
    for ((name, a), (_, b)) in ta.iter().zip(&tb) {
        assert!(a == b, "tally file {name} differs in the build that stopped");
    }
    let _ = std::fs::remove_dir_all(&base);
}
