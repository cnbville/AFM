//! Simulates one full US high school class (250,000 freshmen) from age 14 to
//! the end of every pro career, and checks the outcomes against real-world
//! rates. Bands are wide enough to be seed-stable but tight enough to fail if
//! the model drifts into fantasy. (Ported from the TypeScript model's tests.)

use std::sync::LazyLock;

use afm_engine::career::{
    CohortOptions, CohortResult, CollegeLevel, PortalOutcome, TransferReason, draft_round, simulate_cohort,
};
use afm_engine::player::{Level, PlayingTime};
use afm_engine::positions::Position;
use afm_tools::stats::{Summary, median_last_age, summarize};

static CLASS: LazyLock<(CohortResult, Summary)> = LazyLock::new(|| {
    let result = simulate_cohort(CohortOptions::new(250_000, "national-class"));
    let s = summarize(&result);
    (result, s)
});

fn s() -> &'static Summary {
    &CLASS.1
}
fn result() -> &'static CohortResult {
    &CLASS.0
}

#[test]
fn develops_high_schoolers_steadily() {
    let s = s();
    assert!(s.hs.freshman_overall.mean > 15.0 && s.hs.freshman_overall.mean < 32.0);
    assert!(s.hs.senior_overall.mean > s.hs.freshman_overall.mean + 5.0);
    // Five-star seniors are far ahead of their class but not yet pro-ready.
    assert!(s.hs.five_star_senior_overall > s.hs.senior_overall.p99);
    assert!(s.hs.five_star_senior_overall < s.draft.rookie_overall.median);
}

#[test]
fn gives_four_and_five_stars_only_to_players_who_dominate_high_school() {
    let r = &s().recruiting;
    // Five stars need a top-100 senior film; with athlete-first positions the
    // 100th senior on film can rate 85 among seniors (seen on this class).
    assert!(r.five_star_senior_min >= 85, "{r:?}");
    // Almost none of the top 500-odd seniors goes unranked or ends up below FCS,
    // and few of the national top 100 miss out on four stars.
    assert!(r.top_seniors_unranked <= 2);
    assert!((r.top_seniors_at_d2_or_none as f64) < r.top_seniors as f64 * 0.03);
    assert!(r.top100_seniors_below_four_stars < 15);
    assert_eq!(r.blue_chip_specialists, 0);
}

#[test]
fn still_lets_staffs_sign_unranked_players_they_like_on_film() {
    // Film line C gives two stars to most of the top 6,500 seniors, so fewer signees are unranked.
    assert!(s().recruiting.unranked_signed_fbs_or_fcs > 40);
    assert!(s().recruiting.level80_seniors_with_no_offer < 40);
}

#[test]
fn judges_each_player_against_his_own_position() {
    let r = &s().recruiting;
    // Quarterbacks mature late but are as likely as anyone to be among the best
    // seniors. Only about seven are expected among the top 120, so the band is wide.
    assert!(r.qb_top_senior_ratio > 0.4 && r.qb_top_senior_ratio < 2.5);
    assert!(r.blue_chip_qbs > 8);
    assert!(r.five_star_positions >= 6);
}

#[test]
fn has_off_field_and_academic_trouble_in_college_at_believable_rates() {
    let t = &s().trouble;
    assert!(t.per_college_season > 0.015 && t.per_college_season < 0.05);
    // Some dismissed players leave the game, the rest start over a level down.
    assert!(t.left_football > 0);
    assert!(s().small_schools.transfer_routes.get("FBS>FCS").copied().unwrap_or(0) > 0);
    // A second suspension at the same school is a dismissal.
    assert_eq!(t.suspended_twice_at_one_school, 0);
}

#[test]
fn drafts_recruits_at_realistic_rates_by_star_rating() {
    let r = &s().draft_rate_by_stars;
    assert!(r.five > 0.4 && r.five < 0.85, "{r:?}");
    assert!(r.four > 0.12 && r.four < 0.4);
    assert!(r.three > 0.02 && r.three < 0.1);
    assert!(r.two < r.three);
    assert!(r.unranked < r.two);
}

#[test]
fn still_finds_a_share_of_draftees_among_low_rated_and_small_school_players() {
    let d = &s().drafted_share_by_stars;
    assert!(d.five > 0.03 && d.five < 0.15, "{d:?}");
    assert!(d.four > 0.2 && d.four < 0.45);
    assert!(d.low_or_unrated > 0.1 && d.low_or_unrated < 0.4);
}

