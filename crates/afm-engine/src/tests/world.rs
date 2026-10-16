//! The world's building blocks: data, grades, coach reputation, backgrounds.

use super::mean;
use crate::attributes::Attr;
use crate::player::{ProspectOptions, generate_prospect};
use crate::rng::Rng;
use crate::world::background::{BackgroundSeed, apply_background, generate_background, generate_family, hardship};
use crate::world::coach::{CoachTier, TitleKind, generate_coach};
use crate::world::data::{COLLEGES, Division, STATES, hs_nil};
use crate::world::high_school::transfer_rule;
use crate::world::program::{GradeCat, GradeSpeed, letter, move_grade};

#[test]
fn real_data_loads() {
    let count = |d: Division| COLLEGES.iter().filter(|c| c.division == d).count();
    assert_eq!((count(Division::FBS), count(Division::FCS), count(Division::D2)), (138, 127, 160));
    assert_eq!(STATES.len(), 51);
    // 46 states plus DC allow high school NIL (On3, May 2026).
    assert_eq!(STATES.iter().filter(|s| hs_nil(&s.code)).count(), 47);
    assert!(!hs_nil("AL") && hs_nil("CA"));
    let ca = STATES.iter().find(|s| s.code == "CA").unwrap();
    assert_eq!(ca.powerhouses.len(), 5);
}

#[test]
fn two_conference_titles_move_title_chances_far() {
    // Topic 7: no EA-style C- to C+ after back-to-back titles.
    let mut g = 70.0;
    assert_eq!(letter(g), "C-");
    for _ in 0..2 {
        g = move_grade(g, 93.0, GradeCat::TitleChances.speed(), false);
    }
    assert!(g >= 87.0, "title chances only reached {g:.1} ({})", letter(g));
    // A steady grade like tradition barely moves in two years.
    let mut t = 70.0;
    for _ in 0..2 {
        t = move_grade(t, 93.0, GradeCat::Tradition.speed(), false);
    }
    assert!(t < 78.0);
    assert_eq!(GradeCat::ConferenceStrength.speed(), GradeSpeed::Realignment);
}

#[test]
fn big_results_boost_reputation_massively_and_assistants_move_slower() {
    let mut rng = Rng::new("coach");
    let base = generate_coach(0, "A".into(), crate::world::ethnicity::Ethnicity::White, CoachTier::Fcs, 45, &mut rng);
    let mut champ = base.clone();
    let mut plain = base.clone();
    let mut assistant = base.clone();
    champ.update_reputation(0.2, TitleKind::Conference, 0.0, true, 0.5);
    plain.update_reputation(0.2, TitleKind::None, 0.0, true, 0.5);
    assistant.update_reputation(0.2, TitleKind::Conference, 0.0, false, 0.5);
    assert!(champ.reputation - plain.reputation >= 2.0);
    assert!(assistant.reputation - base.reputation < champ.reputation - base.reputation);
    // Recency bias: last season outweighs the one before.
    let mut a = base.clone();
    let mut b = base.clone();
    a.recent = vec![-0.2, 0.2];
    b.recent = vec![0.2, -0.2];
    assert!(a.hiring_buzz() > b.hiring_buzz() + 2.0);
}

#[test]
fn coaches_lose_sharpness_with_age_even_at_their_ceiling() {
    // Dynasty item 11.14: a coach who has stopped learning still declines from his late 50s.
    let mut rng = Rng::new("old coach");
    let mut c = generate_coach(0, "A".into(), crate::world::ethnicity::Ethnicity::White, CoachTier::Fbs, 50, &mut rng);
    c.learning_ceiling = 1.0;
    c.decline_from = 58.0;
    c.decline_rate = 0.2;
    let start = c.ability().head_coach_value();
    for _ in 0..7 {
        c.age_one_year(None, &mut rng);
    }
    let at_57 = c.ability().head_coach_value();
    for _ in 0..8 {
        c.age_one_year(None, &mut rng);
    }
    let at_65 = c.ability().head_coach_value();
    assert!((at_57 - start).abs() < 0.5, "no decline before 58: {start:.2} -> {at_57:.2}");
    assert!(at_57 - at_65 > 1.0, "decline from 58 to 65: {at_57:.2} -> {at_65:.2}");
}

