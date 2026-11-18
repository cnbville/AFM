//! Lane SF2 (scheme-fit items 1 and 2): the call decides run, pass or play
//! action first and the personnel group fits it, a preference is a lean and
//! never forces the call, and the tight ends past the first run real routes
//! the quarterback reads.

use crate::game::calling::{CallMemo, Memory, OffCtx, Situation, call_offense, call_offense_memo};
use crate::game::guide::{Firm, Want};
use crate::game::playbook::{
    Backfield, Formation, LIBRARY_ALL, ORole, PASS_CONCEPTS, Personnel, PlayKind, Route, Set, concept, pass_concept,
    readable, readable_library, reads_in, route_for, route_in,
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

/// A team with three healthy tight ends and a fullback, so every group can
/// take the field.
fn ctx() -> OffCtx {
    OffCtx {
        has_fb: true,
        te_count: 3,
        qb_run: 0.2,
        level: Level::NFL,
        plan: 0.75,
        designs: [None; 8],
        burner: 0.0,
        shots: 0.0,
        lean: Default::default(),
        players: 11,
    }
}

/// Normal downs only: no short yardage, no hurry-up.
const NORMAL: [(u8, f64, f64); 6] =
    [(1, 10.0, 70.0), (2, 7.0, 55.0), (2, 4.0, 45.0), (1, 10.0, 35.0), (3, 6.0, 40.0), (2, 10.0, 60.0)];

/// What a staff calls over the normal downs: a group's share of the runs
/// and play action, its share of the straight dropbacks, its pass rate,
/// and the pass rate of every call.
struct Split {
    run_look: f64,
    dropback: f64,
    /// The group's own snaps that are runs or throws off a fake.
    mostly: f64,
    pass_rate: f64,
    pass_rate_11: f64,
    all_pass: f64,
}

fn split(staff: &Staff, group: Personnel, seed: u32) -> Split {
    let mut rng = Rng::new(seed);
    let mem = Memory::default();
    let ctx = ctx();
    let (mut looks, mut drops, mut g_looks, mut g_drops, mut g_pass, mut passes) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut g_n, mut e_pass, mut e_n) = (0.0_f64, 0.0, 0.0_f64);
    let n = 12_000;
    for i in 0..n {
        let (d, t, y) = NORMAL[i % NORMAL.len()];
        let c = call_offense(staff, &mem, &sit(d, t, y), &ctx, &mut rng);
        let p = c.formation.personnel;
        let (pass, drop) = match c.kind {
            PlayKind::Pass(_) => (true, !c.play_action()),
            _ => (false, false),
        };
        passes += f64::from(u8::from(pass));
        if drop {
            drops += 1.0;
            g_drops += f64::from(u8::from(p == group));
        } else {
            looks += 1.0;
            g_looks += f64::from(u8::from(p == group));
        }
        if p == group {
            g_n += 1.0;
            g_pass += f64::from(u8::from(pass));
        }
        if p == Personnel::P11 {
            e_n += 1.0;
            e_pass += f64::from(u8::from(pass));
        }
    }
    Split {
        run_look: g_looks / looks,
        dropback: g_drops / drops,
        mostly: g_looks / (g_looks + g_drops).max(1.0),
        pass_rate: g_pass / g_n.max(1.0),
        pass_rate_11: e_pass / e_n.max(1.0),
        all_pass: passes / n as f64,
    }
}

fn wanting(scheme: OffScheme, group: Personnel, creativity: f64) -> Staff {
    let mut staff = Staff::average(scheme);
    staff.offense.creativity = creativity;
    staff.guide.personnel = Want::new(group, Firm::Firm);
    staff
}