#[test]
fn rarely_drafts_d2_players_and_never_before_day_3() {
    let sm = &s().small_schools;
    assert!(s().drafted_share_by_stars.from_d2 < 0.04);
    if sm.d2_drafted > 0 {
        assert!(sm.d2_earliest_round.unwrap() >= 4);
    }
    assert!(sm.d2_udfa > sm.d2_drafted);
    assert!(s().drafted_share_by_stars.from_fcs > 0.01 && s().drafted_share_by_stars.from_fcs < 0.12);
    // Standouts move up through the transfer portal instead.
    assert!(sm.players_moved_up > 100);
    assert!(sm.drafted_after_moving_up > 0);
}

#[test]
fn drafts_imperfectly_without_wild_first_round_reaches() {
    // Real drafts take 1-3 first-rounders a year from outside the consensus top 50.
    assert!(s().draft.first_rounders_outside_top50 <= 5);
}

#[test]
fn makes_a_99_level_rating_as_rare_as_ea_college_football_does() {
    let l = &s().level_ratings;
    // EA rated nobody 99 at launch in College Football 25 or 26 and one player in 27.
    assert!(l.hs_seniors99 <= 2, "{l:?}");
    assert!(l.college_seasons99 <= 2);
    // About ten players a season at 95+, and about a hundred at 90+.
    assert!(l.hs_seniors95plus > 3 && l.hs_seniors95plus < 25);
    assert!(l.hs_seniors90plus > 50 && l.hs_seniors90plus < 200);
    assert!(l.college_seasons95plus > 5 && l.college_seasons95plus < 35);
    assert!(l.college_seasons90plus > 70 && l.college_seasons90plus < 160);
    // A typical FBS player rates in the low 70s, as on an EA roster.
    assert!(l.fbs_season_median > 65.0 && l.fbs_season_median < 78.0);
    assert!(l.five_star_senior_median > 85.0);
    assert!(l.five_star_senior_median > l.four_star_senior_median);
}

#[test]
fn produces_realistic_pro_careers() {
    let c = &s().nfl.career_length;
    assert!(c.all_signed > 2.5 && c.all_signed < 5.5, "{c:?}");
    assert!(c.drafted > 4.0 && c.drafted < 8.5);
    assert!(c.first_round > c.round7 + 3.0);
    assert!(c.udfa < c.drafted);
    // Kickers and punters last longer, but they age too. (George Blanda kicked at 48.)
    assert!(s().nfl.specialists.oldest_season <= 48);
    assert!(s().nfl.specialists.seasons_at_38_plus < 0.15);
}

#[test]
fn peaks_at_realistic_ages_by_position() {
    let p = &s().nfl.median_peak_age;
    assert!(p.all_but_kp >= 25.0 && p.all_but_kp <= 28.0, "{p:?}");
    assert!(p.rb <= 26.0);
    assert!(p.qb >= 28.0);
    assert!(p.qb > p.rb);
}

#[test]
fn keeps_elite_pros_rare() {
    let o = &s().nfl.season_overall;
    assert!(o.mean > 68.0 && o.mean < 80.0, "{o:?}");
    // A class holds only a handful of 90+ pros, so this share swings between
    // seeds (0.3% to 2.6% over eight classes, before and after athlete-first).
    assert!(o.share90plus > 0.002 && o.share90plus < 0.08, "{o:?}");
    let m = s().draft.rookie_overall.median;
    assert!(m > 62.0 && m < 76.0);
}

#[test]
fn treats_potential_as_dynamic_not_fixed_at_birth() {
    let p = &s().potential;
    assert!(p.initial_vs_exit_correlation_drafted > 0.3 && p.initial_vs_exit_correlation_drafted < 0.85, "{p:?}");
    assert!(p.drafted_rose_by8_plus_since14 > 0.3);
    assert!(p.collegians_fell_by8_plus_since14 > 0.005);
    assert!(p.young_regression_share > 0.003 && p.young_regression_share < 0.15);
    // A falling rating is never labelled a breakout, and by their mid-30s veterans have no upside left.
    assert_eq!(p.falling_breakouts, 0);
    // (The TypeScript model gave 0.61-1.06 over six seeds; the Rust engine 0.63-1.27 over nine.)
    assert!(s().nfl.veteran_potential_gap < 1.5);
}

#[test]
fn moves_players_up_across_and_down_through_the_portal_and_some_find_no_school() {
    let records = &result().records;
    for o in [PortalOutcome::Up, PortalOutcome::Sideways, PortalOutcome::Down, PortalOutcome::None] {
        assert!(records.iter().any(|r| r.portal.iter().any(|p| p.outcome == o)), "{o:?}");
    }
    // A lesser dismissal can mean a fresh start at a smaller FBS program.
    assert!(records.iter().any(|r| r.transfers.iter().any(|t| t.reason == TransferReason::DismissedLesser
        && t.from == CollegeLevel::FBS
        && t.to == CollegeLevel::FBS)));
}

