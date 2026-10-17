//! Headline statistics of a simulated national class, checked against
//! real-world rates by the tests and printed by `calibrate`.

use std::collections::{BTreeMap, HashSet};

use afm_engine::career::{CareerRecord, CohortResult, CollegeLevel, ProExit, Transfer, draft_round};
use afm_engine::debug;
use afm_engine::player::{DevelopmentEvent as Ev, Level, PlayingTime, SeasonRecord};
use afm_engine::positions::Position;
use serde::Serialize;

pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() { f64::NAN } else { xs.iter().sum::<f64>() / xs.len() as f64 }
}

pub fn median(xs: &[f64]) -> f64 {
    quantile(xs, 0.5)
}

/// Linear-interpolated quantile.
pub fn quantile(xs: &[f64], q: f64) -> f64 {
    if xs.is_empty() {
        return f64::NAN;
    }
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let i = (s.len() - 1) as f64 * q;
    let (lo, hi) = (i.floor() as usize, i.ceil() as usize);
    s[lo] + (s[hi] - s[lo]) * (i - lo as f64)
}

pub fn correlation(xs: &[f64], ys: &[f64]) -> f64 {
    let (mx, my) = (mean(xs), mean(ys));
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in xs.iter().zip(ys) {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
    }
    sxy / (sxx * syy).sqrt()
}

fn round(x: f64, d: i32) -> f64 {
    let f = 10f64.powi(d);
    (x * f).round() / f
}

fn r3(x: f64) -> f64 {
    round(x, 3)
}

fn share(part: usize, whole: usize) -> f64 {
    part as f64 / whole as f64
}

fn moved_up(t: &Transfer) -> bool {
    t.to > t.from
}

pub fn is_drafted(r: &CareerRecord) -> bool {
    r.draft_pick.is_some()
}

/// Drafted or signed as an undrafted free agent, with a season on a roster.
pub fn signed_pro(r: &CareerRecord) -> bool {
    r.nfl_seasons > 0
}

fn is_specialist(r: &CareerRecord) -> bool {
    r.player.position.is_specialist()
}

fn senior(r: &CareerRecord) -> &SeasonRecord {
    &r.player.history[3]
}

fn senior_level(r: &CareerRecord) -> u8 {
    senior(r).level_rating.unwrap_or(0)
}

/// His last season before the pros.
fn last_amateur(r: &CareerRecord) -> &SeasonRecord {
    r.player.history.iter().rfind(|s| s.level != Level::NFL).unwrap()
}

/// NFL peak: the season he played at his best rating.
pub fn nfl_peak_age(r: &CareerRecord) -> Option<u32> {
    let mut best = -1.0;
    let mut age = None;
    for s in &r.player.history {
        if s.level == Level::NFL && s.overall_start > best {
            best = s.overall_start;
            age = Some(s.age);
        }
    }
    age
}