#[test]
fn a_heavy_group_comes_out_on_runs_and_play_action() {
    // A power team set firmly to 13 (the audit's pg13). The group used to
    // go on the field without looking at the call, so a power team set to
    // 13 threw 45% from 13 and 34% from 12 (real 36% and 46%). Now 13 comes
    // out mostly for runs and throws off a run fake, a straight dropback
    // more often from 12 or 11, and the lighter the group, the more it
    // throws. Real play action comes from the run's groups, so 13 still
    // throws off the fake at its real rate (lane SF1).
    let staff = wanting(OffScheme::PowerGap, Personnel::P13, 10.0);
    let s13 = split(&staff, Personnel::P13, 3);
    let s12 = split(&staff, Personnel::P12, 3);
    println!(
        "13: {:.2} of runs and play action, {:.2} of dropbacks, its own snaps {:.2} runs and fakes; passes {:.2} from 13, \
         {:.2} from 12, {:.2} from 11",
        s13.run_look, s13.dropback, s13.mostly, s13.pass_rate, s12.pass_rate, s13.pass_rate_11
    );
    assert!(
        s13.run_look > s13.dropback + 0.1,
        "13 fits the run looks: {:.2} against {:.2}",
        s13.run_look,
        s13.dropback
    );
    assert!(s13.mostly > 0.6, "13's snaps are mostly runs and fakes: {:.2}", s13.mostly);
    assert!(
        s13.dropback > 0.15 && s13.dropback < 0.5,
        "a straight dropback comes from 13 at times, more often from 12 or 11: {:.2}",
        s13.dropback
    );
    // The right way round: the lighter the group, the more it throws.
    assert!(s13.pass_rate + 0.02 < s12.pass_rate, "{:.2} from 13, {:.2} from 12", s13.pass_rate, s12.pass_rate);
    assert!(s12.pass_rate + 0.1 < s13.pass_rate_11, "{:.2} from 12, {:.2} from 11", s12.pass_rate, s13.pass_rate_11);
}

#[test]
fn a_creative_caller_still_throws_from_heavy_sets() {
    // The tell (topic 7.1): a predictable caller matches his groups to his
    // calls, a creative one shows the same looks for both.
    let tight = split(&wanting(OffScheme::PowerGap, Personnel::P13, 2.0), Personnel::P13, 5);
    let loose = split(&wanting(OffScheme::PowerGap, Personnel::P13, 20.0), Personnel::P13, 5);
    println!(
        "predictable: {:.2} / {:.2}; creative: {:.2} / {:.2}",
        tight.run_look, tight.dropback, loose.run_look, loose.dropback
    );
    assert!(loose.dropback > tight.dropback + 0.1, "{:.2} against {:.2}", loose.dropback, tight.dropback);
    assert!(loose.run_look - loose.dropback < tight.run_look - tight.dropback - 0.2);
}

#[test]
fn a_personnel_preference_never_moves_the_run_or_pass_call() {
    // Cain: "personnel is a preference but scheme should still decide the
    // play". On normal downs and on 3rd and 1 in their territory, a firm 13
    // calls the same share of passes as no preference at all.
    let own = Staff::average(OffScheme::PowerGap);
    let set = wanting(OffScheme::PowerGap, Personnel::P13, 10.0);
    let a = split(&own, Personnel::P13, 6);
    let b = split(&set, Personnel::P13, 7);
    assert!((a.all_pass - b.all_pass).abs() < 0.02, "normal downs: {:.3} against {:.3}", a.all_pass, b.all_pass);
    let third = |staff: &Staff, seed: u32| {
        let mut rng = Rng::new(seed);
        let mem = Memory::default();
        let n = 8000;
        let passes = (0..n)
            .filter(|_| {
                let c = call_offense(staff, &mem, &sit(3, 1.0, 40.0), &ctx(), &mut rng);
                matches!(c.kind, PlayKind::Pass(_))
            })
            .count();
        passes as f64 / n as f64
    };
    let (x, y) = (third(&own, 8), third(&set, 9));
    println!("3rd and 1: passes {x:.3} without a preference, {y:.3} set to 13");
    assert!((x - y).abs() < 0.025, "3rd and 1: {x:.3} against {y:.3}");
}

#[test]
fn a_memo_draws_the_same_calls() {
    // A memo kept over many calls draws the same calls as working the
    // weights out each time, draw for draw.
    let mut staff = wanting(OffScheme::WestCoast, Personnel::P12, 10.0);
    staff.offense.creativity = 14.0;
    let mem = Memory::default();
    let ctx = ctx();
    let sits = [sit(1, 10.0, 70.0), sit(2, 7.0, 55.0), sit(3, 6.0, 40.0), sit(1, 10.0, 15.0), sit(3, 1.0, 30.0)];
    let mut memo = CallMemo::default();
    let (mut a, mut b) = (Rng::new(11), Rng::new(11));
    for i in 0..2000 {
        let s = &sits[i % sits.len()];
        let x = call_offense(&staff, &mem, s, &ctx, &mut a);
        let y = call_offense_memo(&staff, &mem, s, &ctx, &mut b, Some((&mut memo, i % sits.len())));
        assert_eq!(x, y, "call {i}");
    }
}

