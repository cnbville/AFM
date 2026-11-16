//! Scheme fit on every club's draft board (section 7, topic 9): each club
//! reads a prospect's fit to its own systems through its own file, sized
//! by who has the final say, so a 3-4 club and a 4-3 club grade the same
//! edge rusher apart. Your department's looks stay in your file.

use super::legacy::seat;
use crate::level_rating::PositionAligner;
use crate::positions::Position;
use crate::world::pro::office::Power;
use crate::world::pro::scouts::{self, GM, HC, Look, LookKind};
use crate::world::scouting::readers::{self, Staff};
use crate::world::{Stage, World};

/// A declared edge rusher two AI clubs have filed in full, one running a
/// 4-3 and one a 3-4; and your club.
fn edge() -> (World, usize, usize, usize, usize) {
    let mut w = seat("nfl");
    let me = match readers::seat_staff(&w) {
        Some(Staff::Club(t)) => t,
        x => panic!("an NFL seat, got {x:?}"),
    };
    let year = w.year;
    scouts::staff(&mut w, false, year);
    let i =
        w.people.iter().position(|p| p.stage == Stage::Pro && p.player.position == Position::EDGE).expect("an edge");
    let p = &mut w.people[i];
    p.stage = Stage::Declared;
    p.pro = None;
    p.cal.pro_position = Some(Position::EDGE);
    let id = p.player.id;
    let mut ai = (0..w.clubs.len()).filter(|&t| t != me && !w.clubs[t].scouts.is_empty());
    let (a, b) = (ai.next().unwrap(), ai.next().unwrap());
    let d = w.today;
    for (t, front) in [(a, 0), (b, 1)] {
        w.clubs[t].installed.front = front;
        w.clubs[t].office.power = Power::Gm;
        let f = w.clubs[t].files.entry(id).or_default();
        f.clear();
        for (k, kind) in
            [LookKind::School, LookKind::School, LookKind::CrossCheck, LookKind::AllStar, LookKind::Combine]
                .into_iter()
                .enumerate()
        {
            f.push(Look { who: 0, kind, date: d.add(-90 + 10 * k as i32) });
        }
        f.push(Look { who: GM, kind: LookKind::Gm, date: d.add(-30) });
        f.push(Look { who: HC, kind: LookKind::Coach, date: d.add(-25) });
        f.push(Look { who: GM, kind: LookKind::Interview, date: d.add(-20) });
    }
    assert_ne!(w.clubs[a].installed.front(), w.clubs[b].installed.front());
    (w, me, a, b, i)
}

fn board(w: &World, t: usize, i: usize) -> scouts::Board {
    let pos = w.people[i].player.position;
    let align = PositionAligner::for_draft(&[(pos, 60.0)], 1);
    scouts::board_for(w, t, &[(i, 60.0)], &align)
}

fn fit_read(w: &World, t: usize, i: usize) -> f64 {
    let (_, _, f) = readers::read(w, Staff::Club(t), w.people[i].player.id).expect("a written file");
    scouts::club_fit(w, t, i, &f).expect("the file has seen what the systems ask about")
}

#[test]
fn ai_boards_read_fit_to_their_own_systems() {
    let (mut w, _, a, b, i) = edge();
    let (fa, fb) = (fit_read(&w, a, i), fit_read(&w, b, i));
    assert!((fa - fb).abs() > 1e-6, "a 4-3 and a 3-4 club read the same edge apart: {fa} vs {fb}");
    for (t, f) in [(a, fa), (b, fb)] {
        let got = board(&w, t, i).fit.get(&i).copied().expect("fit on an AI board") as f64;
        assert!((got - scouts::FIT_BOARD * f).abs() < 1e-3, "club {t}: {got} against {}", scouts::FIT_BOARD * f);
    }
    // Who has the final say sizes it: a coach with it wants players for
    // his system now, an owner a little less than a GM. (The final say
    // also moves whose voice counts in the room, so the read is taken
    // again.)
    for (power, wgt) in [(Power::Coach, 1.4), (Power::Owner, 0.9)] {
        w.clubs[a].office.power = power;
        let f = fit_read(&w, a, i);
        let got = board(&w, a, i).fit[&i] as f64;
        assert!(
            (got - wgt * scouts::FIT_BOARD * f).abs() < 1e-3,
            "{power:?}: {got} against {}",
            wgt * scouts::FIT_BOARD * f
        );
    }
}

#[test]
fn your_departments_looks_stay_in_your_file() {
    let (mut w, me, a, _, _) = edge();
    let i =
        w.people.iter().position(|p| p.stage == Stage::Pro && !w.clubs[a].files.contains_key(&p.player.id)).unwrap();
    let id = w.people[i].player.id;
    w.people[i].stage = Stage::Declared;
    let d = w.today;
    crate::world::scouting::dept::add(&mut w, id, 0, crate::world::scouting::Kind::School, d.add(-20));
    assert!(!readers::evidence(&w, Staff::Club(me), id).1.is_empty(), "your look is in your file");
    assert!(readers::evidence(&w, Staff::Club(a), id).1.is_empty(), "another club never sees your look");
}
