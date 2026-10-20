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
    // Division III, the NAIA and the junior colleges (NJCAA and CCCAA) in 2026.
    assert_eq!((count(Division::D3), count(Division::NAIA), count(Division::JUCO)), (244, 96, 113));
    assert_eq!(STATES.len(), 51);
    // 46 states plus DC allow high school NIL (On3, May 2026).
    assert_eq!(STATES.iter().filter(|s| hs_nil(&s.code)).count(), 47);
    assert!(!hs_nil("AL") && hs_nil("CA"));
    let ca = STATES.iter().find(|s| s.code == "CA").unwrap();
    assert_eq!(ca.powerhouses.len(), 5);
}

#[test]
fn college_history_matches_real_counts() {
    use crate::world::data::DROPPED_COLLEGES;
    use crate::world::program::{Program, program_from_row};
    let rows = COLLEGES.iter().map(|r| (r, false)).chain(DROPPED_COLLEGES.iter().map(|r| (r, true)));
    let programs: Vec<Program> = rows
        .enumerate()
        .map(|(i, (r, dropped))| {
            let mut p = program_from_row(i, r, 1, &mut Rng::new(1));
            p.set_history(dropped);
            p
        })
        .collect();
    // Division I-A had 105 teams in 1984 and 119 in 2006; FBS had 136 in 2025.
    let fbs = |y: i32| programs.iter().filter(|p| p.real_status(y).is_some_and(|s| s.0 == Division::FBS)).count();
    assert_eq!((fbs(1984), fbs(2006), fbs(2025)), (105, 119, 136));
    let find = |n: &str| programs.iter().find(|p| p.real_name == n).unwrap();
    assert_eq!(find("Boise State").real_status(1990).map(|s| s.0), Some(Division::FCS));
    // UAB dropped football after 2014 and came back in 2017.
    let uab = find("UAB");
    assert!(uab.plays(2014) && !uab.plays(2015) && !uab.plays(2016) && uab.plays(2017));
    assert!(!find("Georgia State").plays(2009) && find("Georgia State").plays(2010));
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

#[test]
fn counts_read_singular_and_plural() {
    use crate::world::news::count;
    assert_eq!(count(1u32, "season"), "1 season");
    assert_eq!(count(0u32, "signee"), "0 signees");
    assert_eq!(count(4u32, "more year"), "4 more years");
}

#[test]
fn the_uncapped_2010_year_keeps_its_books_and_six_year_free_agency() {
    // Trial 7 finding 5: 2010 had no cap, but teams kept their books and
    // free agency needed six accrued seasons.
    use crate::world::pro::money::{cap_books, capped, ufa_seasons};
    assert!(!capped(2010) && cap_books(2010));
    assert!(capped(2009) && capped(2011) && !cap_books(1993));
    assert_eq!((ufa_seasons(2009), ufa_seasons(2010), ufa_seasons(2011)), (4, 6, 4));
}

/// A tiny world's people, under one season's rules, and an FBS program.
fn college_world(season: i32) -> (crate::world::World, usize) {
    use crate::world::{World, WorldOptions};
    let mut w = World::empty(WorldOptions { scale: 0.001, ..Default::default() });
    // The college tests work on the class alone: the generated NFL goes.
    w.people.retain(|p| p.stage != crate::world::Stage::Pro);
    for club in w.clubs.iter_mut() {
        club.roster.clear();
    }
    crate::world::pro::rebuild(&mut w);
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
    assert_eq!(travel_slots(Position::QB, 76), 3);
    let mut entries: Vec<(usize, Position, f64)> = (0..6).map(|i| (i, Position::QB, 80.0 - i as f64)).collect();
    let roles = |entries: &mut [(usize, Position, f64)], sits: &dyn Fn(usize) -> bool| {
        let mut r = depth_chart(entries, Some(76), &|_| true, sits);
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

#[test]
fn a_redshirt_who_costs_nothing_fills_a_hole_before_one_whose_year_it_would_take() {
    // Review fix: one regular quarterback is out, so the two-deep is one
    // short. The redshirt who costs nothing today plays and the other sits,
    // whichever is better: from 2018 the one with games left under the
    // four-game rule, before it the one whose year is already gone.
    use crate::player::PlayingTime as Pt;
    use crate::positions::Position;
    for (season, best_games, other_games) in [(2019, 4, 2), (2016, 0, 1)] {
        let (mut w, c) = college_world(season);
        for i in 0..4 {
            let p = &mut w.people[i];
            crate::athlete::set_position(&mut p.player, Position::QB);
            p.stage = crate::world::Stage::College;
            p.college = Some(c);
            p.last_role = Some(if i >= 2 { Pt::Redshirt } else { Pt::Backup });
            p.redshirt_used = i >= 2;
            p.cal.count_pending = i < 2;
        }
        w.programs[c].roster = vec![0, 1, 2, 3];
        let (best, other) = if w.people[2].player.overall() >= w.people[3].player.overall() { (2, 3) } else { (3, 2) };
        let mut cs = crate::world::college::new_season(&w);
        let (best, other) = (w.people[best].player.id, w.people[other].player.id);
        cs.snaps.insert(best, (10, best_games));
        cs.snaps.insert(other, (5, other_games));
        cs.out.insert(w.people[0].player.id, "suspended");
        let held = crate::world::college::held_redshirts(&w, &cs);
        assert!(held.contains(&best) && !held.contains(&other), "{season}: held {held:?}");
    }
}

#[test]
fn an_early_enrollees_notes_give_the_age_of_his_first_college_row() {
    // Review fix (F2, F15): on campus before his first college row, every
    // note (enrollment, spring, spring term, NIL, portal) gives the age that
    // row will carry; once it exists, its own age.
    use crate::development::{SeasonContext, develop_season};
    use crate::player::{Level, PlayingTime as Pt};
    let (mut w, _) = college_world(2019);
    let p = &mut w.people[0];
    develop_season(&mut p.player, &SeasonContext::new(Level::HS, 9.0, Pt::Starter), &mut p.rng);
    let hs_age = p.player.history.last().unwrap().age;
    assert_eq!(p.row_age(), hs_age);
    p.stage = crate::world::Stage::College;
    assert_eq!(p.row_age(), hs_age + 1);
    develop_season(&mut p.player, &SeasonContext::new(Level::FBS, 12.0, Pt::Redshirt), &mut p.rng);
    assert_eq!(p.row_age(), hs_age + 1);
    assert_eq!(p.player.age, hs_age + 2);
}

#[test]
fn a_player_who_withdraws_from_the_draft_gives_up_his_all_star_invite() {
    // Review fix: back in school with eligibility left, he plays no
    // all-star game that winter.
    use crate::world::recruiting::{Declare, declarations};
    let (mut w, c) = college_world(2018);
    w.year = 2018;
    for p in w.people.iter_mut() {
        p.stage = crate::world::Stage::Declared;
        p.college = Some(c);
        p.class_of = 2017;
        p.college_seasons = 2;
        p.cal.draft_read = Some(crate::world::DraftRead { grade: 40.0, score: 0.0, round: 8 });
        p.cal.all_star = Some("Senior Bowl");
    }
    declarations(&mut w, Declare::Withdraw);
    let back = w.people.iter().filter(|p| p.stage == crate::world::Stage::College).count();
    assert!(back > 0, "nobody withdrew");
    for p in &w.people {
        assert_eq!(p.cal.all_star.is_some(), p.stage == crate::world::Stage::Declared);
    }
}

#[test]
fn all_star_rosters_are_named_after_the_title_games_and_the_withdrawal_day() {
    // Review fix (F7): the one roster day comes after the FBS and FCS title
    // games, after the withdrawal day whenever the first game allows it,
    // and never after the first game held.
    use crate::calendar::YearDates;
    use crate::world::clock::all_star_roster_day;
    use crate::world::recruiting::all_star_held;
    for season in 1990..=2031 {
        let y = YearDates::new(season);
        let first = [
            (y.senior_bowl, "Senior Bowl"),
            (y.shrine_bowl, "East-West Shrine Bowl"),
            (y.hbcu_bowl, "HBCU Legacy Bowl"),
            (y.nflpa_bowl, "NFLPA Collegiate Bowl"),
        ]
        .into_iter()
        .filter(|g| all_star_held(g.1, season + 1))
        .map(|g| g.0)
        .min()
        .unwrap();
        let day = all_star_roster_day(&y);
        let fcs_final = *y.fcs_rounds.last().unwrap();
        assert!(day > y.title_game && day > fcs_final, "{season}: {}", day.pretty());
        assert!(day <= first && day >= first.add(-7), "{season}: {} vs {}", day.pretty(), first.pretty());
        if first.add(-1) >= y.declare_withdraw {
            assert!(day >= y.declare_withdraw, "{season}: {}", day.pretty());
        }
    }
}

#[test]
fn a_played_redshirt_season_names_its_form() {
    // Review fix (F1): every camp redshirt who played draws the year's form,
    // kept four-game years too, and the row names a slump or a breakout, so
    // a slump list gives the reason. A lost season drawn at camp rules out
    // both; a major injury rules out a breakout.
    use crate::player::{DevelopmentEvent as Ev, Level, PlayingTime as Pt};
    for (season, played) in [(2016, 7), (2019, 3)] {
        let (mut w, c) = college_world(season);
        let mut cs = crate::world::college::new_season(&w);
        cs.on_engine = true;
        cs.results[c].wins = 8;
        cs.results[c].losses = 4;
        let ctx = crate::development::SeasonContext::new(Level::FBS, 12.0, Pt::Redshirt);
        for p in w.people.iter_mut() {
            p.stage = crate::world::Stage::College;
            p.college = Some(c);
            crate::development::develop_season(&mut p.player, &ctx, &mut p.rng);
            p.last_role = Some(Pt::Redshirt);
            p.redshirt_used = true;
            cs.snaps.insert(p.player.id, (20, played));
        }
        w.college = Some(cs);
        crate::world::college::settle_eligibility(&mut w);
        let rows: Vec<_> = w.people.iter().map(|p| p.player.history.last().unwrap()).collect();
        let slumps = rows.iter().filter(|h| h.events.has(Ev::Slump)).count();
        assert!((3..=40).contains(&slumps), "{season}: {slumps} slumps in {}", rows.len());
        for h in &rows {
            let form = h.events.has(Ev::Slump) || h.events.has(Ev::Breakout);
            assert!(!(form && h.events.has(Ev::LostSeason)), "{season}: form in a lost season");
            assert!(!(h.events.has(Ev::Breakout) && h.events.has(Ev::MajorInjury)));
            assert_eq!(h.playing_time, if played > 4 || season < 2018 { Pt::Backup } else { Pt::Redshirt });
        }
    }
}

#[test]
fn a_long_save_keeps_its_calendar_and_money() {
    // Bug check (build 9): a save can run far past the trials' last year
    // (2045). Every year to 2200 has its dates in order, an era's rule
    // book and a cap that grows.
    use crate::calendar::YearDates;
    use crate::calendar::rules::RuleBook;
    use crate::world::pro::money;
    let mut cap = 0.0;
    for season in 2026..=2200 {
        let y = YearDates::new(season);
        assert!(y.start < y.nfl_kickoff && y.nfl_kickoff < y.super_bowl, "{season}");
        assert!(y.combine_invites < y.combine_start && y.combine_start < y.draft[0].date, "{season}");
        let r = RuleBook::era(season);
        assert!(r.nfl_games >= 16, "{season}");
        let c = money::price_cap(season);
        assert!(c.is_finite() && c > cap, "{season}: cap {c}");
        cap = c;
    }
}

#[test]
fn a_big_deal_pays_no_more_than_a_real_first_year() {
    // Trial 9 bug 11: Mckenzie Tupou Jr.'s five years at $62M carried a
    // $111.5M signing bonus, 52.6% of the 2025 cap in year one. The excess
    // bonus becomes guaranteed base in the later years.
    use crate::world::pro::money;
    let cap = money::price_cap(2025);
    let mut t = money::Terms::flat(5, 62.03e6);
    t.bonus_share = 0.36;
    t.base_guarantee = 0.19;
    t.ramp = 0.06;
    let gtd = t.guaranteed();
    assert!(t.first_year_cash() > 0.5 * cap);
    t.limit_first_year(0.38 * cap);
    assert!((t.guaranteed() - gtd).abs() < 1.0, "the guarantee changed");
    let k = money::build(0, money::Kind::Veteran, 2025, &t);
    let y1 = &k.years[0];
    let cash = y1.pay() + y1.bonus_paid;
    assert!(cash <= 0.38 * cap + 1.0 && cash > 0.37 * cap, "year one {:.1}% of the cap", 100.0 * cash / cap);
    assert!((k.total() - t.total()).abs() < 1.0, "the money changed");
    // A deal under the line is left alone.
    let mut s = money::Terms::flat(4, 20.0e6);
    s.bonus_share = 0.3;
    s.limit_first_year(0.38 * cap);
    assert_eq!(s.bonus_share, 0.3);
}
