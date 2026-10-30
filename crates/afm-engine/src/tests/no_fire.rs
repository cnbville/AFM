//! The no-fire override (Settings, a test switch): the world's reviews still
//! decide to end your job, but with the override on you keep it, and the
//! day and the reason are kept on your career and told in your inbox. Off,
//! nothing changes.

use crate::world::coach::Employer;
use crate::world::seat::{Job, You};
use crate::world::staff::{Scope, carousel};
use crate::world::{World, WorldOptions};

fn you() -> You {
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

fn tiny() -> World {
    World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() })
}

/// Runs the same review on two copies of one world, the override off and
/// on, and checks the first loses the job and the second keeps it with one
/// kept reason. `make` builds the world the same way each time.
fn both_ways(make: impl Fn() -> (World, usize), scope: Scope) {
    let (mut off, c) = make();
    let job = off.coaches[c].job.expect("you start with a job");
    carousel(&mut off, scope);
    assert_eq!(off.coaches[c].job, None, "off: the review ends your job");
    let why = off.coaches[c].history.last().map(|s| s.ended).unwrap();
    assert!(off.career.as_ref().unwrap().spared.is_empty());

    let (mut on, _) = make();
    on.set_no_fire(true);
    let since = on.career.as_ref().unwrap().no_fire_since;
    assert_eq!(since, Some(on.today), "the career is marked the day it goes on");
    carousel(&mut on, scope);
    assert_eq!(on.coaches[c].job, Some(job), "on: you keep the job");
    let k = on.career.as_ref().unwrap();
    assert_eq!(k.spared.len(), 1, "{:?}", k.spared);
    assert_eq!(k.spared[0].2, why, "the world's own reason is kept");
    assert!(!k.spared[0].1.is_empty(), "where it happened");
    // The mark stays after the override goes off.
    on.set_no_fire(false);
    let k = on.career.as_ref().unwrap();
    assert!(!k.no_fire && k.no_fire_since == since);
}

#[test]
fn an_nfl_owner_would_fire_you_but_the_override_keeps_you() {
    // Four seasons in, two without a win, an impatient owner: the firing
    // chance is 0.9 (this seed's roll fires).
    both_ways(
        || {
            let mut w = tiny();
            let t = (0..w.clubs.len()).find(|&t| crate::world::franchise::active(t, w.year)).unwrap();
            let c = w.start_career(&you(), Job::Nfl(t), 7);
            let year = w.year;
            w.coaches[c].job.as_mut().unwrap().since = year - 4;
            w.coaches[c].recent = vec![-0.4, -0.4];
            w.nfl[t].win_share = vec![0.0, 0.0];
            w.nfl[t].owner = 1.0;
            assert_eq!(w.coaches[c].job.unwrap().employer, Employer::Nfl(t));
            (w, c)
        },
        Scope::NflFirings,
    );
}

#[test]
fn a_college_that_lets_your_deal_run_out_keeps_you_under_the_override() {
    // Six seasons in, two bad ones and a deal that ends this year: not
    // retained, if the AD doesn't fire him first.
    both_ways(
        || {
            let mut w = tiny();
            let p = (0..w.programs.len())
                .find(|&p| w.programs[p].plays(w.year) && w.programs[p].plays(w.year + 1))
                .unwrap();
            let c = w.start_career(&you(), Job::College(p), 7);
            let year = w.year;
            w.coaches[c].job.as_mut().unwrap().since = year - 6;
            w.coaches[c].recent = vec![-0.5, -0.5];
            w.coaches[c].prove_it = None;
            if let Some(k) = w.coaches[c].contract.as_mut() {
                k.through = year;
            }
            (w, c)
        },
        Scope::College,
    );
}