#[test]
fn gives_early_picks_time_and_sends_cut_players_to_the_practice_squad_or_free_agency() {
    let res = result();
    let early: Vec<_> =
        res.records.iter().filter(|r| r.draft_pick.is_some_and(|p| draft_round(p, res.draft_slots) <= 2)).collect();
    let off_early = early
        .iter()
        .filter(|r| {
            r.player
                .history
                .iter()
                .filter(|h| h.level == Level::NFL)
                .take(2)
                .any(|h| h.playing_time == PlayingTime::Unsigned)
        })
        .count();
    assert!(off_early <= 3);
    assert!(res.records.iter().any(|r| r.practice_squad_seasons > 0 && r.nfl_seasons > 0));
    // Released players can sign with another team.
    assert!(res.records.iter().filter(|r| r.pro_teams.len() >= 3).count() > 20);
}

#[test]
fn drafts_kickers_and_punters_off_their_own_board_late() {
    let res = result();
    let specialists: Vec<_> =
        res.records.iter().filter(|r| r.draft_pick.is_some() && r.player.position.is_specialist()).collect();
    assert!(!specialists.is_empty() && specialists.len() < 12);
    for r in specialists {
        assert!(draft_round(r.draft_pick.unwrap(), res.draft_slots) >= 4);
    }
}

#[test]
fn ends_running_back_careers_sooner_than_quarterback_careers() {
    let records = &result().records;
    assert!(median_last_age(records, Position::RB) < median_last_age(records, Position::QB) - 2.0);
}

#[test]
fn is_reproducible_from_a_seed() {
    let a = summarize(&simulate_cohort(CohortOptions::new(3000, "repeat")));
    let b = summarize(&simulate_cohort(CohortOptions::new(3000, "repeat")));
    assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
}

#[test]
fn gives_the_same_class_on_one_core_as_on_many() {
    let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let a = one.install(|| summarize(&simulate_cohort(CohortOptions::new(5000, "threads"))));
    let b = summarize(&simulate_cohort(CohortOptions::new(5000, "threads")));
    assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
}

#[test]
fn places_athletes_in_real_position_shares() {
    // Athlete first (topic 5.2): nobody is born at a position, yet coaches
    // filling rosters by fit and need give the real high school shares, and
    // the draft comes out near the real position mix (2000-2025).
    let records = &result().records;
    let n = records.len() as f64;
    let total: f64 = Position::ALL.iter().map(|p| p.profile().share).sum();
    let real = afm_tools::stats::real_draft_shares();
    let drafted: Vec<_> = records.iter().filter(|r| r.draft_pick.is_some()).collect();
    for pos in Position::ALL {
        let hs = records.iter().filter(|r| r.player.position_history[0].1 == pos).count() as f64 / n;
        let target = pos.profile().share / total;
        assert!(hs > 0.6 * target && hs < 1.4 * target, "{pos}: {hs:.3} of freshmen, roster share {target:.3}");
        if !pos.is_specialist() {
            let d = drafted.iter().filter(|r| r.player.position == pos).count() as f64 / drafted.len() as f64;
            let r = real[pos.index()];
            assert!(d > 0.45 * r && d < 1.8 * r, "{pos}: {d:.3} of picks, real {r:.3}");
        }
    }
    // Positions change: growth, college staffs and pro scouts move players.
    assert!(records.iter().filter(|r| r.player.position_history.len() > 1).count() as f64 > 0.05 * n);
}

#[test]
fn gives_drafted_players_their_positions_real_bodies() {
    let records = &result().records;
    for pos in Position::ALL.into_iter().filter(|p| !p.is_specialist()) {
        let bodies: Vec<_> = records
            .iter()
            .filter(|r| r.draft_pick.is_some() && r.player.position == pos)
            .map(|r| afm_engine::body::body_now(&r.player))
            .collect();
        let (hm, _, wm, _) = afm_engine::body::combine_size(pos);
        let h = bodies.iter().map(|b| b.height).sum::<f64>() / bodies.len() as f64;
        let w = bodies.iter().map(|b| b.weight).sum::<f64>() / bodies.len() as f64;
        assert!((h - hm).abs() < 2.0, "{pos}: drafted height {h:.1}, combine {hm:.1}");
        assert!((w - wm).abs() < 20.0, "{pos}: drafted weight {w:.0}, combine {wm:.0}");
    }
}
