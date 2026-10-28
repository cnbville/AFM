//! The systems to chase (update 2, lane 2): a new job opens the ambition
//! picker the day you start, with offers that read whole and pay within
//! their bounds, three of your own at most; each level's five chases read
//! whole on its seat; every state has a coaches' hall line.

use crate::world::data::Division;
use crate::world::legacy::{self, ambitions, hall, view};
use crate::world::seat::{Job, You};
use crate::world::{World, WorldOptions};

pub(super) fn you() -> You {
    let teaching = vec![12.0; crate::world::coach::GROUP_COUNT];
    serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap()
}

/// A tiny world with your career at one of each level's teams.
pub(super) fn seat(level: &str) -> World {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let job = match level {
        "nfl" => Job::Nfl(
            (0..w.clubs.len())
                .filter(|&t| crate::world::franchise::active(t, w.year))
                .max_by_key(|&t| w.clubs[t].roster.len())
                .unwrap(),
        ),
        "college" => Job::College(w.programs.iter().position(|p| p.division == Division::FBS).unwrap()),
        _ => Job::HighSchool(w.high_schools.iter().position(|h| w.states[h.state].code == "TX").unwrap_or(0)),
    };
    w.start_career(&you(), job, 7);
    w
}

/// Words a screen must never show.
fn whole(s: &str) -> bool {
    !s.is_empty()
        && !s.contains(['{', '}'])
        && !s.split(|c: char| !c.is_alphanumeric()).any(|t| ["NaN", "inf", "undefined", "null"].contains(&t))
}

#[test]
fn a_new_job_opens_the_picker_with_ladders_that_pay_within_bounds() {
    for level in ["nfl", "college", "hs"] {
        let mut w = seat(level);
        legacy::hire_check(&mut w);
        let p = w.legacy.picker.clone().unwrap_or_else(|| panic!("{level}: the picker opens the day you start"));
        assert!((3..=8).contains(&p.offers.len()), "{level}: {} offers", p.offers.len());
        for id in &p.offers {
            let a = w.legacy.ambitions.iter().find(|a| a.id == *id).unwrap();
            assert!(whole(&a.title) && whole(&a.why) && whole(&a.closes), "{level}: {a:?}");
            assert_eq!(a.steps.len(), 5, "{level}: {}", a.title);
            // A step pays by its own difficulty: a waiting step at the end
            // (a building paid off three years after it opens) pays less.
            for s in &a.steps {
                assert!(whole(&s.label), "{level}: {}: {:?}", a.title, s.label);
                assert!((0.25..=1.0).contains(&s.pay), "{level}: {}: {:?}", a.title, a.steps);
            }
            assert!(whole(&ambitions::quote(&w, a)), "{level}: {}", a.title);
        }
        // "Decide later" once, then three of your own at most.
        assert!(ambitions::keep(&mut w, true).is_ok());
        assert!(ambitions::keep(&mut w, true).is_err(), "{level}: put off only once");
        for (k, id) in p.offers.iter().enumerate() {
            let r = ambitions::pick(&mut w, *id, true);
            assert_eq!(r.is_ok(), k < 3, "{level}: pick {k}: {r:?}");
        }
        assert!(ambitions::loud(&mut w, p.offers[0], true).is_ok());
        assert!(ambitions::keep(&mut w, false).is_ok());
        assert!(w.legacy.picker.is_none());
        let mine = ambitions::mine(&w);
        assert_eq!(mine.iter().filter(|a| a.open()).count(), 3, "{level}");
        assert!(mine.iter().any(|a| a.loud), "{level}: one said out loud");
        // The same job doesn't open a second picker.
        legacy::hire_check(&mut w);
        assert!(w.legacy.picker.is_none(), "{level}: one picker a job");
    }
}

#[test]
fn each_levels_five_chases_read_whole() {
    for level in ["nfl", "college", "hs"] {
        let w = seat(level);
        let (_, five) = view::five(&w).unwrap_or_else(|| panic!("{level}: your five"));
        assert_eq!(five.len(), 5, "{level}");
        for c in &five {
            for s in [&c.name, &c.word, &c.home, &c.line] {
                assert!(whole(s), "{level} {}: {s:?}", c.code);
            }
            for (k, v, _) in &c.kv {
                assert!(whole(k) && whole(v), "{level} {}: {k}: {v:?}", c.code);
            }
            let finite = match &c.vis {
                view::Vis::Bars { data, .. } => data.iter().all(|d| d.1.iter().all(|x| x.is_finite())),
                view::Vis::Rows { rows, max } => max.is_finite() && rows.iter().all(|r| r.1.is_finite() && whole(&r.0)),
                view::Vis::Line { pts, lo, hi, .. } => {
                    lo.is_finite() && hi.is_finite() && pts.iter().all(|p| p.1.is_finite())
                }
                view::Vis::Empty { text } => whole(text),
            };
            assert!(finite, "{level} {}: {:?}", c.code, c.vis);
        }
        assert!(whole(&view::context(&w)), "{level}");
    }
}

#[test]
fn every_state_has_a_coaches_hall_line() {
    let w = World::empty(WorldOptions { scale: 0.001, history_years: 0, ..Default::default() });
    let mut researched = 0;
    for s in &w.states {
        let r = hall::rule_for(&s.code);
        assert!(whole(&r.hall) && whole(&r.line), "{}: {r:?}", s.code);
        if r.researched {
            researched += 1;
            assert!(r.source.starts_with("http"), "{}: a published line names its source", s.code);
        } else {
            assert!(r.line.contains("proposed"), "{}: an unpublished line says it's proposed", s.code);
        }
    }
    assert!(researched >= 10, "{researched} states on a published line");
    for level in ["hs", "college", "nfl"] {
        let g = hall::greats(level);
        assert!(g.len() >= 4 && g.iter().all(|x| whole(&x.0) && whole(&x.1)), "{level}: {g:?}");
    }
}

#[test]
fn a_feeder_takes_your_help_before_its_first_june() {
    let mut w = seat("hs");
    let h = w.seat_school().expect("your school");
    let feeders = crate::world::front_office::school::feeders(&w, h);
    let f = feeders.iter().find(|f| !f.youth).expect("a feeder middle school");
    // The School and feeders screen offers every feeder from your first
    // day, not only the ones June has listed.
    w.legacy.hs.feeders.clear();
    legacy::hs::give_feeder(&mut w, &f.name, "playbook").unwrap();
    let plan = w.legacy.hs.feeders.iter().find(|x| x.name == f.name).expect("its plan");
    assert!(plan.playbook && !plan.clinic && plan.adopt == 0.0, "the help lands now, the take-up in June");
    assert!(legacy::hs::give_feeder(&mut w, "Nowhere Middle School", "clinic").is_err());
    if let Some(y) = feeders.iter().find(|f| f.youth) {
        assert!(legacy::hs::give_feeder(&mut w, &y.name, "clinic").is_err(), "a youth league takes none of it");
    }
}
