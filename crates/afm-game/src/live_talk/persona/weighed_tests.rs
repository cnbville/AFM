//! A typed plan ask to your coordinator is weighed as you said it, never
//! read onto the sit-down's chip (Cain 2026-11-15: "why is it still
//! linking it to a chip?"): which setting, how far and whether you asked
//! or made it your call, against his own read (the case he's making, his
//! numbers, how sure he is, his trust in you). The chip keeps its plain
//! answer.

use afm_engine::world::World;
use afm_engine::world::coach::Role;
use afm_engine::world::folk::Team;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::office::asks;
use afm_engine::world::folk::persona::terms::Request;
use afm_engine::world::folk::persona::wants::Willing;
use afm_engine::world::folk::scene::{self, Kind, Scene};

use super::tests::game;

/// Your coordinator on one side of the ball, if the staff has one.
fn coordinator(w: &World, role: Role) -> Option<usize> {
    let team = Team::seat(w)?;
    (0..w.coaches.len())
        .find(|&k| w.coaches[k].job.is_some_and(|j| j.role == role && Team::from_employer(j.employer) == team))
}

/// His staff meeting is open with his case on the table (0 more pressure,
/// 1 more zone, 2 run more, 3 throw more).
fn his_case(w: &mut World, c: usize, kind: u8) {
    let mut s = Scene::new(Kind::StaffMeeting, w.today, "Staff meeting".to_string());
    s.a = c as u64;
    s.b = kind as u64;
    scene::add(w, s);
}

fn pressure(w: &World) -> f64 {
    w.plans.guide.pressure.to.unwrap_or(0.3)
}

#[test]
fn a_plan_ask_is_read_as_said_not_as_a_chip() {
    let (g, ..) = game("plan-said", false);
    let w = g.world.as_ref().unwrap();
    let a = asks::plan_ask(w, "more pressure, a lot, order").unwrap();
    assert_eq!((a.m, a.steps, a.order, a.chip), (0, asks::LOT, true, false));
    let a = asks::plan_ask(w, "less blitz, a little").unwrap();
    assert_eq!((a.m, a.steps, a.order), (4, asks::LITTLE, false));
    // A share is the steps it takes from where the plan is (pressure starts
    // at 30%, a step is 8 points): 46% is two steps up.
    let a = asks::plan_ask(w, "pressure 46%").unwrap();
    assert_eq!(a.m, 0);
    assert!((a.steps - 2.0).abs() < 0.01, "{}", a.steps);
    // Passing more is the run share down; zone is less man.
    assert_eq!(asks::plan_ask(w, "pass more").unwrap().m, 3);
    assert_eq!(asks::plan_ask(w, "more zone").unwrap().m, 1);
    // The sit-down's chip is the chip.
    assert!(asks::plan_ask(w, asks::CHIP_WORDS[0]).unwrap().chip);
    assert!(asks::plan_ask(w, "change something").is_none());
}

#[test]
fn a_coordinator_weighs_a_typed_plan_ask_against_his_own_case() {
    let (mut g, ..) = game("plan-weigh", false);
    let w = g.world.as_mut().unwrap();
    let Some(dc) = coordinator(w, Role::DefensiveCoordinator) else { return };
    let him = Them::Coach(dc);
    // He's arguing for more pressure: asking for less is against him.
    his_case(w, dc, 0);
    let less = Request::Plan("less pressure".into());
    let wl = asks::willing(w, him, &less).unwrap();
    assert!(matches!(wl, Willing::If(_)), "he pushes back: {wl:?}");
    let said = asks::words(w, him, &less, &wl);
    assert!(said.iter().any(|x| x.contains("asking for more pressure")), "{said:?}");
    // The same ask as your call: he puts it in, it costs his trust in you,
    // and the week is kept as your call against his numbers.
    let order = Request::Plan("less pressure, order".into());
    assert_eq!(asks::willing(w, him, &order), Some(Willing::Yes));
    let before = afm_engine::world::folk::persona::office::trust(w, him);
    let p0 = pressure(w);
    asks::run(w, him, &order).unwrap();
    assert!(pressure(w) < p0, "the plan moved");
    assert!(afm_engine::world::folk::persona::office::trust(w, him) < before, "he holds it against you a little");
    let case = w.folk.dialogue.case.expect("the week is kept");
    assert_eq!((case.coach, case.said, case.mine), (dc, 2, Some(4)));
    // Asking for what he wants, a lot: he's for it, and it moves two steps.
    let p1 = pressure(w);
    let more = Request::Plan("more pressure, a lot".into());
    assert_eq!(asks::willing(w, him, &more), Some(Willing::Yes));
    asks::run(w, him, &more).unwrap();
    assert!((pressure(w) - (p1 + 0.16)).abs() < 0.005, "{p1} -> {}", pressure(w));
    assert_eq!(w.folk.dialogue.case.map(|c| (c.said, c.mine)), Some((0, None)), "his to own");
}

#[test]
fn a_plan_ask_off_his_side_of_the_ball_is_not_his_and_the_chip_keeps_its_answer() {
    let (mut g, ..) = game("plan-side", false);
    let w = g.world.as_mut().unwrap();
    let Some(dc) = coordinator(w, Role::DefensiveCoordinator) else { return };
    let him = Them::Coach(dc);
    let run = Request::Plan("run more".into());
    assert_eq!(asks::willing(w, him, &run), Some(Willing::No));
    assert!(asks::words(w, him, &run, &Willing::No).iter().any(|x| x.contains("offensive coordinator")));
    // The chip, against his case: the plain ask he always took, one step.
    his_case(w, dc, 0);
    let chip = Request::Plan(asks::CHIP_WORDS[4].into());
    assert_eq!(asks::willing(w, him, &chip), Some(Willing::Yes));
    let p0 = pressure(w);
    asks::run(w, him, &chip).unwrap();
    assert!((pressure(w) - (p0 - 0.08)).abs() < 0.005);
}