#[test]
fn a_memo_draws_the_same_calls_on_third_down_in_heavy_sets() {
    // Third down's check of the routes that reach the sticks reads how the
    // group lines up: in 22 the second tight end takes the flanker's place
    // in drive, and runs the flanker's 12-yard in from the wing (a power
    // gap, wide zone or wing-T set) but his own flat from across the ball.
    // So the memo keeps those weights apart, and draws the same calls as
    // working them out each time, on every down that reads the sticks.
    use crate::game::calling::sticks_depth;
    let drive = pass_concept(concept("drive"));
    let doubles = Formation::plain(Personnel::P22, Backfield::UnderCenter, 1.0);
    let wing = Formation { set: Set::Wing, ..doubles };
    assert_eq!(sticks_depth(drive, &wing), 12.0);
    assert_eq!(sticks_depth(drive, &doubles), 2.0);
    for (want, seed) in [(Personnel::P22, 31), (Personnel::P13, 32)] {
        let staff = wanting(OffScheme::PowerGap, want, 10.0);
        let mem = Memory::default();
        let ctx = ctx();
        let sits = [sit(3, 6.0, 40.0), sit(3, 9.0, 55.0), sit(4, 5.0, 35.0), sit(3, 4.0, 9.0)];
        let mut memo = CallMemo::default();
        let (mut a, mut b) = (Rng::new(seed), Rng::new(seed));
        for i in 0..4000 {
            let s = &sits[i % sits.len()];
            let x = call_offense(&staff, &mem, s, &ctx, &mut a);
            let y = call_offense_memo(&staff, &mem, s, &ctx, &mut b, Some((&mut memo, i % sits.len())));
            assert_eq!(x, y, "{} personnel, call {i}", want.name());
        }
    }
}

