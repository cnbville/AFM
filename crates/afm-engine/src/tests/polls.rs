//! The poll and award voters (section 6 topic 9): AP and Coaches Poll
//! ballots, preseason anchoring, the committee's protocol and recusal.

use crate::calendar::Date;
use crate::world::college::{self, PollKind};
use crate::world::data::Division;
use crate::world::schedule::{Game, GameKind, Played};
use crate::world::voters::{self, Member, MemberRole, Traits};
use crate::world::{World, WorldOptions, polls};

/// An empty world in a season's fall, every FBS program on the board with
/// an even preseason view and no games yet.
fn season(year: i32) -> World {
    let mut w =
        World::empty(WorldOptions { scale: 0.01, start_year: year + 12, history_years: 0, ..Default::default() });
    assert_eq!(w.year, year);
    w.rules = w.changes.rules(year);
    let mut cs = college::new_season(&w);
    cs.games.clear();
    cs.started = true;
    for p in w.programs.iter_mut().filter(|p| p.division == Division::FBS) {
        p.roster = vec![0];
        p.prestige = 0.4;
        cs.results[p.id].preseason = 50.0;
    }
    w.college = Some(cs);
    voters::ensure(&mut w);
    w
}

fn id(w: &World, name: &str) -> usize {
    w.programs.iter().find(|p| p.real_name == name).unwrap_or_else(|| panic!("{name}")).id
}

fn fbs(w: &World) -> Vec<usize> {
    w.programs.iter().filter(|p| p.division == Division::FBS && p.plays(w.year)).map(|p| p.id).collect()
}

fn below_fbs(w: &World) -> usize {
    w.programs.iter().find(|p| p.division == Division::FCS).unwrap().id
}

/// A played game: `winner` at home by `by` points.
fn game(w: &mut World, date: Date, winner: usize, loser: usize, by: u32, winner_home: bool) {
    let (home, away) = if winner_home { (winner, loser) } else { (loser, winner) };
    let mut g = Game::new(date, home, away, Division::FBS, GameKind::Regular);
    let score = if winner_home { [20 + by, 20] } else { [20, 20 + by] };
    g.played = Some(Played { score, home_won: winner_home, temperature: 60.0, ..Played::default() });
    let cs = w.college.as_mut().unwrap();
    for (t, won) in [(winner, true), (loser, false)] {
        if won {
            cs.results[t].wins += 1;
        } else {
            cs.results[t].losses += 1;
        }
    }
    cs.games.push(g);
}

fn rank(release: &polls::Release, t: usize) -> Option<usize> {
    release.top.iter().position(|&x| x == t).map(|k| k + 1)
}

#[test]
fn every_panel_is_built_and_old_worlds_get_them() {
    let w = season(2010);
    assert_eq!(w.voters.ap.len(), voters::AP_PANEL);
    // The Harris panel served the BCS from 2005 to 2013.
    assert_eq!(w.voters.harris.len(), voters::HARRIS_PANEL);
    assert_eq!(w.voters.heisman.len(), 870);
    for r in 0..6 {
        assert_eq!(w.voters.heisman.iter().filter(|v| v.region == r).count(), 145);
    }
    // No committee before the playoff; one of 13 from 2014.
    assert!(w.voters.committee.is_empty());
    let w = season(2016);
    assert_eq!(w.voters.committee.len(), voters::COMMITTEE_SIZE);
    assert!(w.voters.harris.is_empty());
    // Each AP voter writes for his own outlet, and has a name.
    let mut outlets: Vec<u16> = w.voters.ap.iter().map(|v| v.outlet).collect();
    outlets.sort();
    outlets.dedup();
    assert_eq!(outlets.len(), voters::AP_PANEL);
    let v = &w.voters.ap[0];
    assert!(voters::media_name(w.options.seed, v.id, v.born, v.ethnicity).contains(' '));
}

#[test]
fn a_ballot_ranks_an_unbeaten_sec_team_over_a_two_loss_power() {
    let mut w = season(2010);
    let (vandy, osu) = (id(&w, "Vanderbilt"), id(&w, "Ohio State"));
    // The big name, picked high in August: the voters know it.
    w.programs[osu].prestige = 0.95;
    w.college.as_mut().unwrap().results[osu].preseason = 75.0;
    let others: Vec<usize> = fbs(&w).into_iter().filter(|&t| t != vandy && t != osu).collect();
    let start = w.dates.cfb_weeks[0];
    for k in 0..8 {
        let d = start.add(7 * k as i32);
        game(&mut w, d, vandy, others[k], 10, k % 2 == 0);
        if k < 6 {
            game(&mut w, d, osu, others[10 + k], 10, true);
        } else {
            game(&mut w, d, others[10 + k], osu, 7, true);
        }
    }
    let rel = polls::vote(&mut w, start.add(7 * 8), PollKind::Ap);
    let (v, o) = (rank(&rel, vandy).unwrap(), rank(&rel, osu).unwrap_or(99));
    assert!(v < o, "unbeaten Vanderbilt {v}, two-loss Ohio State {o}");
    assert_eq!(rel.points.len(), rel.top.len());
    assert!(rel.points.windows(2).all(|p| p[0] >= p[1]));
    // Every AP voter keeps his ballot of 25.
    assert!(w.voters.ap.iter().all(|m| m.ballot.len() == 25));
}

