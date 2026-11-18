//! Every talk at the very start of a career (Cain 2026-11-17, item 7 of
//! "Give coaches real depth when asked why"): a small world with a season
//! played on the engine, a college job taken on January 1 with no game
//! under you, and every person in your building asked what the game would
//! ask him. Each must know his own situation from the world's records,
//! never only from the games kept since you came: a coach his season review
//! with its numbers (and, hired after it, that it isn't his), your boss
//! last season's record, a starter his own season, a coach his starter's
//! numbers. No card may be left with a placeholder ("in general terms",
//! "he doesn't know"). Builds a small world, so the quick check skips it
//! (full tests run it).

use crate::world::folk::persona::knows::{Knows, Topic};
use crate::world::folk::persona::people::People;
use crate::world::folk::persona::review::{self, HEADER};
use crate::world::folk::persona::{Them, office};
use crate::world::folk::{Team, facts};
use crate::world::seat::{Job, Start};
use crate::world::{World, WorldOptions};

/// One small world on January 1 with a season played on the engine,
/// built once and written to a file; each test reads its own copy.
fn fresh() -> World {
    static FILE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let file = FILE.get_or_init(|| {
        let w = World::new(WorldOptions {
            seed: 233,
            scale: 0.004,
            history_years: 1,
            stop_jan1: true,
            ..Default::default()
        });
        let f = std::env::temp_dir().join(format!("afm-fresh-talks-{}.afmworld", std::process::id()));
        crate::save::write_world(&w, &f, "test", "", None).unwrap();
        f
    });
    let mut w = crate::save::read_world(file).unwrap();
    w.after_load();
    w
}

/// A program whose season is over and on file, taken on January 1.
fn take_job(w: &mut World) -> usize {
    let d0 = w.today;
    let p = (0..w.programs.len())
        .filter(|&p| w.programs[p].division == crate::world::data::Division::FBS)
        .find(|&p| {
            review::season_of(w, Team::College(p)).is_some()
                && !w.college_fixtures(Some(p)).iter().any(|f| f.score.is_none() && !f.cancelled && f.date >= d0)
        })
        .expect("an FBS program done for the year with its season in the tallies");
    w.start_career_as(&super::legacy::you(), Start::Head(Job::College(p)), 5);
    p
}

fn bad(words: &str) -> bool {
    words.contains("general terms") || words == "He doesn't know."
}

#[test]
fn every_talk_at_the_start_of_a_career_knows_last_season() {
    let mut w = fresh();
    let p = take_job(&mut w);
    let team = Team::College(p);
    assert!(w.seat_games.is_empty(), "no game under you yet");
    let (season, row) = review::season_of(&w, team).unwrap();
    let record = format!("{}-{}", row.wins, row.losses);

    // Every coach on your staff, asked about his unit: a season review with
    // numbers; one hired after it says so. His case never claims a season
    // he didn't coach.
    let me = w.career.as_ref().map(|c| c.coach);
    let staff: Vec<usize> = facts::assistants(&w, team).into_iter().map(|x| x.0).filter(|&c| Some(c) != me).collect();
    assert!(!staff.is_empty(), "a staff to talk to");
    let mut reviewed = 0;
    for &c in &staff {
        let known = People.knows(&w, Them::Coach(c), Topic::Football, None);
        assert!(!known.is_empty(), "coach {c}: nothing to say");
        for k in &known {
            assert!(!bad(&k.words), "coach {c}: {:?}", k.words);
        }
        let first = w.coaches[c].job.unwrap().first_season();
        if known[0].words.starts_with(HEADER) {
            reviewed += 1;
            // What he'd say, for the log: the trial's "why".
            eprintln!("coach {c} ({}):", crate::world::folk::facts::coach_role(&w, c));
            for k in &known {
                eprintln!("  {}", k.words);
            }
            assert!(known[0].words.contains(&record), "coach {c}: {:?}", known[0].words);
            assert!(known[0].words.contains(&season.to_string()), "coach {c}: {:?}", known[0].words);
            assert_eq!(
                first > season,
                known[0].words.contains("before he got here"),
                "coach {c}: {:?}",
                known[0].words
            );
            assert!(
                known.iter().skip(1).any(|k| k.words.chars().any(|ch| ch.is_ascii_digit())),
                "coach {c}: {known:?}"
            );
        }
        let case = crate::world::folk::persona::talk::stakes::case_facts(&w, c);
        if first > season {
            assert!(case.iter().any(|x| x.contains("hasn't coached a game here yet")), "coach {c}: {case:?}");
        }
    }
    assert!(reviewed >= 2, "the coordinators review the season: {reviewed} of {}", staff.len());

    // Your boss, asked how it's going before a game: last season's record.
    let ad = office::cast_them(
        &w,
        crate::world::folk::cast::CastRef::new(team, crate::world::folk::cast::Role::AthleticDirector, 0),
    );
    let view = People.knows(&w, ad, Topic::Football, None);
    assert!(view.iter().any(|k| k.words.contains(&record)), "your boss: {view:?}");

    // A starter from last season, asked about his own game, and a coach
    // asked about him: his season's numbers, the coach's with his place.
    let roster = team.roster(&w);
    let starter = row
        .regulars(1)
        .into_iter()
        .chain(row.regulars(0))
        .filter_map(|id| crate::world::folk::person(&w, id))
        .find(|i| roster.contains(i))
        .expect("a starter still on the roster");
    let id = w.people[starter].player.id;
    let own = People.knows(&w, Them::Player(id), Topic::Football, None);
    assert!(own.iter().any(|k| k.words.contains(&season.to_string())), "his own season: {own:?}");
    let coach = staff[0];
    let seen = People.knows(&w, Them::Coach(coach), Topic::Football, Some(Them::Player(id)));
    assert!(seen.iter().any(|k| k.words.contains(&season.to_string())), "his coach on him: {seen:?}");
    eprintln!("your boss: {:?}", view.iter().map(|k| &k.words).collect::<Vec<_>>());
    eprintln!("his coach on a starter: {:?}", seen.iter().map(|k| &k.words).collect::<Vec<_>>());
    for k in own.iter().chain(&seen).chain(&view) {
        assert!(!bad(&k.words), "{:?}", k.words);
    }
}
