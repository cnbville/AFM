//! A following and famous names (Ground, part I: GR42-GR46): a famous
//! man's son carries his name and hype; the class's biggest followings make
//! big and huge news; a commitment lifts the program's brand exposure for a
//! year, up to 3 points, for the class's 10 biggest followings, and only
//! while he holds it; a recruit weighs brand
//! exposure up to twice by his following.

use crate::calendar::events::Loudness;
use crate::world::background::Bloodline;
use crate::world::following::{self, LIFT_TOP};
use crate::world::pro::{ProCareer, ProSeason, Status};
use crate::world::program::GradeCat;
use crate::world::{Alumnus, Stage, World, WorldOptions};

fn tiny() -> World {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    w.add_class_aged(16);
    w
}

/// A retired four-time Pro Bowler, born so that a son of his is 14 this
/// year, made from one of the world's boys.
fn famous_man(w: &mut World, pro_bowls: usize) -> usize {
    let year = w.year;
    let seed = w.options.seed as u64;
    let p = w.people[0].clone();
    let key = p.player.id as u64;
    let born = (year - 52..=year - 34)
        .find(|&b| following::kin(seed, key, b).iter().any(|&(nephew, at)| !nephew && at == year))
        .expect("a birth year with a son due");
    let mut a = Alumnus::from_person(&p, year - 20, Some(12), false);
    a.player = Some(Box::new(p.player.clone()));
    a.name = "Darnell Hayes".into();
    a.born = crate::calendar::Date::ymd(born, 5, 1);
    let mut c = ProCareer::new(None, Status::FreeAgent, year - 20);
    for k in 0..10 {
        c.seasons.push(ProSeason { year: year - 20 + k, pro_bowl: (k as usize) < pro_bowls, ..Default::default() });
    }
    a.pro = Some(Box::new(c));
    w.alumni.push(a);
    w.alumni.len() - 1
}

#[test]
fn a_famous_mans_son_carries_his_name_and_its_hype() {
    let mut w = tiny();
    let k = famous_man(&mut w, 4);
    let (eth, home) = (w.alumni[k].ethnicity, w.alumni[k].home_state);
    let start = w.people.len();
    w.add_class_aged(14);
    // One boy of the new class whose father played in the NFL.
    let i = start + 3;
    for p in &mut w.people[start..] {
        if p.family.bloodline == Bloodline::NflPlayer {
            p.family.bloodline = Bloodline::None;
        }
    }
    {
        let p = &mut w.people[i];
        p.family.bloodline = Bloodline::NflPlayer;
        p.ethnicity = eth;
        p.home_state = home;
    }
    let before = crate::world::fame::fame(&w, i);
    following::names(&mut w, start);
    let id = w.people[i].player.id;
    let n = w.following.name_of(id).expect("the son carries the name");
    assert!(!n.nephew);
    assert!(n.what.contains("four-time Pro Bowl"), "{}", n.what);
    assert!(n.line().starts_with("His father is Darnell Hayes, a four-time Pro Bowl"), "{}", n.line());
    assert!(w.people[i].name.ends_with("Hayes") || w.people[i].name.ends_with("Hayes Jr."), "{}", w.people[i].name);
    // His fame takes the name's hype: 0.35 x the man's 0.65.
    let after = crate::world::fame::fame(&w, i);
    assert!((after - before - 0.35 * 0.65).abs() < 1e-6, "{before} -> {after}");
    // A man with one Pro Bowl less famous: less hype; none for a man with none.
    let mut w2 = tiny();
    let k2 = famous_man(&mut w2, 0);
    assert_eq!(following::man_fame(&w2, &w2.alumni[k2]), 0.0);
    w2.alumni[k2].pro.as_mut().unwrap().seasons[0].pro_bowl = true;
    assert_eq!(following::man_fame(&w2, &w2.alumni[k2]), 0.4);
}

/// Followers for the class's 12 best-known boys: 120,000 down to 10,000 in
/// steps of 10,000, everyone else 500.
fn spread(w: &mut World) -> Vec<usize> {
    let class = w.people.iter().find(|p| p.stage == Stage::HighSchool).unwrap().class_of;
    let kids: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].class_of == class).collect();
    for (r, &i) in kids.iter().enumerate() {
        w.people[i].followers = if r < 12 { 120_000 - 10_000 * r as u32 } else { 500 };
    }
    following::class_tops(w);
    kids
}

#[test]
fn the_class_biggest_followings_make_big_and_huge_news() {
    let mut w = tiny();
    let kids = spread(&mut w);
    assert_eq!(following::commit_news(&w, kids[0]), Some(Loudness::Huge));
    assert_eq!(following::commit_news(&w, kids[1]), Some(Loudness::Big));
    assert_eq!(following::commit_news(&w, kids[9]), Some(Loudness::Big));
    assert_eq!(following::commit_news(&w, kids[10]), None);
    assert_eq!(following::commit_news(&w, kids[20]), None);
    // His share of the class's biggest following, for his pick (GR45).
    assert_eq!(w.people[kids[0]].spotlight, 1.0);
    assert!((w.people[kids[6]].spotlight - 0.5).abs() < 1e-6);
    assert!(w.people[kids[20]].spotlight < 0.01);
    assert_eq!(following::followers_words(412_400), "412,000 followers");
    assert_eq!(following::followers_words(1_250_000), "1.2 million followers");
}