#[test]
fn anchoring_keeps_a_preseason_number_one_high_after_a_close_early_loss() {
    let mut w = season(2012);
    let (bama, fsu) = (id(&w, "Alabama"), id(&w, "Florida State"));
    {
        let cs = w.college.as_mut().unwrap();
        cs.results[bama].preseason = 90.0;
        cs.results[fsu].preseason = 80.0;
    }
    w.programs[bama].prestige = 0.95;
    w.programs[fsu].prestige = 0.8;
    let pre_day = w.dates.cfb_weeks[0].add(-10);
    college::release_poll(&mut w, pre_day, PollKind::ApPreseason);
    let pre = w.college.as_ref().unwrap().polls.last().unwrap().top.clone();
    assert_eq!(pre[0], bama, "the preseason favorite");
    // Week one: Alabama loses by three on the road at Florida State;
    // forty other teams beat teams from below.
    let d = w.dates.cfb_weeks[0];
    game(&mut w, d, fsu, bama, 3, true);
    let fcs = below_fbs(&w);
    for t in fbs(&w).into_iter().filter(|&t| t != bama && t != fsu).take(40) {
        game(&mut w, d, t, fcs, 21, true);
    }
    college::release_poll(&mut w, d.add(1), PollKind::Ap);
    let after = w.college.as_ref().unwrap().polls.last().unwrap().top.clone();
    let r = after.iter().position(|&t| t == bama).map(|k| k + 1).unwrap_or(99);
    assert!(r <= 10, "Alabama fell to {r} after a close early loss");
    // The winner climbs past it.
    let f = after.iter().position(|&t| t == fsu).map(|k| k + 1).unwrap_or(99);
    assert!(f < r, "Florida State {f}, Alabama {r}");
}

#[test]
fn coaches_lean_to_their_own_team() {
    let mut w = season(2012);
    let teams = fbs(&w);
    // Every coach on the panel votes for his own program: give each a
    // head coach id of his own.
    w.voters.coaches = teams
        .iter()
        .take(65)
        .enumerate()
        .map(|(k, &t)| voters::CoachSeat { program: t as u16, coach: k as u32, since: 2012, ballot: Vec::new() })
        .collect();
    for (k, &t) in teams.iter().take(65).enumerate() {
        w.programs[t].head_coach = Some(k);
    }
    let start = w.dates.cfb_weeks[0];
    // Two even 3-0 teams; only one's coach is on the panel.
    let (a, b) = (teams[0], teams[100]);
    let fcs = below_fbs(&w);
    for k in 0..3 {
        game(&mut w, start.add(7 * k), a, fcs, 14, true);
        game(&mut w, start.add(7 * k), b, fcs, 14, true);
    }
    polls::vote(&mut w, start.add(15), PollKind::Coaches);
    let seat = w.voters.coaches.iter().find(|s| s.program as usize == a).unwrap();
    let own = seat.ballot.iter().position(|&p| p as usize == a);
    let other = seat.ballot.iter().position(|&p| p as usize == b);
    assert!(own.is_some());
    assert!(own < other.or(Some(99)), "own {own:?}, other {other:?}");
}

fn member(recused: Vec<u16>) -> Member {
    Member {
        name: "Test Member".into(),
        role: MemberRole::Administrator,
        school: None,
        recused,
        coach: None,
        since: 2016,
        term_end: 2018,
        traits: Traits { anchor: 0, brand: 30, resume: 150, margin: 128, homer: 50, care: 220, g5: 128 },
    }
}

#[test]
fn the_committee_recuses_and_reads_the_whole_season() {
    let mut w = season(2016);
    let (bama, clemson) = (id(&w, "Alabama"), id(&w, "Clemson"));
    let others: Vec<usize> = fbs(&w).into_iter().filter(|&t| t != bama && t != clemson).collect();
    let start = w.dates.cfb_weeks[0];
    for k in 0..10 {
        let d = start.add(7 * k as i32);
        game(&mut w, d, bama, others[k], 14, k % 2 == 0);
        // Clemson loses its opener and its last game.
        if k == 0 || k == 9 {
            game(&mut w, d, others[20 + k], clemson, 3, true);
        } else {
            game(&mut w, d, clemson, others[20 + k], 14, true);
        }
    }
    // Twelve of the thirteen are recused from Alabama: the one left in the
    // room decides its place, and the absent votes don't count against it.
    let mut committee: Vec<Member> = (0..12).map(|_| member(vec![bama as u16])).collect();
    committee.push(member(Vec::new()));
    w.voters.committee = committee;
    let rel = polls::committee(&mut w, start.add(7 * 10 + 3));
    assert_eq!(rel.top.len(), 25);
    assert_eq!(rank(&rel, bama), Some(1));
    let c = rank(&rel, clemson).unwrap_or(99);
    assert!(c > 1, "two-loss Clemson {c}");
    // Every member recused from Alabama: nobody may list it, so it only
    // comes onto the board below the teams the members could rank.
    w.voters.committee = (0..13).map(|_| member(vec![bama as u16])).collect();
    let rel = polls::committee(&mut w, start.add(7 * 10 + 3));
    assert_ne!(rank(&rel, bama), Some(1));
}

#[test]
fn the_bcs_standings_combine_polls_and_computers() {
    let mut w = season(2008);
    let teams = fbs(&w);
    let start = w.dates.cfb_weeks[0];
    let (a, b) = (teams[3], teams[4]);
    for k in 0..8 {
        let d = start.add(7 * k as i32);
        game(&mut w, d, a, teams[10 + k], 17, true);
        game(&mut w, d, teams[30 + k], b, 3, true);
    }
    let d = start.add(7 * 8);
    college::release_poll(&mut w, d, PollKind::Coaches);
    college::release_poll(&mut w, d, PollKind::Harris);
    college::release_poll(&mut w, d, PollKind::Bcs);
    let bcs = w.college.as_ref().unwrap().polls.last().unwrap().clone();
    assert_eq!(bcs.kind, PollKind::Bcs);
    assert_eq!(bcs.top[0], a, "the unbeaten team leads the standings");
    assert!(!bcs.top.contains(&b));
}