#[test]
fn background_marks_are_noteworthy_not_driving() {
    let mut rng = Rng::new("backgrounds");
    let mut poor = Vec::new();
    let mut rich = Vec::new();
    let mut grit_hard = Vec::new();
    let mut grit_easy = Vec::new();
    for k in 0..4000 {
        let resources = if k % 2 == 0 { 0.1 } else { 0.9 };
        let money = if k % 2 == 0 { 0.15 } else { 0.85 };
        let seed = BackgroundSeed { area_money: money, area_safety: money, school_resources: resources };
        let mut p = generate_prospect(&mut rng, ProspectOptions { talent_z: Some(1.5), ..Default::default() });
        let before = p.potential();
        let (mut bg, structure) = generate_background(seed, &mut rng);
        let family = generate_family(&bg, Some(structure), &mut rng);
        apply_background(&mut p, &mut bg, &family, &mut rng);
        let lift = p.potential() - before;
        if k % 2 == 0 {
            poor.push(lift);
        } else {
            rich.push(lift);
        }
        if hardship(&bg) > 0.6 {
            grit_hard.push(bg.grit);
        } else if hardship(&bg) < 0.3 {
            grit_easy.push(bg.grit);
        }
        assert!(p.ceilings[Attr::Speed] <= 99.0);
    }
    let gap = mean(&poor) - mean(&rich);
    // Kids without resources keep a little more hidden upside: small, not decisive.
    assert!(gap > 0.3 && gap < 3.0, "upside gap {gap:.2}");
    assert!(mean(&grit_hard) > mean(&grit_easy) + 0.8);
}

#[test]
fn state_transfer_rules_read_from_summaries() {
    assert_eq!(transfer_rule("Transfers sit the first 50% of regular-season varsity games").sit_out, 0.5);
    assert_eq!(transfer_rule("transfers after the deadline sit 365 days").sit_out, 1.0);
    assert!(transfer_rule("One-time transfer allowed without sitting").free_once);
}

#[test]
fn heights_read_like_the_combine() {
    use crate::body::height_text;
    assert_eq!(height_text(72.75), "6'0 3/4\"");
    assert_eq!(height_text(75.5), "6'3 1/2\"");
    assert_eq!(height_text(71.99), "6'0\"");
    assert_eq!(height_text(69.125), "5'9 1/8\"");
}

#[test]
fn class_counts_read_the_leading_number() {
    // Florida's cell reads "7 (plus Independent League and FIT)"; DC's is
    // blank in the source and keeps the default of 4.
    let classes = |code: &str| STATES.iter().find(|s| s.code == code).unwrap().number_of_classes;
    assert_eq!((classes("FL"), classes("DC"), classes("TX"), classes("CA")), (7, 4, 12, 15));
    assert!(STATES.iter().all(|s| (1..=15).contains(&s.number_of_classes)));
}

#[test]
fn walk_ons_count_against_the_limit_only_under_the_house_roster_limit() {
    // F17: before 2025 the limits count players on aid, so walk-ons sit
    // outside them; the House settlement's 105 counts the whole roster,
    // except at the FCS programs that stayed out of it.
    use crate::world::ncaa::whole_roster_limit;
    use crate::world::program::program_from_row;
    let mut rng = Rng::new(7);
    let mut program = |name: &str| {
        let (i, row) = COLLEGES.iter().enumerate().find(|(_, c)| c.name == name).unwrap();
        program_from_row(i, row, 7, &mut rng)
    };
    let (fbs, fcs, out) = (program("Alabama"), program("Montana"), program("Holy Cross"));
    assert!(!whole_roster_limit(&fbs, 2024) && whole_roster_limit(&fbs, 2025));
    assert!(!whole_roster_limit(&fcs, 2024) && whole_roster_limit(&fcs, 2025));
    assert!(!whole_roster_limit(&out, 2025));
}

#[test]
fn notes_read_an_before_a_vowel_sound() {
    use crate::text::a_or_an;
    for w in ["EDGE", "OT", "IOL", "LB", "S", "RB", "LS", "FCS", "8", "11", "18", "80", "800", "11,000", "unranked"] {
        assert_eq!(a_or_an(w), "an", "{w}");
    }
    for w in ["QB", "WR", "TE", "DT", "CB", "K", "P", "CFP", "UCLA", "1", "10", "48", "180", "1,800", "Utah", "backup"]
    {
        assert_eq!(a_or_an(w), "a", "{w}");
    }
}

/// A tiny world's people, under one season's rules, and an FBS program.
fn college_world(season: i32) -> (crate::world::World, usize) {
    use crate::world::{World, WorldOptions};
    let mut w = World::empty(WorldOptions { scale: 0.001, ..Default::default() });
    w.add_class_aged(18);
    w.rules = w.changes.rules(season);
    let c = w.programs.iter().position(|p| p.division == Division::FBS).unwrap();
    (w, c)
}

