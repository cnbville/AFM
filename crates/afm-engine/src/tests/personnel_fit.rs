//! Lane SF2 (scheme-fit items 1 and 2): the call decides run, pass or play
//! action first and the personnel group fits it, a preference is a lean and
//! never forces the call, and the tight ends past the first run real routes
//! the quarterback reads.

use crate::game::calling::{CallMemo, Memory, OffCtx, Situation, call_offense, call_offense_memo};
use crate::game::guide::{Firm, Want};
use crate::game::playbook::{
    Backfield, Formation, ORole, PASS_CONCEPTS, Personnel, PlayKind, Route, concept, pass_concept, readable,
    readable_library, reads_in, route_for, route_in,
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
            PlayKind::Pass(k) => (true, !pass_concept(k).play_action),
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
    // A power team set firmly to 13 (the audit's pg13): before, it threw
    // 45% from 13 and 34% from 12 (real 36% and 46%), because the group was
    // put on the field without looking at the call.
    let staff = wanting(OffScheme::PowerGap, Personnel::P13, 10.0);
    let s13 = split(&staff, Personnel::P13, 3);
    let s12 = split(&staff, Personnel::P12, 3);
    println!(
        "13: {:.2} of runs and play action, {:.2} of dropbacks; passes {:.2} from 13, {:.2} from 12, {:.2} from 11",
        s13.run_look, s13.dropback, s13.pass_rate, s12.pass_rate, s13.pass_rate_11
    );
    assert!(
        s13.run_look > s13.dropback + 0.1,
        "13 fits the run looks: {:.2} against {:.2}",
        s13.run_look,
        s13.dropback
    );
    assert!(s13.dropback > 0.15, "a straight dropback still comes from 13 at times: {:.2}", s13.dropback);
    // The right way round: the lighter the group, the more it throws.
    assert!(s13.pass_rate + 0.05 < s12.pass_rate, "{:.2} from 13, {:.2} from 12", s13.pass_rate, s12.pass_rate);
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
    // The call keeps its own memo for the snap (play action's share and the
    // concept read the same weights), so a memo kept over many calls draws
    // the same calls, draw for draw.
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
}
