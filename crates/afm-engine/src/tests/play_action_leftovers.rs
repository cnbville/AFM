//! The scheme-fit lane SF1's leftovers (minor bugs 410, 411 and 412): the
//! boot, the shot and the dig are called off a fake at more than chance,
//! straight drops line up in the gun, and a throw off a run fake is play
//! action on the film.

use crate::game::calling::{Film, Memory, OffCtx, Situation, call_offense};
use crate::game::playbook::{
    Backfield, Family, OffCall, PASS_CONCEPTS, Personnel, PlayKind, faked_family, pass_concept,
};
use crate::game::{OffScheme, Staff};
use crate::player::Level;
use crate::rng::Rng;

fn sit(down: u8, togo: f64, yl: f64) -> Situation {
    Situation {
        down,
        togo,
        yardline_100: yl,
        score_diff: 0,
        quarter: 2,
        half_left: 600.0,
        game_left: 2400.0,
        timeouts: 3,
        opp_timeouts: 3,
        ease: 0.0,
        concede: false,
        def_ease: 0.0,
        def_concede: false,
        college: false,
        lean: 0.0,
        weather: 0.0,
    }
}

fn ctx() -> OffCtx {
    OffCtx {
        has_fb: true,
        te_count: 2,
        qb_run: 0.3,
        level: Level::NFL,
        plan: 1.0,
        designs: [None; 8],
        burner: 0.0,
        shots: 0.0,
        lean: Default::default(),
        players: 11,
    }
}

fn concept(name: &str) -> usize {
    PASS_CONCEPTS.iter().position(|c| c.name == name).expect("a library concept")
}

fn throw(i: usize, fake: bool) -> OffCall {
    let mut c = OffCall::new(
        crate::game::playbook::Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0),
        PlayKind::Pass(i),
        1.0,
        false,
    );
    c.fake = fake;
    c
}

/// 412: a faked throw is play action, a faked screen stays a screen.
#[test]
fn a_throw_off_a_run_fake_is_play_action() {
    let flood = concept("flood");
    assert_eq!(throw(flood, false).family(), Family::Dropback);
    assert_eq!(throw(flood, true).family(), Family::PlayAction);
    let screen = concept("RB screen");
    assert_eq!(throw(screen, true).family(), Family::Screen);
    let boot = concept("play-action boot");
    assert_eq!(throw(boot, false).family(), Family::PlayAction);
    assert_eq!(faked_family(pass_concept(flood), true), Family::PlayAction);
}

/// 412: the film's family shares count the faked throws as play action.
#[test]
fn the_films_families_count_faked_throws_as_play_action() {
    let flood = concept("flood");
    let mut film = Film::default();
    for k in 0..100 {
        film.offense(&throw(flood, k % 4 == 0), 1);
    }
    let fam = film.families();
    assert!((fam[Family::PlayAction as usize] - 0.25).abs() < 1e-6);
    assert!((fam[Family::Dropback as usize] - 0.75).abs() < 1e-6);
    // Added and faded with the rest of the film.
    let mut both = Film::default();
    both.add(&film);
    both.add(&film);
    assert!((both.families()[Family::PlayAction as usize] - 0.25).abs() < 1e-6);
    both.fade(0.5);
    assert!((both.families()[Family::PlayAction as usize] - 0.25).abs() < 1e-6);
}

/// Where each kind of play lines up, and what the fake pool calls, over a
/// spread of downs.
fn tally(scheme: OffScheme) -> (f64, f64, f64) {
    let staff = Staff::average(scheme);
    let mem = Memory::default();
    let ctx = ctx();
    let mut rng = Rng::new(412);
    let downs = [(1, 10.0, 70.0), (2, 7.0, 55.0), (2, 3.0, 45.0), (3, 6.0, 40.0), (1, 10.0, 35.0), (3, 2.0, 60.0)];
    let (mut drops, mut drops_uc, mut runs, mut runs_uc, mut pa, mut pa_special) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..12_000 {
        let (d, t, y) = downs[i % downs.len()];
        let call = call_offense(&staff, &mem, &sit(d, t, y), &ctx, &mut rng);
        let uc = call.formation.backfield == Backfield::UnderCenter;
        match call.kind {
            PlayKind::Run(_) => {
                runs += 1.0;
                runs_uc += uc as u8 as f64;
            }
            PlayKind::Pass(c) if call.play_action() => {
                pa += 1.0;
                pa_special += pass_concept(c).play_action as u8 as f64;
            }
            PlayKind::Pass(_) => {
                drops += 1.0;
                drops_uc += uc as u8 as f64;
            }
            _ => {}
        }
    }
    (drops_uc / drops, runs_uc / runs, pa_special / pa)
}

/// 411: straight drops come from the gun, runs from under center as the
/// system leans, in every system.
#[test]
fn straight_drops_line_up_in_the_gun() {
    for scheme in [OffScheme::WestCoast, OffScheme::PowerGap, OffScheme::AirRaid] {
        let (drops, runs, _) = tally(scheme);
        assert!(drops < 0.12, "{scheme:?}: {drops:.3} of straight drops under center");
        if scheme != OffScheme::AirRaid {
            assert!(runs > 0.35, "{scheme:?}: {runs:.3} of runs under center");
        }
    }
    // The system still leans it: the Air Raid almost never, the wing-T most.
    let (air, _, _) = tally(OffScheme::AirRaid);
    let (power, _, _) = tally(OffScheme::PowerGap);
    assert!(air < power, "{air:.3} {power:.3}");
}

/// 410: the boot, the shot and the dig weigh more than one of 27 each off a
/// fake (they were about 2% of play action each).
#[test]
fn the_boot_the_shot_and_the_dig_carry_a_real_share_of_play_action() {
    let (_, _, special) = tally(OffScheme::WestCoast);
    assert!(special > 0.06, "{special:.3} of play action is the boot, the shot or the dig");
}