#[test]
fn the_redshirt_role_goes_only_below_the_travel_squad() {
    // F14: six freshmen at quarterback; the travel squad carries three.
    use crate::player::PlayingTime as Pt;
    use crate::positions::Position;
    use crate::world::season::{depth_chart, travel_slots};
    assert_eq!(travel_slots(Position::QB), 3);
    let mut entries: Vec<(usize, Position, f64)> = (0..6).map(|i| (i, Position::QB, 80.0 - i as f64)).collect();
    let roles = |entries: &mut [(usize, Position, f64)], sits: &dyn Fn(usize) -> bool| {
        let mut r = depth_chart(entries, true, &|_| true, sits);
        r.sort_by_key(|x| x.0);
        r.into_iter().map(|x| x.1).collect::<Vec<_>>()
    };
    let all = roles(&mut entries, &|_| false);
    assert_eq!(all, [Pt::Starter, Pt::Rotation, Pt::Backup, Pt::Redshirt, Pt::Redshirt, Pt::Redshirt]);
    // A quarterback sitting the season out takes no travel place.
    let one_sits = roles(&mut entries, &|i| i == 1);
    assert_eq!(one_sits, [Pt::Starter, Pt::Rotation, Pt::Backup, Pt::Backup, Pt::Redshirt, Pt::Redshirt]);
}

#[test]
fn camp_redshirts_sit_unless_their_position_is_short() {
    // F14: before 2018 a camp redshirt sits; from 2018 he plays up to four
    // games; either way he plays when absences leave his position short.
    use crate::player::PlayingTime as Pt;
    use crate::positions::Position;
    for (season, games, short, held) in
        [(2016, 0, false, true), (2016, 0, true, false), (2019, 2, false, false), (2019, 4, false, true)]
    {
        let (mut w, c) = college_world(season);
        for i in 0..3 {
            let p = &mut w.people[i];
            crate::athlete::set_position(&mut p.player, Position::QB);
            p.stage = crate::world::Stage::College;
            p.college = Some(c);
            p.last_role = Some(if i == 2 { Pt::Redshirt } else { Pt::Backup });
            p.redshirt_used = i == 2;
            p.cal.count_pending = i != 2;
        }
        w.programs[c].roster = vec![0, 1, 2];
        let mut cs = crate::world::college::new_season(&w);
        let rs = w.people[2].player.id;
        cs.snaps.insert(rs, (10, games));
        if short {
            cs.out.insert(w.people[0].player.id, "suspended");
        }
        let got = crate::world::college::held_redshirts(&w, &cs);
        assert_eq!(got.contains(&rs), held, "{season}, {games} games, short {short}");
    }
}

#[test]
fn a_camp_redshirt_who_plays_has_a_season_on_his_row() {
    // F1: before 2018 any game costs the year and the season was a
    // backup's; from 2018 four games or fewer keep it and the row stays a
    // redshirt row. His play and an injury's missed games show either way.
    use crate::player::{DevelopmentEvent as Ev, Level, PlayingTime as Pt};
    for (season, played, burned) in [(2016, 7, true), (2019, 3, false), (2019, 6, true)] {
        let (mut w, c) = college_world(season);
        let mut cs = crate::world::college::new_season(&w);
        cs.on_engine = true;
        cs.results[c].wins = 8;
        cs.results[c].losses = 4;
        let p = &mut w.people[0];
        p.stage = crate::world::Stage::College;
        p.college = Some(c);
        let ctx = crate::development::SeasonContext::new(Level::FBS, 12.0, Pt::Redshirt);
        crate::development::develop_season(&mut p.player, &ctx, &mut p.rng);
        let row = p.player.history.last_mut().unwrap();
        row.events.add(Ev::MajorInjury);
        row.games_missed = 0;
        let age = row.age;
        p.last_role = Some(Pt::Redshirt);
        p.redshirt_used = true;
        cs.snaps.insert(p.player.id, (40, played));
        // A freshman who never got in keeps his redshirt year in every era.
        let q = &mut w.people[1];
        q.stage = crate::world::Stage::College;
        q.cal.count_pending = true;
        w.college = Some(cs);
        crate::world::college::settle_eligibility(&mut w);
        let p = &w.people[0];
        let row = p.player.history.last().unwrap();
        assert!(row.performance.is_some(), "{season}");
        assert_eq!(row.playing_time, if burned { Pt::Backup } else { Pt::Redshirt });
        assert_eq!(p.redshirt_used, !burned);
        assert!((1..=12 - played).contains(&row.games_missed), "{} games missed", row.games_missed);
        let note = p.notes.last().unwrap();
        assert!(note.contains(&format!("at {age}")), "{note}");
        assert!(w.people[1].redshirt_used && w.people[1].college_seasons == 0);
    }
}