/// Players suspended twice at the same school, a second incident that should have ended his time there.
fn repeat_suspensions_at_one_school(r: &CareerRecord) -> bool {
    let mut last_level: Option<Level> = None;
    let mut suspensions = 0;
    let moves: HashSet<u32> = r.transfers.iter().map(|t| t.age + 1).collect();
    for s in &r.player.history {
        if !s.level.is_college() {
            continue;
        }
        if Some(s.level) != last_level || moves.contains(&s.age) {
            suspensions = 0;
        }
        last_level = Some(s.level);
        if s.events.has(Ev::Suspended) {
            suspensions += 1;
            if suspensions >= 2 {
                return true;
            }
        }
    }
    false
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Summary {
    pub players: usize,
    pub draft_slots: u32,
    pub rates: Rates,
    pub draft_rate_by_stars: StarRates,
    pub drafted_share_by_stars: DraftedShares,
    pub hs: Hs,
    pub draft: Draft,
    pub nfl: Nfl,
    pub small_schools: SmallSchools,
    pub recruiting: Recruiting,
    pub trouble: TroubleStats,
    pub level_ratings: LevelRatings,
    pub levels: Levels,
    /// Pros with 4+ seasons: his best season against his potential going into it.
    pub reaching_potential: Reach,
    /// The same against his potential on draft day.
    pub reaching_draft_day_potential: Reach,
    pub potential: PotentialStats,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Rates {
    pub college: f64,
    pub signed_fbs: f64,
    pub finished_at_fbs: f64,
    pub drafted: f64,
    pub signed_pro: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct StarRates {
    pub five: f64,
    pub four: f64,
    pub three: f64,
    pub two: f64,
    pub unranked: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct DraftedShares {
    pub five: f64,
    pub four: f64,
    pub three: f64,
    pub low_or_unrated: f64,
    pub from_small_schools: f64,
    pub from_fcs: f64,
    pub from_d2: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct MeanP99 {
    pub mean: f64,
    pub p99: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Hs {
    pub freshman_overall: MeanP99,
    pub senior_overall: MeanP99,
    pub five_star_senior_overall: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Spread {
    pub p10: f64,
    pub median: f64,
    pub p90: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Draft {
    /// Draftees' rating on draft day.
    pub rookie_overall: Spread,
    pub age: f64,
    /// First-round picks from outside the consensus top 50: reaches.
    pub first_rounders_outside_top50: usize,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct SeasonOverall {
    pub mean: f64,
    pub p10: f64,
    pub p90: f64,
    pub share90plus: f64,
    pub share80plus: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct CareerLength {
    pub drafted: f64,
    /// Drafted players and undrafted signings.
    pub all_signed: f64,
    pub first_round: f64,
    pub round7: f64,
    pub udfa: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct PeakAge {
    pub all_but_kp: f64,
    pub rb: f64,
    pub qb: f64,
    pub kp: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Specialists {
    pub signed: usize,
    pub seasons_at_38_plus: f64,
    pub oldest_season: u32,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Nfl {
    pub season_overall: SeasonOverall,
    pub career_length: CareerLength,
    pub median_peak_age: PeakAge,
    /// Upside veterans of 33-35 still show above their rating.
    pub veteran_potential_gap: f64,
    pub specialists: Specialists,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct SmallSchools {
    pub d2_drafted: usize,
    /// Earliest round a D2 player was drafted in (None if none was).
    pub d2_earliest_round: Option<u32>,
    pub d2_udfa: usize,
    pub fcs_drafted: usize,
    pub players_moved_up: usize,
    pub drafted_after_moving_up: usize,
    /// Every move, by route. Moves down follow a dismissal.
    pub transfer_routes: BTreeMap<String, usize>,
    pub drafted_signed_fcs: usize,
    pub drafted_signed_d2: usize,
    pub drafted_after_moving_up_from_d2: usize,
    pub drafted_after_moving_up_rounds1to3: usize,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Recruiting {
    pub five_star_senior_min: u8,
    pub top_seniors: usize,
    pub top_seniors_unranked: usize,
    pub top_seniors_at_d2_or_none: usize,
    /// QBs' share of the top 100-odd seniors, against their share of all players (1 = fair).
    pub qb_top_senior_ratio: f64,
    pub blue_chip_qbs: usize,
    pub five_star_positions: usize,
    /// National top-100-odd seniors (level 90+, kickers and punters aside) with fewer than four stars.
    pub top100_seniors_below_four_stars: usize,
    pub blue_chip_specialists: usize,
    pub unranked_signed_fbs_or_fcs: usize,
    /// Seniors around the national top 2,000 who got no college offer at all.
    pub level80_seniors_with_no_offer: usize,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct TroubleStats {
    pub per_college_season: f64,
    pub suspensions: usize,
    pub dismissals: usize,
    pub left_football: usize,
    pub five_stars_in_trouble: usize,
    pub suspended_twice_at_one_school: usize,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct LevelRatings {
    pub hs_seniors99: usize,
    pub hs_seniors95plus: usize,
    pub hs_seniors90plus: usize,
    pub five_star_senior_median: f64,
    pub four_star_senior_median: f64,
    pub college_seasons99: usize,
    pub hs_seasons99_any_age: usize,
    pub college_seasons95plus: usize,
    pub fbs_season_median: f64,
    pub best_d2_season: u8,
    pub college_seasons90plus: usize,
    pub drafted_final_college_median: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct PeakCounts {
    #[serde(rename = "97+")]
    pub p97: usize,
    #[serde(rename = "92+")]
    pub p92: usize,
    #[serde(rename = "85+")]
    pub p85: usize,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Levels {
    pub college_exits_rated65_plus: usize,
    pub college_exits_rated70_plus: usize,
    pub drafted_below65: f64,
    pub peak_counts: PeakCounts,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Reach {
    pub within2: f64,
    /// 90% of it or more, but more than 2 points short.
    pub reached90pct_plus: f64,
    pub below90pct: f64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct PotentialStats {
    pub initial_vs_exit_correlation_drafted: f64,
    pub initial_vs_exit_correlation_all: f64,
    pub drafted_rose_by8_plus_since14: f64,
    pub collegians_fell_by8_plus_since14: f64,
    pub young_regression_share: f64,
    /// Seasons tagged as a breakout although the rating fell.
    pub falling_breakouts: usize,
    /// Share of player seasons (18 and up) that lost more than a rating point
    /// (fix list topic 4: was under 0.3%).
    pub seasons_lost_1_plus: f64,
    /// NFL starters still starting the next season, by age, all positions
    /// (fix list 4.7; real nflverse about 0.70 at 22-27, 0.5-0.6 at 30).
    pub nfl_starter_kept_next_year_by_age: BTreeMap<u32, f64>,
}

fn reach(gaps: &[(f64, f64)]) -> Reach {
    let n = gaps.len();
    Reach {
        within2: r3(share(gaps.iter().filter(|x| x.0 <= 2.0).count(), n)),
        reached90pct_plus: r3(share(gaps.iter().filter(|x| x.0 > 2.0 && x.1 >= 0.9).count(), n)),
        below90pct: r3(share(gaps.iter().filter(|x| x.1 < 0.9).count(), n)),
    }
}

pub fn summarize(result: &CohortResult) -> Summary {
    let records = &result.records;
    let draft_slots = result.draft_slots;
    let n = records.len();
    let drafted: Vec<&CareerRecord> = records.iter().filter(|r| is_drafted(r)).collect();
    let pros: Vec<&CareerRecord> = records.iter().filter(|r| signed_pro(r)).collect();
    let by_stars = |s: u8| records.iter().filter(move |r| r.stars == s).collect::<Vec<_>>();
    let draft_rate = |rs: &[&CareerRecord]| {
        if rs.is_empty() { f64::NAN } else { share(rs.iter().filter(|r| is_drafted(r)).count(), rs.len()) }
    };
    let round_of = |r: &CareerRecord| draft_round(r.draft_pick.unwrap(), draft_slots);
    let on_roster = |s: &SeasonRecord| {
        s.level == Level::NFL && s.playing_time != PlayingTime::PracticeSquad && s.playing_time != PlayingTime::Unsigned
    };
    let nfl_seasons: Vec<&SeasonRecord> =
        records.iter().flat_map(|r| r.player.history.iter().filter(|s| on_roster(s))).collect();
    let nfl_overalls: Vec<f64> = nfl_seasons.iter().map(|s| s.overall).collect();
    let freshman: Vec<f64> = records.iter().map(|r| r.initial_overall).collect();
    let senior_overall: Vec<f64> = records.iter().map(|r| senior(r).overall).collect();
    let peak_ages_by = |pos: &[Position]| -> Vec<f64> {
        pros.iter()
            .filter(|r| pos.contains(&r.player.position) && r.nfl_seasons >= 4)
            .map(|r| nfl_peak_age(r).unwrap() as f64)
            .collect()
    };
    let first_round: Vec<&CareerRecord> = drafted.iter().copied().filter(|r| round_of(r) == 1).collect();
    let late_round: Vec<&CareerRecord> = drafted.iter().copied().filter(|r| round_of(r) == 7).collect();
    let mut young_seasons = 0usize;
    let mut regressions = 0usize;
    let mut adult_seasons = 0usize;
    let mut lost_1 = 0usize;
    let mut kept: BTreeMap<u32, (usize, usize)> = BTreeMap::new();
    for r in records {
        let h = &r.player.history;
        for i in 1..h.len() {
            if h[i].age >= 18 && h[i].age <= 23 {
                young_seasons += 1;
                if h[i].overall < h[i - 1].overall {
                    regressions += 1;
                }
            }
            if h[i].age >= 18 {
                adult_seasons += 1;
                if h[i].overall < h[i].overall_start - 1.0 {
                    lost_1 += 1;
                }
            }
            let prev = &h[i - 1];
            if prev.level == Level::NFL && prev.playing_time == PlayingTime::Starter && (22..=31).contains(&prev.age) {
                let e = kept.entry(prev.age).or_default();
                e.1 += 1;
                if h[i].level == Level::NFL && h[i].playing_time == PlayingTime::Starter {
                    e.0 += 1;
                }
            }
        }
    }
    // How close pros with 4+ seasons came to their potential: at their best
    // season against their potential that season, and against their potential
    // on draft day (ceilings keep rising in the pros as veterans learn).
    let long_pros: Vec<&CareerRecord> = pros.iter().copied().filter(|r| r.nfl_seasons >= 4).collect();
    let reach_then: Vec<(f64, f64)> = long_pros
        .iter()
        .map(|r| {
            let h = &r.player.history;
            let mut k = 0;
            for (i, s) in h.iter().enumerate() {
                if s.overall_start > h[k].overall_start {
                    k = i;
                }
            }
            let potential_then =
                if k > 0 { debug::season_potential(&h[k - 1]) } else { debug::original_potential(&r.player) };
            (potential_then - h[k].overall_start, h[k].overall_start / potential_then)
        })
        .collect();
    let reach_draft_day: Vec<(f64, f64)> = long_pros
        .iter()
        .map(|r| {
            let exit = debug::potential_at_exit(r);
            (exit - r.peak_overall, r.peak_overall / exit)
        })
        .collect();
    let top_seniors: Vec<&CareerRecord> = records.iter().filter(|r| senior_level(r) >= 85).collect();
    let exit_overall: Vec<f64> =
        records.iter().filter(|r| r.college.is_some()).map(|r| last_amateur(r).overall).collect();
    let draft_overall: Vec<f64> = drafted.iter().map(|r| last_amateur(r).overall).collect();
    // Consensus board: every prospect ranked by the grade teams saw.
    let mut graded: Vec<&CareerRecord> = records.iter().filter(|r| r.draft_grade.is_some()).collect();
    graded.sort_by(|a, b| b.draft_grade.unwrap().total_cmp(&a.draft_grade.unwrap()));
    let consensus_rank: std::collections::HashMap<u32, usize> =
        graded.iter().enumerate().map(|(i, r)| (r.player.id, i + 1)).collect();
    let veteran_gaps: Vec<f64> = records
        .iter()
        .flat_map(|r| r.player.history.iter().filter(|s| on_roster(s) && s.age >= 33 && s.age <= 35))
        .map(|s| debug::season_potential(s) - s.overall)
        .collect();
    let specialist_nfl_rows: Vec<&SeasonRecord> = records
        .iter()
        .filter(|r| is_specialist(r))
        .flat_map(|r| r.player.history.iter().filter(|s| s.level == Level::NFL))
        .collect();
    let college_rows = || records.iter().flat_map(|r| r.player.history.iter().filter(|s| s.level.is_college()));
    let count_where = |f: &dyn Fn(&CareerRecord) -> bool| records.iter().filter(|r| f(r)).count();
    let seasons_with = |e: Ev| records.iter().flat_map(|r| r.player.history.iter()).filter(|s| s.events.has(e)).count();
    let d2_drafted: Vec<&CareerRecord> =
        drafted.iter().copied().filter(|r| r.college == Some(CollegeLevel::SmallCollege)).collect();
    let five = by_stars(5);
    let mut transfer_routes = BTreeMap::new();
    for r in records {
        for t in &r.transfers {
            let route = format!("{}>{}", t.from.level().name(), t.to.level().name());
            *transfer_routes.entry(route).or_insert(0) += 1;
        }
    }
    let level90 = |r: &CareerRecord| senior_level(r) >= 90;
    // Recruiting looks at the position he played as a senior.
    let qbs_top = records.iter().filter(|r| level90(r) && senior(r).position == Position::QB).count();
    let all_top = records.iter().filter(|r| level90(r)).count();
    let qbs = records.iter().filter(|r| senior(r).position == Position::QB).count();
    let level_of = |s: &SeasonRecord| s.level_rating.unwrap_or(0);

    Summary {
        players: n,
        draft_slots,
        rates: Rates {
            college: round(share(count_where(&|r| r.college.is_some()), n), 4),
            signed_fbs: round(share(count_where(&|r| r.signed_with == Some(CollegeLevel::FBS)), n), 4),
            finished_at_fbs: round(share(count_where(&|r| r.college == Some(CollegeLevel::FBS)), n), 4),
            drafted: round(share(drafted.len(), n), 5),
            signed_pro: round(share(pros.len(), n), 5),
        },
        draft_rate_by_stars: StarRates {
            five: r3(draft_rate(&five)),
            four: r3(draft_rate(&by_stars(4))),
            three: r3(draft_rate(&by_stars(3))),
            two: r3(draft_rate(&by_stars(2))),
            unranked: round(draft_rate(&by_stars(0)), 5),
        },
        drafted_share_by_stars: DraftedShares {
            five: r3(share(drafted.iter().filter(|r| r.stars == 5).count(), drafted.len())),
            four: r3(share(drafted.iter().filter(|r| r.stars == 4).count(), drafted.len())),
            three: r3(share(drafted.iter().filter(|r| r.stars == 3).count(), drafted.len())),
            low_or_unrated: r3(share(drafted.iter().filter(|r| r.stars <= 2).count(), drafted.len())),
            from_small_schools: r3(share(
                drafted.iter().filter(|r| r.college != Some(CollegeLevel::FBS)).count(),
                drafted.len(),
            )),
            from_fcs: r3(share(drafted.iter().filter(|r| r.college == Some(CollegeLevel::FCS)).count(), drafted.len())),
            from_d2: r3(share(d2_drafted.len(), drafted.len())),
        },
        hs: Hs {
            freshman_overall: MeanP99 { mean: round(mean(&freshman), 1), p99: round(quantile(&freshman, 0.99), 1) },
            senior_overall: MeanP99 {
                mean: round(mean(&senior_overall), 1),
                p99: round(quantile(&senior_overall, 0.99), 1),
            },
            five_star_senior_overall: round(mean(&five.iter().map(|r| senior(r).overall).collect::<Vec<_>>()), 1),
        },
        draft: Draft {
            rookie_overall: Spread {
                p10: round(quantile(&draft_overall, 0.1), 1),
                median: round(median(&draft_overall), 1),
                p90: round(quantile(&draft_overall, 0.9), 1),
            },
            age: round(
                mean(
                    &drafted
                        .iter()
                        .map(|r| r.player.history.iter().find(|s| s.level == Level::NFL).unwrap().age as f64)
                        .collect::<Vec<_>>(),
                ),
                1,
            ),
            first_rounders_outside_top50: drafted
                .iter()
                .filter(|r| round_of(r) == 1 && consensus_rank[&r.player.id] > 50)
                .count(),
        },
        nfl: Nfl {
            season_overall: SeasonOverall {
                mean: round(mean(&nfl_overalls), 1),
                p10: round(quantile(&nfl_overalls, 0.1), 1),
                p90: round(quantile(&nfl_overalls, 0.9), 1),
                share90plus: r3(share(nfl_overalls.iter().filter(|&&o| o >= 90.0).count(), nfl_overalls.len())),
                share80plus: r3(share(nfl_overalls.iter().filter(|&&o| o >= 80.0).count(), nfl_overalls.len())),
            },
            career_length: CareerLength {
                drafted: round(mean(&drafted.iter().map(|r| r.nfl_seasons as f64).collect::<Vec<_>>()), 2),
                all_signed: round(mean(&pros.iter().map(|r| r.nfl_seasons as f64).collect::<Vec<_>>()), 2),
                first_round: round(mean(&first_round.iter().map(|r| r.nfl_seasons as f64).collect::<Vec<_>>()), 2),
                round7: round(mean(&late_round.iter().map(|r| r.nfl_seasons as f64).collect::<Vec<_>>()), 2),
                udfa: round(mean(&pros.iter().filter(|r| r.udfa).map(|r| r.nfl_seasons as f64).collect::<Vec<_>>()), 2),
            },
            median_peak_age: PeakAge {
                all_but_kp: median(&peak_ages_by(&[
                    Position::QB,
                    Position::RB,
                    Position::WR,
                    Position::TE,
                    Position::OT,
                    Position::IOL,
                    Position::EDGE,
                    Position::DT,
                    Position::LB,
                    Position::CB,
                    Position::S,
                ])),
                rb: median(&peak_ages_by(&[Position::RB])),
                qb: median(&peak_ages_by(&[Position::QB])),
                kp: median(&peak_ages_by(&[Position::K, Position::P])),
            },
            veteran_potential_gap: round(median(&veteran_gaps), 2),
            specialists: Specialists {
                signed: count_where(&|r| is_specialist(r) && r.nfl_seasons > 0),
                seasons_at_38_plus: r3(share(
                    specialist_nfl_rows.iter().filter(|s| s.age >= 38).count(),
                    specialist_nfl_rows.len(),
                )),
                oldest_season: specialist_nfl_rows.iter().map(|s| s.age).max().unwrap_or(0),
            },
        },
        small_schools: SmallSchools {
            d2_drafted: d2_drafted.len(),
            d2_earliest_round: d2_drafted.iter().map(|r| round_of(r)).min(),
            d2_udfa: count_where(&|r| r.udfa && r.college == Some(CollegeLevel::SmallCollege)),
            fcs_drafted: drafted.iter().filter(|r| r.college == Some(CollegeLevel::FCS)).count(),
            players_moved_up: count_where(&|r| r.transfers.iter().any(moved_up)),
            drafted_after_moving_up: drafted.iter().filter(|r| r.transfers.iter().any(moved_up)).count(),
            transfer_routes,
            drafted_signed_fcs: drafted.iter().filter(|r| r.signed_with == Some(CollegeLevel::FCS)).count(),
            drafted_signed_d2: drafted.iter().filter(|r| r.signed_with == Some(CollegeLevel::SmallCollege)).count(),
            drafted_after_moving_up_from_d2: drafted
                .iter()
                .filter(|r| r.signed_with == Some(CollegeLevel::SmallCollege) && r.transfers.iter().any(moved_up))
                .count(),
            drafted_after_moving_up_rounds1to3: drafted
                .iter()
                .filter(|r| r.transfers.iter().any(moved_up) && round_of(r) <= 3)
                .count(),
        },
        recruiting: Recruiting {
            five_star_senior_min: five.iter().map(|r| senior_level(r)).min().unwrap_or(0),
            top_seniors: top_seniors.len(),
            top_seniors_unranked: top_seniors.iter().filter(|r| r.stars == 0).count(),
            top_seniors_at_d2_or_none: top_seniors
                .iter()
                .filter(|r| r.signed_with.is_none_or(|c| c == CollegeLevel::SmallCollege))
                .count(),
            qb_top_senior_ratio: round(share(qbs_top, all_top) / share(qbs, n), 2),
            blue_chip_qbs: count_where(&|r| r.stars >= 4 && senior(r).position == Position::QB),
            five_star_positions: five.iter().map(|r| senior(r).position).collect::<HashSet<_>>().len(),
            top100_seniors_below_four_stars: count_where(&|r| {
                !is_specialist(r) && senior_level(r) >= 90 && r.stars < 4
            }),
            blue_chip_specialists: count_where(&|r| is_specialist(r) && r.stars >= 4),
            unranked_signed_fbs_or_fcs: count_where(&|r| {
                r.stars == 0 && matches!(r.signed_with, Some(CollegeLevel::FBS | CollegeLevel::FCS))
            }),
            level80_seniors_with_no_offer: count_where(&|r| senior_level(r) >= 80 && r.signed_with.is_none()),
        },
        trouble: TroubleStats {
            per_college_season: r3(share(
                records
                    .iter()
                    .flat_map(|r| r.player.history.iter())
                    .filter(|s| s.events.has(Ev::Suspended) || s.events.has(Ev::Dismissed))
                    .count(),
                college_rows().count(),
            )),
            suspensions: seasons_with(Ev::Suspended),
            dismissals: seasons_with(Ev::Dismissed),
            left_football: count_where(&|r| r.left_football),
            five_stars_in_trouble: five
                .iter()
                .filter(|r| r.player.has_had(Ev::Suspended) || r.player.has_had(Ev::Dismissed))
                .count(),
            suspended_twice_at_one_school: count_where(&repeat_suspensions_at_one_school),
        },
        level_ratings: LevelRatings {
            hs_seniors99: count_where(&|r| senior_level(r) >= 99),
            hs_seniors95plus: count_where(&|r| senior_level(r) >= 95),
            hs_seniors90plus: all_top,
            five_star_senior_median: median(&five.iter().map(|r| senior_level(r) as f64).collect::<Vec<_>>()),
            four_star_senior_median: median(&by_stars(4).iter().map(|r| senior_level(r) as f64).collect::<Vec<_>>()),
            college_seasons99: college_rows().filter(|s| level_of(s) >= 99).count(),
            hs_seasons99_any_age: records
                .iter()
                .flat_map(|r| r.player.history.iter())
                .filter(|s| s.level == Level::HS && level_of(s) >= 99)
                .count(),
            college_seasons95plus: college_rows().filter(|s| level_of(s) >= 95).count(),
            fbs_season_median: median(
                &records
                    .iter()
                    .flat_map(|r| r.player.history.iter())
                    .filter(|s| s.level == Level::FBS)
                    .map(|s| level_of(s) as f64)
                    .collect::<Vec<_>>(),
            ),
            best_d2_season: records
                .iter()
                .flat_map(|r| r.player.history.iter())
                .filter(|s| s.level == Level::SmallCollege)
                .map(level_of)
                .max()
                .unwrap_or(0),
            college_seasons90plus: college_rows().filter(|s| level_of(s) >= 90).count(),
            drafted_final_college_median: median(
                &drafted.iter().map(|r| level_of(last_amateur(r)) as f64).collect::<Vec<_>>(),
            ),
        },
        levels: Levels {
            college_exits_rated65_plus: exit_overall.iter().filter(|&&o| o >= 65.0).count(),
            college_exits_rated70_plus: exit_overall.iter().filter(|&&o| o >= 70.0).count(),
            drafted_below65: r3(share(draft_overall.iter().filter(|&&o| o < 65.0).count(), drafted.len())),
            peak_counts: PeakCounts {
                p97: count_where(&|r| r.peak_overall >= 97.0),
                p92: count_where(&|r| r.peak_overall >= 92.0),
                p85: count_where(&|r| r.peak_overall >= 85.0),
            },
        },
        reaching_potential: reach(&reach_then),
        reaching_draft_day_potential: reach(&reach_draft_day),
        potential: PotentialStats {
            initial_vs_exit_correlation_drafted: r3(correlation(
                &drafted.iter().map(|r| debug::initial_potential(r)).collect::<Vec<_>>(),
                &drafted.iter().map(|r| debug::potential_at_exit(r)).collect::<Vec<_>>(),
            )),
            initial_vs_exit_correlation_all: r3(correlation(
                &records.iter().map(debug::initial_potential).collect::<Vec<_>>(),
                &records.iter().map(debug::potential_at_exit).collect::<Vec<_>>(),
            )),
            drafted_rose_by8_plus_since14: r3(share(
                drafted.iter().filter(|r| debug::potential_at_exit(r) - debug::initial_potential(r) >= 8.0).count(),
                drafted.len(),
            )),
            collegians_fell_by8_plus_since14: r3(share(
                count_where(&|r| {
                    r.college.is_some() && debug::potential_at_exit(r) - debug::initial_potential(r) <= -8.0
                }),
                count_where(&|r| r.college.is_some()),
            )),
            young_regression_share: r3(share(regressions, young_seasons)),
            falling_breakouts: records
                .iter()
                .flat_map(|r| r.player.history.iter())
                .filter(|s| s.events.has(Ev::Breakout) && s.overall < s.overall_start)
                .count(),
            seasons_lost_1_plus: r3(share(lost_1, adult_seasons)),
            nfl_starter_kept_next_year_by_age: kept.iter().map(|(&a, &(k, n))| (a, r3(share(k, n)))).collect(),
        },
    }
}

/// Last age of a career that ended in retirement after 3+ seasons, median by position.
pub fn median_last_age(records: &[CareerRecord], pos: Position) -> f64 {
    median(
        &records
            .iter()
            .filter(|r| r.player.position == pos && r.pro_exit == Some(ProExit::Retired) && r.nfl_seasons >= 3)
            .map(|r| r.player.history.last().unwrap().age as f64)
            .collect::<Vec<_>>(),
    )
}

/// Real share of draft picks by position, 2000-2025 (nflverse, `data/real-world/draft_positions_2000_2025.csv`).
pub fn real_draft_shares() -> [f64; afm_engine::positions::POSITION_COUNT] {
    let csv = include_str!("../../../data/real-world/draft_positions_2000_2025.csv");
    let header: Vec<&str> = csv.lines().next().unwrap_or_default().split(',').collect();
    let row: Vec<&str> = csv.lines().find(|l| l.starts_with("all,")).unwrap_or_default().split(',').collect();
    let mut out = [0.0; afm_engine::positions::POSITION_COUNT];
    for pos in Position::ALL {
        let key = format!("pct_{}", pos.name());
        if let Some(k) = header.iter().position(|h| *h == key) {
            out[pos.index()] = row.get(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) / 100.0;
        }
    }
    out
}