#[test]
fn a_memo_draws_the_same_calls_for_every_scheme_group_and_roster() {
    // The coordinators' draft keeps a box's weights in a memo, one slot a
    // situation (`world/plans.rs`, `draft_sides`). It must draw the same
    // calls as working the weights out each time, draw for draw: for every
    // scheme, every group a guide can want, the rosters that field them or
    // don't, and the rule books' own players a side, over a box from first
    // and ten to the goal line and the hurry-up.
    let late = |mut s: Situation| {
        s.quarter = 4;
        s.half_left = 100.0;
        s.game_left = 100.0;
        s.score_diff = -4;
        s
    };
    let sits = [
        sit(1, 10.0, 75.0),
        sit(2, 8.0, 60.0),
        sit(3, 2.0, 45.0),
        sit(3, 6.0, 40.0),
        sit(3, 12.0, 70.0),
        sit(4, 5.0, 35.0),
        sit(2, 4.0, 15.0),
        sit(3, 4.0, 6.0),
        sit(1, 3.0, 3.0),
        late(sit(2, 10.0, 60.0)),
        late(sit(3, 7.0, 45.0)),
    ];
    let schemes = [
        OffScheme::WestCoast,
        OffScheme::AirRaid,
        OffScheme::WideZone,
        OffScheme::PowerGap,
        OffScheme::SpreadOption,
        OffScheme::WingT,
    ];
    let wants = [
        None,
        Some(Personnel::P10),
        Some(Personnel::P12),
        Some(Personnel::P13),
        Some(Personnel::P21),
        Some(Personnel::P22),
    ];
    // Tight ends, a fullback, players a side.
    let rosters = [(1, false, 11), (2, true, 11), (3, true, 11), (3, false, 11), (3, true, 12), (3, true, 8)];
    let mem = Memory::default();
    let mut seed = 100;
    for scheme in schemes {
        for want in wants {
            for (te, fb, players) in rosters {
                let mut staff = Staff::average(scheme);
                if let Some(g) = want {
                    staff.guide.personnel = Want::new(g, Firm::Firm);
                }
                let ctx = OffCtx { te_count: te, has_fb: fb, players, ..ctx() };
                let mut memo = CallMemo::default();
                seed += 1;
                let (mut a, mut b) = (Rng::new(seed), Rng::new(seed));
                for i in 0..220 {
                    let s = &sits[i % sits.len()];
                    let x = call_offense(&staff, &mem, s, &ctx, &mut a);
                    let y = call_offense_memo(&staff, &mem, s, &ctx, &mut b, Some((&mut memo, i % sits.len())));
                    let group = want.map(|g| g.name());
                    assert_eq!(
                        x, y,
                        "{scheme:?}, want {group:?}, {te} tight ends, fullback {fb}, {players} a side, call {i}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_rule_books_own_sets_run_their_own_concepts() {
    // The call is made in 11-man groups and the game fields the rule
    // book's own set for them. The tunnel screen is built for the flanker:
    // 13 has none in the 11-man game, but the CFL's 12-man set and arena
    // football's 8-man set for 13 do, so there it can come from a 13 call.
    let staff = wanting(OffScheme::WideZone, Personnel::P13, 10.0);
    let mem = Memory::default();
    let tunnel = concept("tunnel screen");
    let tunnels = |players: u8| {
        let mut rng = Rng::new(21);
        let ctx = OffCtx { players, ..ctx() };
        (0..12_000)
            .filter(|i| {
                let (d, t, y) = NORMAL[i % NORMAL.len()];
                let c = call_offense(&staff, &mem, &sit(d, t, y), &ctx, &mut rng);
                c.formation.personnel == Personnel::P13 && c.kind == PlayKind::Pass(tunnel)
            })
            .count()
    };
    let (us, cfl, arena) = (tunnels(11), tunnels(12), tunnels(8));
    println!("tunnel screens from 13: {us} in the 11-man game, {cfl} in the CFL, {arena} in arena football");
    assert_eq!(us, 0);
    assert!(cfl > 0 && arena > 0, "{cfl} in the CFL, {arena} in arena football");
}

#[test]
fn the_extra_tight_ends_take_the_missing_receivers_jobs() {
    use ORole::*;
    let f12 = Formation::plain(Personnel::P12, Backfield::UnderCenter, 1.0);
    let f13 = Formation::plain(Personnel::P13, Backfield::UnderCenter, 1.0);
    // Curl-flat reads the Z, the X, the slot, the tight end and the back,
    // and draws the second tight end a 5-yard out. In 12 the second tight
    // end takes the slot's place in the reads and runs his own out; in 13
    // the second takes the Z's place and his curl, and the third the
    // slot's place and the second's out.
    let c = pass_concept(concept("curl-flat"));
    assert_eq!(route_for(c, TE2), Route::Out(5.0));
    assert_eq!(reads_in(c, &f12), vec![Z, X, TE2, TE, RB]);
    assert_eq!(route_in(c, &f12, TE2), Route::Out(5.0));
    assert_eq!(reads_in(c, &f13), vec![TE2, X, TE3, TE, RB]);
    assert_eq!(route_in(c, &f13, TE2), route_for(c, Z));
    assert_eq!(route_in(c, &f13, TE3), Route::Out(5.0));
    // Texas draws no route for him: he runs the slot's 10-yard out. In
    // smash he keeps the slot's corner, a tight end's own route.
    let t = pass_concept(concept("Texas"));
    assert_eq!(route_in(t, &f12, TE2), route_for(t, Slot));
    let smash = pass_concept(concept("smash"));
    assert_eq!(route_in(smash, &f12, TE2), Route::Corner(11.0));
    for c in PASS_CONCEPTS {
        for f in [&f12, &f13] {
            let reads = reads_in(c, f);
            for (k, r) in reads.iter().enumerate() {
                assert!(f.personnel.has(*r), "{}: {r:?} is on the field", c.name);
                assert!(!reads[..k].contains(r), "{}: {r:?} is read once", c.name);
                if matches!(r, TE2 | TE3) {
                    let route = route_in(c, f, *r);
                    assert!(!matches!(route, Route::Block | Route::CheckRelease), "{}: {r:?} runs a route", c.name);
                }
            }
            assert_eq!(readable(c, f), !reads.is_empty(), "{}", c.name);
        }
    }
}

#[test]
fn a_tight_end_across_the_ball_runs_his_own_job() {
    use ORole::*;
    // The bootleg floods the strong side (the tight end's and the
    // fullback's flats, the flanker's corner) and draws the second tight
    // end the drag back across into the roll. In 22 personnel he takes the
    // flanker's place in the reads but lines up inline on the weak side,
    // so he runs his own drag, not a corner away from the roll; on the
    // wing, on the strong side, he runs the corner. Either strength.
    let boot = pass_concept(concept("play-action boot"));
    let f22 = Formation::plain(Personnel::P22, Backfield::UnderCenter, 1.0);
    assert_eq!(reads_in(boot, &f22), vec![TE, FB, TE2, X]);
    assert_eq!(route_in(boot, &f22, TE2), Route::Drag);
    assert_eq!(route_in(boot, &Formation { strength: -1.0, ..f22 }, TE2), Route::Drag);
    assert_eq!(route_in(boot, &Formation { set: Set::Wing, ..f22 }, TE2), Route::Corner(12.0));
    // 13 personnel: the second tight end lines up on the strong side and
    // takes the corner, the third, inline on the weak side, the slot's drag.
    let f13 = Formation::plain(Personnel::P13, Backfield::UnderCenter, 1.0);
    assert_eq!(reads_in(boot, &f13), vec![TE, TE2, X, TE3]);
    assert_eq!(route_in(boot, &f13, TE2), Route::Corner(12.0));
    assert_eq!(route_in(boot, &f13, TE3), Route::Drag);
    // Smash in 13: the second takes the slot's corner; the third takes the
    // flanker's place across the ball and runs the flat, not his hitch.
    let smash = pass_concept(concept("smash"));
    assert_eq!(route_in(smash, &f13, TE2), Route::Corner(11.0));
    assert_eq!(route_in(smash, &f13, TE3), Route::Flat);
}

#[test]
fn third_down_reads_the_routes_the_group_runs() {
    // Y-option reads the tight end's 7-yard sit, then the slot's 10-yard
    // curl: in 11 personnel it reaches 3rd and 8. In 12 the second tight
    // end takes the slot's place and runs his own flat, so it doesn't.
    use crate::game::calling::sticks_depth;
    let c = pass_concept(concept("Y-option"));
    let f11 = Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0);
    let f12 = Formation::plain(Personnel::P12, Backfield::Shotgun, 1.0);
    assert_eq!(sticks_depth(c, &f11), 10.0);
    assert_eq!(sticks_depth(c, &f12), 7.0);
}

#[test]
fn screens_and_max_protection_keep_their_men() {
    use ORole::*;
    let f13 = Formation::plain(Personnel::P13, Backfield::UnderCenter, 1.0);
    // The tunnel screen is built for the flanker: 13 has none, so the
    // caller can't draw it from 13.
    let tunnel = pass_concept(concept("tunnel screen"));
    assert!(reads_in(tunnel, &f13).is_empty() && !readable(tunnel, &f13));
    // A bubble's ball goes to the man it is built for, never a tight end.
    let bubble = pass_concept(concept("bubble screen"));
    assert!(!reads_in(bubble, &f13).iter().any(|r| matches!(r, TE2 | TE3)));
    assert_eq!(route_in(bubble, &f13, TE3), route_for(bubble, TE3));
    // Seven-man protection keeps the extra tight ends in to block.
    let shot = pass_concept(concept("play-action shot"));
    assert_eq!(route_in(shot, &f13, TE3), Route::Block);
    assert!(!reads_in(shot, &f13).iter().any(|r| matches!(r, TE2 | TE3)));
}

#[test]
fn groups_with_their_receivers_read_as_before() {
    // 11, 10 and 21 personnel lack no receiver a concept reads for a tight
    // end to take: their routes and reads are the concept's own.
    for p in [Personnel::P11, Personnel::P10, Personnel::P21] {
        let f = Formation::plain(p, Backfield::Shotgun, 1.0);
        for c in PASS_CONCEPTS {
            let own: Vec<ORole> = c.reads.iter().copied().filter(|r| p.has(*r)).collect();
            assert_eq!(reads_in(c, &f), own, "{} in {}", c.name, p.name());
            for r in p.roles() {
                assert_eq!(route_in(c, &f, *r), route_for(c, *r), "{} in {}: {r:?}", c.name, p.name());
            }
        }
    }
}

#[test]
fn the_runnable_concepts_table_matches_the_reads() {
    // The call reads a group's runnable library concepts from a table it
    // works out once (`readable_library`): the same answer as `readable`,
    // concept by concept, in every group.
    use Personnel::*;
    for p in [P11, P12, P21, P10, P22, P13, Cfl10, Cfl11, Cfl21, Arena, ArenaTe] {
        let mask = readable_library(p);
        for back in [Backfield::Shotgun, Backfield::UnderCenter] {
            let f = Formation::plain(p, back, 1.0);
            for (i, c) in PASS_CONCEPTS.iter().enumerate() {
                assert_eq!((mask >> i) & 1 == 1, readable(c, &f), "{} in {}", c.name, p.name());
            }
        }
    }
    // 11 personnel runs the whole library, so its calls take the plain
    // draw; 13 doesn't (no flanker for the tunnel screen).
    assert_eq!(readable_library(P11), LIBRARY_ALL);
    assert_ne!(readable_library(P13), LIBRARY_ALL);
    assert_eq!(LIBRARY_ALL.count_ones() as usize, PASS_CONCEPTS.len());
}