#[test]
fn a_commitment_lifts_brand_exposure_for_a_year_while_he_holds_it() {
    let mut w = tiny();
    let kids = spread(&mut w);
    let be = GradeCat::BrandExposure as usize;
    let j = 0;
    let base = w.programs[j].grades[be];
    assert!(base + LIFT_TOP <= 99.0);
    let d = w.today;
    // The class's biggest following commits: the full 3 points.
    let top = kids[0];
    w.people[top].cal.commit = Some((j, d, false));
    following::committed(&mut w, top, j, d);
    following::day(&mut w, d);
    assert!((w.programs[j].grades[be] - base - LIFT_TOP).abs() < 1e-6);
    // A second big one doesn't add: the program holds its biggest lift. A kid
    // outside the class's 10 biggest followings lifts nothing.
    let second = kids[6];
    w.people[second].cal.commit = Some((j, d, false));
    following::committed(&mut w, second, j, d);
    following::committed(&mut w, kids[20], j, d);
    following::day(&mut w, d.add(1));
    assert!((w.programs[j].brand_lift - LIFT_TOP).abs() < 1e-6);
    assert_eq!(w.following.lifts.len(), 2);
    // The year's grades move from their own read, then the lift goes back on.
    following::strip(&mut w);
    assert!((w.programs[j].grades[be] - base).abs() < 1e-6);
    following::day(&mut w, d.add(2));
    assert!((w.programs[j].grades[be] - base - LIFT_TOP).abs() < 1e-6);
    // The top one decommits: only the second's half lift is left.
    w.people[top].cal.commit = None;
    following::day(&mut w, d.add(3));
    assert!((w.programs[j].brand_lift - 0.5 * LIFT_TOP).abs() < 1e-4, "{}", w.programs[j].brand_lift);
    // A year on, it's gone.
    following::day(&mut w, d.add(366));
    assert!(w.following.lifts.is_empty());
    assert!((w.programs[j].grades[be] - base).abs() < 1e-6);
    assert_eq!(w.programs[j].brand_lift, 0.0);
}

#[test]
fn a_flip_moves_the_lift() {
    let mut w = tiny();
    let kids = spread(&mut w);
    let be = GradeCat::BrandExposure as usize;
    let d = w.today;
    let (a, b) = (0, 1);
    let (base_a, base_b) = (w.programs[a].grades[be], w.programs[b].grades[be]);
    let top = kids[0];
    w.people[top].cal.commit = Some((a, d, false));
    following::committed(&mut w, top, a, d);
    following::day(&mut w, d);
    w.people[top].cal.commit = Some((b, d.add(30), false));
    following::committed(&mut w, top, b, d.add(30));
    following::day(&mut w, d.add(30));
    assert!((w.programs[a].grades[be] - base_a).abs() < 1e-6);
    assert!((w.programs[b].grades[be] - base_b - LIFT_TOP).abs() < 1e-6);
}

/// A college player's honors add to his fame as a pro's do (the college
/// followings retune): each FBS All-America season and each national award
/// 0.05, the Heisman 0.15, up to 0.25 in all, and last season's biggest
/// honor again on top (0.2 for the Heisman, 0.1 for the rest); an FCS
/// All-America counts half. Honors from before last season add no
/// "latest" part.
#[test]
fn a_college_players_honors_raise_his_fame() {
    use crate::world::data::Division;
    let mut w = tiny();
    let i = w.people.iter().position(|p| p.stage == Stage::HighSchool).unwrap();
    w.people[i].stage = Stage::College;
    w.people[i].stars = 3;
    let id = w.people[i].player.id;
    let base = crate::world::fame::fame(&w, i);
    let last = w.year - 1;
    let add = |w: &mut World, aa: Option<(i32, Division)>, award: Option<(i32, &'static str)>| {
        let h = w.stats.honors.entry(id).or_default();
        h.all_america.extend(aa);
        h.awards.extend(award);
        crate::world::fame::fame(w, i) - base
    };
    // An All-America season two years back: 0.05, nothing for "latest".
    let d = add(&mut w, Some((last - 1, Division::FBS)), None);
    assert!((d - 0.05).abs() < 1e-9, "{d}");
    // Last season's FCS All-America: half the season, and 0.1 as the latest.
    let d = add(&mut w, Some((last, Division::FCS)), None);
    assert!((d - (0.075 + 0.1)).abs() < 1e-9, "{d}");
    // Last season's Heisman: 0.15 more, and the latest becomes 0.2.
    let d = add(&mut w, None, Some((last, "Heisman Trophy")));
    assert!((d - (0.225 + 0.2)).abs() < 1e-9, "{d}");
    // More honors stop at 0.25 for the seasons.
    let d = add(&mut w, None, Some((last - 1, "Davey O'Brien Award")));
    assert!((d - (0.25 + 0.2)).abs() < 1e-9, "{d}");
    // High schoolers read no college honors.
    w.people[i].stage = Stage::HighSchool;
    let hs = crate::world::fame::fame(&w, i);
    w.stats.honors.remove(&id);
    assert_eq!(hs, crate::world::fame::fame(&w, i));
}
