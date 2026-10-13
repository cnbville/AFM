//! Simulates a whole national high school class from freshman year to the end
//! of every pro career. Roster slots at each level are a fixed share of the
//! class, matching real participation: about 1.3% sign with FBS schools, 1% with
//! FCS, 4.5% with smaller colleges, and roughly 257 per 250,000 are drafted.
//! The pros are a full league of 32 teams (see `league`) whose depth charts
//! the class competes for.
//!
//! Speed: every player carries his own random stream, so the per-player work
//! (generation, four high school seasons, recruiting film, level ratings) runs
//! on all cores and still gives the same result for a seed on any machine.
//! Class-wide decisions (offers, the portal, the draft, the league) use the
//! class's own stream in a fixed order.

use std::collections::HashMap;

use rayon::prelude::*;

use crate::attributes::{Attr, AttributeGroup};
use crate::development::{SeasonContext, SeasonReport, Trouble, develop_season};
use crate::league::{DepthSlots, Member, Team, advance_league, depth_slots, make_league};
use crate::level_rating::{
    LevelGroup, LevelScale, Pool, PositionAligner, build_level_scale, level_group, level_rating, sets_level_scale,
};
use crate::player::{DevelopmentEvent as Ev, Level, Player, PlayingTime, ProspectOptions, generate_prospect};
use crate::positions::{Position, raw_weight};
use crate::rng::{Rng, Seed, clamp, round_half_up};
use crate::scouting::{ScoutOptions, scout_player};

/// The tuning knobs of the class pipeline.
pub struct Pipeline {
    /// Real US freshman class size the shares below are expressed against.
    pub reference_class_size: f64,
    /// Share of the class at 5, 4, 3 and 2 stars.
    pub star_shares: [(u8, f64); 4],
    pub college_shares: [(CollegeLevel, f64); 3],
    pub draft_picks: f64,
    /// UDFAs signed per drafted player.
    pub udfa_ratio: f64,
    /// How far above the FBS median a lower-level player must be to get an FBS offer in the portal (FCS, D2).
    pub transfer_margin: [f64; 2],
    /// Chance a player who earns a transfer up takes it. Most standouts stay put:
    /// about 85% of FCS all-conference players with eligibility left return.
    pub transfer_chance: f64,
    /// Scholarships open to lower-level transfers each year, by route, as a share
    /// of the class. Real portals move roughly 400 FCS and 150 D2 players up to
    /// FBS a year; D2 to FCS is a smaller, less tracked route.
    pub transfer_slots: [(CollegeLevel, CollegeLevel, f64); 3],
    /// Yearly chance a player enters the portal looking for playing time, by
    /// role. A coaching change at his school makes it more likely.
    pub portal_entry: PortalEntryRates,
    /// Yearly chance his college's head coach leaves or is fired.
    pub college_coaching_change: f64,
    /// Points scouts take off a grade for production against weaker opponents
    /// (FBS, FCS, D2, HS). Teams can't be sure a D2 star will hold up against pro athletes.
    pub competition_discount: [f64; 4],
    /// First round in which teams will spend a pick on a D2/D3/NAIA player (day 3 = round 4).
    pub small_college_earliest_round: u32,
    /// Off-field and academic trouble in college. A suspension or academic
    /// ineligibility costs a season of games; a dismissal ends his time at the
    /// school. The chance is a year's risk for average work ethic and
    /// coachability and runs about three times higher for the least disciplined.
    /// A serious cause (violence, a felony charge) drops him a level or ends his
    /// career; a lesser one (academics, a failed drug test, team rules) usually
    /// means a fresh start at a smaller FBS program if he is good enough.
    pub college_trouble: CollegeTrouble,
    /// Points every draft board takes off for each suspension and dismissal on a player's record.
    pub red_flag_discount: RedFlags,
    /// Stars follow what a senior is on film as well as what he might become.
    /// Nobody gets four or five stars without being among the best seniors in
    /// the country (top 100 for five stars, top 1,000 for four, 2,700 for three
    /// and 6,500 for two), and nobody in the top 150 goes without four stars,
    /// the top 800 without three, or the top 3,000 without two. Kickers and
    /// punters get at most three stars. (Film line option C.)
    pub star_film_rank: StarFilmRank,
    /// Worst senior film rank an FBS staff will sign below three stars.
    pub fbs_offer_max_film_rank: f64,
    /// Roster value a rookie contract buys, by round (UDFAs last): guaranteed
    /// money makes teams patient with early picks. It fades out over the deal.
    pub rookie_protection: [f64; 9],
    pub running_qb_retire_offset: f64,
}

pub struct PortalEntryRates {
    pub backup: f64,
    pub rotation: f64,
    pub coaching_change: f64,
}
pub struct CollegeTrouble {
    pub chance: f64,
    pub dismissal_share: f64,
    pub serious_share: f64,
    pub serious_quit_share: f64,
    pub lesser_quit_share: f64,
}
pub struct RedFlags {
    pub suspended: f64,
    pub dismissed: f64,
}
pub struct StarFilmRank {
    pub max_for5: f64,
    pub max_for4: f64,
    pub max_for3: f64,
    pub max_for2: f64,
    pub min_for4: f64,
    pub min_for3: f64,
    pub min_for2: f64,
}

pub const PIPELINE: Pipeline = Pipeline {
    reference_class_size: 250_000.0,
    star_shares: [(5, 0.00013), (4, 0.0014), (3, 0.008), (2, 0.016)],
    college_shares: [(CollegeLevel::FBS, 0.013), (CollegeLevel::FCS, 0.01), (CollegeLevel::SmallCollege, 0.045)],
    draft_picks: 257.0,
    udfa_ratio: 1.6,
    transfer_margin: [0.0, 2.0],
    transfer_chance: 0.15,
    transfer_slots: [
        (CollegeLevel::FCS, CollegeLevel::FBS, 0.0004),
        (CollegeLevel::SmallCollege, CollegeLevel::FBS, 0.00015),
        (CollegeLevel::SmallCollege, CollegeLevel::FCS, 0.0001),
    ],
    portal_entry: PortalEntryRates { backup: 0.18, rotation: 0.07, coaching_change: 1.8 },
    college_coaching_change: 0.12,
    competition_discount: [0.0, 1.0, 5.0, 0.0],
    small_college_earliest_round: 4,
    college_trouble: CollegeTrouble {
        chance: 0.03,
        dismissal_share: 0.3,
        serious_share: 0.35,
        serious_quit_share: 0.45,
        lesser_quit_share: 0.1,
    },
    red_flag_discount: RedFlags { suspended: 4.0, dismissed: 8.0 },
    star_film_rank: StarFilmRank {
        max_for5: 100.0,
        max_for4: 1000.0,
        max_for3: 2700.0,
        max_for2: 6500.0,
        min_for4: 150.0,
        min_for3: 800.0,
        min_for2: 3000.0,
    },
    fbs_offer_max_film_rank: 4500.0,
    rookie_protection: [0.0, 9.0, 6.0, 4.0, 2.5, 1.5, 1.0, 0.5, 0.0],
    running_qb_retire_offset: -4.0,
};

/// General draft value by position, in grade points: teams pay for
/// quarterbacks, pass rushers, tackles and corners, and wait on backs.
pub fn position_value(pos: Position) -> f64 {
    match pos {
        Position::QB => 3.5,
        Position::EDGE | Position::OT => 1.5,
        Position::CB => 1.0,
        Position::WR | Position::DT => 0.5,
        Position::IOL | Position::S => -0.8,
        Position::LB | Position::TE => -1.0,
        Position::RB => -3.5,
        Position::K | Position::P => 0.0,
    }
}

/// Typical age a player at each position walks away. Longevity, love of the
/// game, wear and still being good move it a lot. A quarterback who relies
/// on his legs leaves earlier than a pocket passer.
pub fn retire_age(pos: Position) -> f64 {
    match pos {
        Position::QB => 35.0,
        Position::RB => 28.0,
        Position::WR | Position::EDGE | Position::LB | Position::CB | Position::S => 30.0,
        Position::TE | Position::DT => 31.0,
        Position::OT | Position::IOL => 32.0,
        Position::K | Position::P => 37.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CollegeLevel {
    SmallCollege,
    FCS,
    FBS,
}

impl CollegeLevel {
    pub fn level(self) -> Level {
        match self {
            CollegeLevel::FBS => Level::FBS,
            CollegeLevel::FCS => Level::FCS,
            CollegeLevel::SmallCollege => Level::SmallCollege,
        }
    }
    /// How it is said in notes and on screens.
    pub fn name(self) -> &'static str {
        match self {
            CollegeLevel::FBS => "FBS",
            CollegeLevel::FCS => "FCS",
            CollegeLevel::SmallCollege => "D2",
        }
    }
    fn below(self) -> Option<CollegeLevel> {
        match self {
            CollegeLevel::FBS => Some(CollegeLevel::FCS),
            CollegeLevel::FCS => Some(CollegeLevel::SmallCollege),
            CollegeLevel::SmallCollege => None,
        }
    }
    fn index(self) -> usize {
        self as usize
    }
}

fn competition_discount(level: Option<CollegeLevel>) -> f64 {
    let d = PIPELINE.competition_discount;
    match level {
        Some(CollegeLevel::FBS) => d[0],
        Some(CollegeLevel::FCS) => d[1],
        Some(CollegeLevel::SmallCollege) => d[2],
        None => d[3],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransferReason {
    Standout,
    PlayingTime,
    CoachingChange,
    DismissedLesser,
    DismissedSerious,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortalOutcome {
    Up,
    Sideways,
    Down,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProExit {
    Retired,
    Unsigned,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transfer {
    pub age: u32,
    pub from: CollegeLevel,
    pub to: CollegeLevel,
    pub reason: TransferReason,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalEntry {
    pub age: u32,
    pub from: CollegeLevel,
    pub reason: TransferReason,
    pub outcome: PortalOutcome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CareerRecord {
    pub player: Player,
    pub initial_overall: f64,
    /// Hidden: see `debug`.
    pub(crate) initial_potential: f64,
    /// Recruiting stars at signing (0 = unranked).
    pub stars: u8,
    /// Where his senior film ranked nationally (1 = best).
    pub senior_film_rank: u32,
    /// College he finished at (after any transfers).
    pub college: Option<CollegeLevel>,
    /// College that first signed him out of high school.
    pub signed_with: Option<CollegeLevel>,
    /// Profile of his current FBS program, 0-1 (1 = a national power); None below FBS.
    pub prestige: Option<f64>,
    pub transfers: Vec<Transfer>,
    /// Every portal entry, including those that found no school.
    pub portal: Vec<PortalEntry>,
    /// Left football during college (a dismissal, or the portal found him no school).
    pub left_football: bool,
    /// True potential when leaving college (or high school). Hidden: see `debug`.
    pub(crate) potential_at_exit: f64,
    /// Consensus draft grade when he left college: what teams saw, after red flags and the competition discount.
    pub draft_grade: Option<f64>,
    pub draft_pick: Option<u32>,
    pub udfa: bool,
    /// Team that drafted or first signed him.
    pub team: Option<usize>,
    /// Every pro team he was with, in order.
    pub pro_teams: Vec<usize>,
    /// Seasons on an active roster.
    pub nfl_seasons: u32,
    pub practice_squad_seasons: u32,
    /// How his pro career ended: retired, or retired after going unsigned.
    pub pro_exit: Option<ProExit>,
    pub peak_overall: f64,
    pub peak_age: u32,
    /// A quarterback whose game leans on his legs.
    pub running_qb: bool,
    /// Plain-language reasons for the notable things that happened to him, for trial cards.
    pub notes: Vec<String>,
}

impl CareerRecord {
    fn new(player: Player) -> Self {
        let ovr = player.overall();
        CareerRecord {
            initial_overall: ovr,
            initial_potential: player.potential(),
            stars: 0,
            senior_film_rank: 0,
            college: None,
            signed_with: None,
            prestige: None,
            transfers: Vec::new(),
            portal: Vec::new(),
            left_football: false,
            potential_at_exit: 0.0,
            draft_grade: None,
            draft_pick: None,
            udfa: false,
            team: None,
            pro_teams: Vec::new(),
            nfl_seasons: 0,
            practice_squad_seasons: 0,
            pro_exit: None,
            peak_overall: ovr,
            peak_age: player.age,
            running_qb: false,
            notes: Vec::new(),
            player,
        }
    }

    fn note(&mut self, s: String) {
        self.notes.push(s);
    }
}

pub struct CohortResult {
    pub records: Vec<CareerRecord>,
    pub draft_slots: u32,
    pub teams: Vec<Team>,
}

#[derive(Clone, Debug)]
pub struct CohortOptions {
    pub size: usize,
    pub seed: Seed,
}

impl CohortOptions {
    pub fn new(size: usize, seed: impl Into<Seed>) -> Self {
        CohortOptions { size, seed: seed.into() }
    }
}

/// Per-player state carried through college.
struct College {
    coaching: Vec<f64>,
    /// First-year dip at his current school, cleared after that year.
    pending_dip: Vec<Option<f64>>,
}

pub fn simulate_cohort(opts: CohortOptions) -> CohortResult {
    let size = opts.size;
    let mut rng = Rng::new(opts.seed);
    let share = size as f64 / PIPELINE.reference_class_size;

    // Each player gets his own random stream, so per-player work can run in parallel.
    let seeds: Vec<u32> = (0..size).map(|_| rng.next_u32()).collect();
    let generated: Vec<(CareerRecord, Rng, f64)> = seeds
        .par_iter()
        .enumerate()
        .map(|(i, &seed)| {
            let mut prng = Rng::new(seed);
            let player = generate_prospect(&mut prng, ProspectOptions { id: Some(i as u32), ..Default::default() });
            let hs_coaching = clamp(prng.normal(8.0, 3.0), 1.0, 20.0);
            (CareerRecord::new(player), prng, hs_coaching)
        })
        .collect();
    let mut records = Vec::with_capacity(size);
    let mut prngs = Vec::with_capacity(size);
    let mut hs_coaching = Vec::with_capacity(size);
    for (r, p, c) in generated {
        records.push(r);
        prngs.push(p);
        hs_coaching.push(c);
    }
    // The pro league exists the whole time: it ages and changes staffs while the class grows up.
    let mut teams = make_league(&mut rng);

    // --- High school: four seasons, ages 14-17 -------------------------------
    // Varsity starting spots get easier to earn as a class gets older.
    let hs_starter_share = [0.12, 0.3, 0.5, 0.6];
    for share_this_year in hs_starter_share {
        let seen: Vec<(Position, f64)> = records.par_iter().map(|r| (r.player.position, r.player.overall())).collect();
        let roles = roles_by_rank(&seen, share_this_year, 0.2);
        records.par_iter_mut().zip(prngs.par_iter_mut()).zip(roles.par_iter()).zip(hs_coaching.par_iter()).for_each(
            |(((r, prng), &role), &coaching)| {
                develop_season(&mut r.player, &SeasonContext::new(Level::HS, coaching, role), prng);
                track_peak(r);
            },
        );
    }

    // --- Recruiting: rank the class on what scouts see -------------------------
    // Recruiting services rank on upside; college staffs offer on film, so what
    // a senior already is counts for more in who signs where. Film is his senior
    // season: the summer before college comes after signing day. Several
    // services and staffs grade the same film, so between them they read it
    // well. Upside is one shared guess: services follow each other's
    // projections, so pooling them doesn't sharpen it. A senior who missed much
    // of his last season to injury has less film, and what there is worries staffs.
    let film_and_projection: Vec<(f64, f64)> = records
        .par_iter_mut()
        .zip(prngs.par_iter_mut())
        .map(|(r, prng)| {
            let senior = &r.player.history[3];
            let senior_season = senior.overall_start;
            let games_missed = senior.games_missed;
            let major = senior.events.has(Ev::MajorInjury);
            let now = r.player.overall();
            let mut film = 0.0;
            let mut upside = 0.0;
            for k in 0..3 {
                let rep = scout_player(&r.player, ScoutOptions { accuracy: 12.0, level: Level::HS }, prng);
                film += (senior_season + rep.overall - now) / 3.0;
                if k == 0 {
                    upside = rep.potential - rep.overall;
                }
            }
            let missed = games_missed as f64 / 10.0;
            if missed > 0.3 {
                film -= 6.0 * missed;
                if major {
                    r.note(format!("A major injury cost him {games_missed} of 10 games as a senior, so recruiters had little senior film and ranked him lower."));
                }
            }
            (film, film + (now - senior_season) + upside)
        })
        .collect();
    // Dominance is judged against his own position: quarterbacks and linemen
    // mature later, so the best of them rate lower at 17 than the best backs.
    let align_film = PositionAligner::new(
        &records.iter().zip(&film_and_projection).map(|(r, fp)| (r.player.position, fp.0)).collect::<Vec<_>>(),
    );
    let mut film_score = vec![0.0; size];
    let mut star_score = vec![0.0; size];
    let mut recruit_score = vec![0.0; size];
    for (i, r) in records.iter().enumerate() {
        let film = align_film.align(r.player.position, film_and_projection[i].0);
        let projection = film_and_projection[i].1;
        film_score[i] = film;
        // Few programs spend a high ranking or an early scholarship on a kicker or punter.
        let specialist = if r.player.position.is_specialist() { 8.0 } else { 0.0 };
        star_score[i] = 0.25 * film + 0.75 * projection - specialist;
        recruit_score[i] = 0.4 * film + 0.6 * projection;
    }
    let mut film_rank = vec![0u32; size];
    let mut by_film: Vec<usize> = (0..size).collect();
    by_film.sort_by(|&a, &b| film_score[b].total_cmp(&film_score[a]));
    for (k, &i) in by_film.iter().enumerate() {
        film_rank[i] = k as u32 + 1;
        records[i].senior_film_rank = k as u32 + 1;
    }
    let mut by_star_score: Vec<usize> = (0..size).collect();
    by_star_score.sort_by(|&a, &b| star_score[b].total_cmp(&star_score[a]));
    let fr = &PIPELINE.star_film_rank;
    for (stars, star_share) in PIPELINE.star_shares {
        let (max_rank, must_rank) = match stars {
            5 => (fr.max_for5, 0.0),
            4 => (fr.max_for4, fr.min_for4),
            3 => (fr.max_for3, fr.min_for3),
            _ => (fr.max_for2, fr.min_for2),
        };
        let mut n = round_half_up(star_share * size as f64) as i64;
        let ceiling = (n as f64).max(round_half_up(max_rank * share));
        let floor = round_half_up(must_rank * share);
        // First the dominant seniors this tier must include, then the best of the rest on upside.
        for floor_pass in [true, false] {
            for &i in &by_star_score {
                if n <= 0 {
                    break;
                }
                let rank = film_rank[i] as f64;
                let r = &mut records[i];
                if r.stars != 0 || rank > ceiling || (floor_pass && rank > floor) {
                    continue;
                }
                if stars >= 4 && r.player.position.is_specialist() {
                    continue;
                }
                r.stars = stars;
                n -= 1;
            }
        }
    }
    // Offers follow the rankings: every three-star or better signs before anyone
    // below him. Below that, staffs trust their own film over a service's last
    // stars. FBS staffs pick two-stars and unranked seniors on film and upside;
    // FCS and then D2 staffs sign the best of what's left on film.
    let tier = |r: &CareerRecord| if r.stars >= 3 { r.stars } else { 0 };
    let mut by_recruit: Vec<usize> = (0..size).collect();
    by_recruit.sort_by(|&a, &b| {
        tier(&records[b]).cmp(&tier(&records[a])).then(recruit_score[b].total_cmp(&recruit_score[a]))
    });
    let mut cursor = 0usize;
    let mut col = College { coaching: vec![0.0; size], pending_dip: vec![None; size] };
    for (level, level_share) in PIPELINE.college_shares {
        let n = round_half_up(level_share * size as f64) as usize;
        if level == CollegeLevel::FBS {
            let limit = PIPELINE.fbs_offer_max_film_rank * share;
            let on_film = |i: usize| (tier(&records[i]) > 0 || film_rank[i] as f64 <= limit) as u8;
            by_recruit[cursor..].sort_by(|&a, &b| {
                on_film(b)
                    .cmp(&on_film(a))
                    .then(tier(&records[b]).cmp(&tier(&records[a])))
                    .then(recruit_score[b].total_cmp(&recruit_score[a]))
            });
        }
        if level == CollegeLevel::FCS {
            by_recruit[cursor..].sort_by(|&a, &b| film_score[b].total_cmp(&film_score[a]));
        }
        let mut k = 0;
        while k < n && cursor < size {
            let i = by_recruit[cursor];
            cursor += 1;
            let r = &mut records[i];
            r.college = Some(level);
            r.signed_with = Some(level);
            // Blue-chip recruits land at the programs with the best staffs.
            if level == CollegeLevel::FBS {
                r.prestige = Some(1.0 - k as f64 / n as f64);
            }
            col.coaching[i] = program_coaching(r, &mut rng);
            let step = match level {
                CollegeLevel::FBS => Step::HsToFbs,
                CollegeLevel::FCS => Step::HsToFcs,
                CollegeLevel::SmallCollege => Step::HsToSmallCollege,
            };
            let adj = first_year_adjustment(&r.player, step, &mut rng);
            col.pending_dip[i] = Some(adj.dip);
            note_dip(r, &adj, &format!("college ({})", level.name()));
            k += 1;
        }
    }
    for r in records.iter_mut() {
        if r.college.is_none() {
            r.potential_at_exit = r.player.potential();
        }
    }

    // --- College: four seasons of eligibility in five years --------------------
    let collegians: Vec<usize> = (0..size).filter(|&i| records[i].college.is_some()).collect();
    let mut active = vec![false; size];
    for &i in &collegians {
        active[i] = true;
    }
    let mut entrants: Vec<usize> = Vec::new();
    let mut seasons_played: Vec<Option<u32>> = vec![None; size];
    let mut redshirted = vec![false; size];
    let mut last_role: Vec<Option<PlayingTime>> = vec![None; size];
    let mut dismissed: Vec<(usize, bool, String)> = Vec::new();
    // Players already suspended at their current school: a second incident there ends it.
    let mut on_notice = vec![false; size];
    let college_starter_share = [0.08, 0.3, 0.5, 0.65, 0.7];
    let draft_slots = round_half_up(PIPELINE.draft_picks * share).max(1.0) as u32;
    for year in 0..5usize {
        let active_list: Vec<usize> = collegians.iter().copied().filter(|&i| active[i]).collect();
        if active_list.is_empty() {
            break;
        }
        advance_league(&mut teams, &mut rng);
        let mut coaching_changed = vec![false; size];
        for level in [CollegeLevel::FBS, CollegeLevel::FCS, CollegeLevel::SmallCollege] {
            let group: Vec<usize> =
                active_list.iter().copied().filter(|&i| records[i].college == Some(level)).collect();
            // Staffs pick starters on what they see in camp: a player still adjusting
            // looks worse than his rating. Young players also have to beat the
            // upperclassmen ahead of them, who are better at bigger programs.
            let blocked = match year {
                0 => 10.0,
                1 => 5.0,
                _ => 0.0,
            };
            let seen: Vec<(Position, f64)> = group
                .iter()
                .map(|&i| {
                    let r = &records[i];
                    let v = r.player.overall()
                        - 12.0 * col.pending_dip[i].unwrap_or(0.0)
                        - blocked * r.prestige.unwrap_or(0.3)
                        + rng.normal(0.0, 1.5);
                    (r.player.position, v)
                })
                .collect();
            let roles = roles_by_rank(&seen, college_starter_share[year], 0.25);
            for (g, &i) in group.iter().enumerate() {
                let mut role = roles[g];
                let ct = &PIPELINE.college_trouble;
                let mut trouble = if rng.chance(trouble_chance(&records[i].player)) {
                    Some(if rng.chance(ct.dismissal_share) { Trouble::Dismissed } else { Trouble::Suspended })
                } else {
                    None
                };
                let mut cause = String::new();
                if trouble == Some(Trouble::Suspended) && on_notice[i] {
                    trouble = Some(Trouble::Dismissed);
                    cause = "a second suspension at the same school".to_string();
                }
                if trouble == Some(Trouble::Suspended) {
                    on_notice[i] = true;
                }
                if trouble.is_some() {
                    role = PlayingTime::Backup;
                } else if role == PlayingTime::Backup && !redshirted[i] && year <= 1 {
                    role = PlayingTime::Redshirt;
                    redshirted[i] = true;
                }
                let dip = col.pending_dip[i].take().unwrap_or(0.0);
                let r = &mut records[i];
                let ctx = SeasonContext {
                    trouble,
                    adjustment: dip,
                    ..SeasonContext::new(level.level(), col.coaching[i], role)
                };
                develop_season(&mut r.player, &ctx, &mut prngs[i]);
                add_wear(r, role, 0.5);
                track_peak(r);
                // A year without games doesn't use one of his four seasons.
                if role != PlayingTime::Redshirt && trouble.is_none() {
                    seasons_played[i] = Some(seasons_played[i].unwrap_or(0) + 1);
                }
                if trouble == Some(Trouble::Dismissed) {
                    let serious = cause.is_empty() && rng.chance(ct.serious_share);
                    if cause.is_empty() {
                        cause = if serious { rng.pick(&SERIOUS_CAUSES) } else { rng.pick(&LESSER_CAUSES) }.to_string();
                    }
                    dismissed.push((i, serious, cause));
                }
                if trouble == Some(Trouble::Suspended) {
                    let why = rng.pick(&SUSPENSION_CAUSES);
                    r.note(format!("Suspended for the season at {} ({why}).", r.player.age - 1));
                }
                last_role[i] = Some(role);
                if rng.chance(PIPELINE.college_coaching_change) {
                    coaching_changed[i] = true;
                    col.coaching[i] = program_coaching(r, &mut rng);
                }
            }
        }

        // --- Declaring for the draft: projection plus urgency ---------------------
        let eligible: Vec<usize> =
            active_list.iter().copied().filter(|&i| year >= 2 || seasons_played[i].unwrap_or(0) >= 4).collect();
        let grades: Vec<DraftScore> = eligible.iter().map(|&i| draft_score(&records[i], &mut rng)).collect();
        let mut board: Vec<f64> = grades
            .iter()
            .map(|g| g.score)
            .chain(entrants.iter().map(|&i| records[i].draft_grade.unwrap_or(0.0)))
            .collect();
        board.sort_by(|a, b| b.total_cmp(a));
        let order = draft_order(&teams, &mut rng);
        for (e, &i) in eligible.iter().enumerate() {
            let g = grades[e];
            let exhausted = seasons_played[i].unwrap_or(0) >= 4 || year == 4;
            let projected_pick = board.iter().position(|&s| s <= g.score).map_or(0, |p| p + 1) as u32;
            let declare = exhausted
                || decide_to_declare(
                    &mut records[i],
                    &g,
                    projected_pick,
                    draft_slots,
                    &order,
                    &teams,
                    coaching_changed[i],
                    last_role[i].unwrap_or(PlayingTime::Backup),
                    &mut rng,
                );
            if declare {
                active[i] = false;
                let r = &mut records[i];
                r.potential_at_exit = r.player.potential();
                r.draft_grade = Some(g.score);
                entrants.push(i);
            }
        }

        // A dismissed player who isn't ready to declare starts over, a level down or at a smaller program, or leaves the game.
        let mut starting_over = vec![false; size];
        let sorted = level_ratings(&records, &collegians, &active);
        for (i, serious, cause) in std::mem::take(&mut dismissed) {
            if !active[i] {
                continue;
            }
            let from = records[i].college.unwrap();
            let ovr = records[i].player.overall();
            let ct = &PIPELINE.college_trouble;
            let to = if serious {
                if rng.chance(ct.serious_quit_share) { None } else { from.below() }
            } else if rng.chance(ct.lesser_quit_share) {
                None
            } else {
                // Better players get more offers: a good enough one can stay at his level at a smaller program.
                let stay_chance = clamp(0.5 + (ovr - percentile(&sorted[from.index()], 0.5)) / 8.0, 0.05, 0.95);
                if rng.chance(stay_chance) { Some(from) } else { from.below() }
            };
            let age = records[i].player.age - 1;
            let kind = if serious { ", a serious cause" } else { ", a lesser cause" };
            if let Some(to) = to {
                let reason = if serious { TransferReason::DismissedSerious } else { TransferReason::DismissedLesser };
                move_school(&mut records[i], i, to, reason, age, &mut rng, &mut col);
                on_notice[i] = false;
                starting_over[i] = true;
                let (good, where_) = if to == from {
                    ("Good enough to keep offers at his level, he", format!("a lower-profile {} program", to.name()))
                } else {
                    ("He", format!("{}, a level down", to.name()))
                };
                records[i].note(format!("Dismissed at {age} for {cause}{kind}. {good} moved to {where_}."));
            } else {
                active[i] = false;
                let r = &mut records[i];
                r.left_football = true;
                r.potential_at_exit = r.player.potential();
                let kind = if serious { ", a serious cause," } else { "" };
                r.note(format!("Dismissed at {age} for {cause}{kind} and never played again."));
            }
        }

        // --- Transfer portal ----------------------------------------------------
        if year <= 3 {
            let levels = level_ratings(&records, &collegians, &active);
            // A player who turns down a move up has decided to stay this year, and
            // one just dismissed has to prove himself where he landed first.
            let mut stayed = starting_over;
            // Standouts at lower levels move up for the exposure, coaching and
            // competition. Each level only has so many open scholarships for transfers.
            for (from, to, slots) in PIPELINE.transfer_slots {
                let margin = PIPELINE.transfer_margin[if from == CollegeLevel::FCS { 0 } else { 1 }];
                let bar = percentile(&levels[to.index()], 0.5) + margin;
                let mut candidates: Vec<(usize, f64)> = collegians
                    .iter()
                    .copied()
                    .filter(|&i| {
                        active[i]
                            && records[i].college == Some(from)
                            && seasons_played[i].unwrap_or(0) < 4
                            && !stayed[i]
                    })
                    .map(|i| (i, records[i].player.overall()))
                    .filter(|&(_, ovr)| ovr >= bar)
                    // Coaches recruit the portal on film, so their order is only roughly by ability.
                    .map(|(i, ovr)| (i, ovr + rng.normal(0.0, 4.0)))
                    .collect();
                candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
                let mut open = round_half_up(slots * size as f64) as i64;
                for (i, _) in candidates {
                    if open <= 0 {
                        break;
                    }
                    if !rng.chance(PIPELINE.transfer_chance) {
                        stayed[i] = true;
                        continue;
                    }
                    open -= 1;
                    let age = records[i].player.age - 1;
                    records[i].portal.push(PortalEntry {
                        age,
                        from,
                        reason: TransferReason::Standout,
                        outcome: PortalOutcome::Up,
                    });
                    move_school(&mut records[i], i, to, TransferReason::Standout, age, &mut rng, &mut col);
                    stayed[i] = true;
                    let to_name = to.name();
                    records[i].note(format!("At {age} he was rated above the median {to_name} player, entered the portal and moved up to {to_name}."));
                }
            }
            // Players short of snaps look for a new school, more so after a coaching
            // change. Where they land depends on how the market rates them: across,
            // a level down, or nowhere.
            let snapshot: Vec<usize> = collegians.iter().copied().filter(|&i| active[i]).collect();
            for i in snapshot {
                if stayed[i] || seasons_played[i].is_none_or(|s| s >= 4) {
                    continue;
                }
                let role = last_role[i];
                let base = match role {
                    Some(PlayingTime::Backup) => PIPELINE.portal_entry.backup,
                    Some(PlayingTime::Rotation) => PIPELINE.portal_entry.rotation,
                    _ => 0.0,
                };
                let changed = coaching_changed[i];
                let p = base
                    * if changed { PIPELINE.portal_entry.coaching_change } else { 1.0 }
                    * (1.0 + (records[i].player.personality.competitiveness - 10.0) * 0.04);
                if !rng.chance(p) {
                    continue;
                }
                let from = records[i].college.unwrap();
                let reason = if changed { TransferReason::CoachingChange } else { TransferReason::PlayingTime };
                let seen = records[i].player.overall() + rng.normal(0.0, 3.0);
                let age = records[i].player.age - 1;
                let why = if changed {
                    "after a coaching change".to_string()
                } else {
                    format!("stuck as a {}", role.map_or("backup", |r| r.name()))
                };
                let mut to: Option<CollegeLevel> = None;
                if seen >= percentile(&levels[from.index()], 0.4) {
                    to = Some(from);
                } else if let Some(below) = from.below().filter(|b| seen >= percentile(&levels[b.index()], 0.45)) {
                    to = Some(below);
                } else if from == CollegeLevel::FBS
                    && seen >= percentile(&levels[CollegeLevel::SmallCollege.index()], 0.5)
                    && rng.chance(0.4)
                {
                    to = Some(CollegeLevel::SmallCollege);
                }
                let r = &mut records[i];
                let Some(to) = to else {
                    r.portal.push(PortalEntry { age, from, reason, outcome: PortalOutcome::None });
                    active[i] = false;
                    r.left_football = true;
                    r.potential_at_exit = r.player.potential();
                    r.note(format!("Entered the portal at {age} {why}, found no school and left football."));
                    continue;
                };
                let outcome = if to == from { PortalOutcome::Sideways } else { PortalOutcome::Down };
                r.portal.push(PortalEntry { age, from, reason, outcome });
                move_school(r, i, to, reason, age, &mut rng, &mut col);
                let where_ = if to == from {
                    format!("across to another {} school", to.name())
                } else {
                    format!("down to {}", to.name())
                };
                r.note(format!("Entered the portal at {age} {why} and moved {where_} for playing time."));
            }
        }
    }

    // --- Draft and undrafted free agency ---------------------------------------
    let pros = run_draft(&mut records, &entrants, &mut teams, draft_slots, &mut rng);

    // --- Pro careers ------------------------------------------------------------
    simulate_league_years(&mut records, &mut prngs, pros, &mut teams, &mut rng);

    attach_level_ratings(&mut records, share);
    CohortResult { records, draft_slots, teams }
}

const SERIOUS_CAUSES: [&str; 3] = ["an arrest for assault", "a felony charge", "a violent incident off the field"];
const LESSER_CAUSES: [&str; 3] = ["academic ineligibility", "a failed drug test", "repeated violations of team rules"];
const SUSPENSION_CAUSES: [&str; 3] = ["academics", "a failed drug test", "a violation of team rules"];

/// Coaching at his current program: FBS powers have the best staffs.
fn program_coaching(r: &CareerRecord, rng: &mut Rng) -> f64 {
    let base = match r.college {
        Some(CollegeLevel::FBS) => 11.0 + 4.0 * r.prestige.unwrap_or(0.5),
        Some(CollegeLevel::FCS) => 11.0,
        _ => 8.5,
    };
    clamp(rng.normal(base, 2.0), 1.0, 20.0)
}

fn move_school(
    r: &mut CareerRecord,
    i: usize,
    to: CollegeLevel,
    reason: TransferReason,
    age: u32,
    rng: &mut Rng,
    col: &mut College,
) {
    let from = r.college.unwrap();
    r.transfers.push(Transfer { age, from, to, reason });
    // A move across goes to a smaller program; a move up lands about mid-pack.
    r.prestige = if to != CollegeLevel::FBS {
        None
    } else if from == CollegeLevel::FBS {
        Some(clamp(r.prestige.unwrap_or(0.5) - rng.range(0.15, 0.45), 0.0, 0.6))
    } else {
        Some(rng.range(0.2, 0.7))
    };
    r.college = Some(to);
    col.coaching[i] = program_coaching(r, rng);
    let step = if to > from {
        Step::Up
    } else if to == from {
        Step::Sideways
    } else {
        Step::Down
    };
    let adj = first_year_adjustment(&r.player, step, rng);
    col.pending_dip[i] = Some(adj.dip);
}

/// Sorted ratings of the active players at each level (indexed by `CollegeLevel`).
fn level_ratings(records: &[CareerRecord], collegians: &[usize], active: &[bool]) -> [Vec<f64>; 3] {
    let mut out: [Vec<f64>; 3] = Default::default();
    for &i in collegians {
        if active[i] {
            out[records[i].college.unwrap().index()].push(records[i].player.overall());
        }
    }
    for v in out.iter_mut() {
        v.sort_by(f64::total_cmp);
    }
    out
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::INFINITY;
    }
    sorted[(sorted.len() - 1).min((q * sorted.len() as f64).floor() as usize)]
}

// --- First-year adjustment ---------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    HsToFbs,
    HsToFcs,
    HsToSmallCollege,
    Up,
    Sideways,
    Down,
    FbsToNfl,
    FcsToNfl,
    SmallCollegeToNfl,
    NewTeam,
}

impl Step {
    fn size(self) -> f64 {
        match self {
            Step::HsToFbs => 1.0,
            Step::HsToFcs => 0.8,
            Step::HsToSmallCollege => 0.55,
            Step::Up => 0.55,
            Step::Sideways => 0.25,
            Step::Down => 0.1,
            Step::FbsToNfl => 1.0,
            Step::FcsToNfl => 1.2,
            Step::SmallCollegeToNfl => 1.45,
            Step::NewTeam => 0.2,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Step::HsToFbs => "the jump from high school to FBS",
            Step::HsToFcs => "the jump from high school to FCS",
            Step::HsToSmallCollege => "the step from high school to D2",
            Step::Up => "a move up a level",
            Step::Sideways => "a new school",
            Step::Down => "a move down a level",
            Step::FbsToNfl => "the jump to the NFL",
            Step::FcsToNfl => "the big jump from FCS to the NFL",
            Step::SmallCollegeToNfl => "the huge jump from D2 to the NFL",
            Step::NewTeam => "a new team and scheme",
        }
    }
}

/// How long a position takes to learn at a new level.
fn position_difficulty(pos: Position) -> f64 {
    match pos {
        Position::QB => 1.3,
        Position::OT => 1.2,
        Position::IOL | Position::CB => 1.1,
        Position::S | Position::LB | Position::DT | Position::TE => 1.0,
        Position::EDGE | Position::WR => 0.9,
        Position::RB => 0.65,
        Position::K | Position::P => 0.5,
    }
}

/// Share of his ceiling a player has reached in one attribute group, weighted by what his position needs.
fn readiness(p: &Player, group: AttributeGroup) -> f64 {
    let mut sum = 0.0;
    let mut w = 0.0;
    for key in Attr::ALL {
        if key.profile().group != group {
            continue;
        }
        let weight = raw_weight(p.position, key) + 0.2;
        sum += weight * (p.attributes[key] / p.ceilings[key].max(1.0)).min(1.0);
        w += weight;
    }
    if w > 0.0 { sum / w } else { 0.8 }
}

#[derive(Clone, Debug)]
pub struct Adjustment {
    pub dip: f64,
    pub step: Step,
    /// The biggest reasons, in plain words.
    pub reasons: Vec<String>,
}

/// How hard a player's first year at a new level is, 0-1. It depends on the
/// size of the step, the position, how polished his technique is, his
/// football IQ and make-up, whether his body is ready, and his fit with the
/// new scheme plus luck. Some arrive ready at once; others need a year.
pub fn first_year_adjustment(p: &Player, step: Step, rng: &mut Rng) -> Adjustment {
    let polish = readiness(p, AttributeGroup::Technical);
    let iq = readiness(p, AttributeGroup::Mental);
    let body = readiness(p, AttributeGroup::Physical);
    let pers = &p.personality;
    let makeup = (pers.work_ethic + pers.competitiveness + pers.coachability) / 3.0;
    let fit = rng.normal(0.0, 0.12);
    let luck = rng.normal(0.0, 0.08);
    let polish_pct = round_half_up(polish * 100.0);
    let mut terms: Vec<(String, f64)> = vec![
        (step.name().to_string(), 0.42 * step.size() * position_difficulty(p.position)),
        (
            if polish < 0.8 {
                format!("raw technique ({polish_pct}% of his ceiling)")
            } else {
                format!("polished technique ({polish_pct}% of his ceiling)")
            },
            0.9 * (0.8 - polish),
        ),
        (
            (if iq < 0.75 { "still learning to read the game" } else { "a quick football mind" }).to_string(),
            0.6 * (0.75 - iq),
        ),
        (
            (if body < 0.85 { "a body not yet ready" } else { "a body ready for the level" }).to_string(),
            0.6 * (0.85 - body),
        ),
        ((if makeup >= 10.0 { "strong make-up" } else { "weak make-up" }).to_string(), -0.025 * (makeup - 10.0)),
        ((if fit < 0.0 { "a good scheme fit" } else { "a poor scheme fit" }).to_string(), fit),
    ];
    let dip = clamp(terms.iter().map(|t| t.1).sum::<f64>() + luck, 0.0, 1.0);
    if position_difficulty(p.position) >= 1.2 && step.size() >= 0.8 {
        terms[0].0 += &format!(" at a position that takes time ({})", p.position);
    }
    terms.retain(|t| t.1.abs() >= 0.04);
    terms.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    let reasons = terms
        .into_iter()
        .take(3)
        .map(|(t, v)| format!("{t} ({})", if v > 0.0 { "harder" } else { "easier" }))
        .collect();
    Adjustment { dip, step, reasons }
}

fn note_dip(r: &mut CareerRecord, adj: &Adjustment, where_: &str) {
    if adj.dip >= 0.55 {
        r.note(format!("A hard first year in {where_} (dip {:.2}): {}.", adj.dip, adj.reasons.join(", ")));
    } else if adj.dip <= 0.12 {
        r.note(format!("Ready at once in {where_} (dip {:.2}): {}.", adj.dip, adj.reasons.join(", ")));
    }
}

// --- Declaring early -----------------------------------------------------------

/// Whether an underclassman leaves for the draft. He weighs where he projects
/// against how urgently he needs to go (little NIL money at a small school, a
/// running back's short shelf life, his age, injuries, a coaching change, a
/// lost starting job, fear of his stock falling) and what keeps him in school
/// (big NIL money, room to raise his stock, a bad year to make up for, a bad
/// franchise likely to pick him).
#[allow(clippy::too_many_arguments)]
fn decide_to_declare(
    r: &mut CareerRecord,
    g: &DraftScore,
    projected_pick: u32,
    draft_slots: u32,
    order: &[usize],
    teams: &[Team],
    coaching_changed: bool,
    last_role: PlayingTime,
    rng: &mut Rng,
) -> bool {
    if r.player.position.is_specialist() {
        return false;
    }
    let p = &r.player;
    let round = if projected_pick > draft_slots { 8 } else { draft_round(projected_pick, draft_slots) };
    let base = [0.0, 3.0, 1.6, 0.6, -0.2, -0.7, -1.2, -1.6, -3.0][round as usize];
    let mut go: Vec<(String, f64)> = Vec::new();
    let mut stay: Vec<(String, f64)> = Vec::new();
    match r.college {
        Some(CollegeLevel::FCS) => go.push(("little NIL money at FCS".into(), 0.5)),
        Some(CollegeLevel::SmallCollege) => go.push(("almost no NIL money at D2".into(), 0.9)),
        _ => {}
    }
    if p.position == Position::RB {
        go.push(("a running back's short shelf life".into(), 0.7));
    }
    if p.age >= 23 {
        go.push((format!("his age ({})", p.age), 0.7));
    }
    let college: Vec<_> = p.history.iter().filter(|s| s.level.is_college()).collect();
    if college.iter().any(|s| s.events.has(Ev::MajorInjury)) {
        go.push(("a major injury in college: get paid before the next one".into(), 0.6));
    }
    if coaching_changed {
        go.push(("a coaching change at his school".into(), 0.5));
    }
    let before = if college.len() >= 2 { Some(college[college.len() - 2]) } else { None };
    if before.is_some_and(|b| b.playing_time == PlayingTime::Starter) && last_role != PlayingTime::Starter {
        go.push(("losing his starting job".into(), 0.6));
    }
    let headroom = g.potential_seen - g.overall_seen;
    if (2..=5).contains(&round) && headroom < 3.0 {
        go.push(("fear his stock had peaked".into(), 0.5));
    }
    if r.college == Some(CollegeLevel::FBS) && last_role == PlayingTime::Starter {
        let prestige = r.prestige.unwrap_or(0.0);
        let at = if prestige >= 0.66 { "a big program" } else { "his school" };
        stay.push((format!("NIL money at {at}"), -1.1 * prestige * if round == 1 { 0.3 } else { 1.0 }));
    }
    if headroom > 3.0 {
        stay.push(("room to raise his stock".into(), -(0.12 * (headroom - 3.0)).min(1.2)));
    }
    let last = college[college.len() - 1];
    if last.performance.is_some_and(|perf| perf < last.overall_start - 3.0) {
        stay.push(("a bad final year to make up for".into(), -0.5));
    }
    let likely_team = if projected_pick >= 1 && projected_pick <= draft_slots {
        Some(&teams[order[(projected_pick as usize - 1) % order.len()]])
    } else {
        None
    };
    if let Some(t) = likely_team.filter(|t| round <= 2 && t.organisation() <= 8.0) {
        stay.push((format!("the badly run {} likely to pick him", t.name), -0.6));
    }
    let total = base + go.iter().chain(stay.iter()).map(|t| t.1).sum::<f64>() + rng.normal(0.0, 0.5);
    let declare = total > 0.0;
    let projected = if round == 8 { "to go undrafted".to_string() } else { format!("round {round}") };
    let reasons =
        |xs: &[(String, f64)]| xs.iter().filter(|t| t.1.abs() >= 0.2).map(|t| t.0.clone()).collect::<Vec<_>>();
    let pushes = reasons(&go);
    let pulls = reasons(&stay);
    let age = p.age - 1;
    if declare {
        let mut s = format!("Declared early at {age}, projected {projected}");
        if !pushes.is_empty() {
            s += &format!(", pushed by {}", pushes.join(", "));
        }
        if !pulls.is_empty() {
            s += &format!(", despite {}", pulls.join(", "));
        }
        r.note(s + ".");
    } else if round <= 3 {
        let mut s = format!("Stayed in school at {age} though projected {projected}");
        if !pulls.is_empty() {
            s += &format!(", for {}", pulls.join(", "));
        }
        r.note(s + ".");
    }
    declare
}

// --- The draft -----------------------------------------------------------------

/// Draft order: the weakest teams pick first. Returns team indices.
pub(crate) fn draft_order(teams: &[Team], rng: &mut Rng) -> Vec<usize> {
    let mut v: Vec<(usize, f64)> = teams.iter().map(|t| (t.id, t.strength() + rng.normal(0.0, 0.6))).collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1));
    v.into_iter().map(|(t, _)| t).collect()
}

/// How much a team needs a position, in grade points.
pub(crate) fn need(team: &Team, pos: Position) -> f64 {
    let Some((rating, age)) = team.weakest_starter(pos) else {
        return 0.0;
    };
    if pos == Position::QB {
        // A team with a young franchise quarterback doesn't spend a high pick on another.
        if let Some(best) = team.depth[Position::QB.index()].iter().max_by(|a, b| a.rating.total_cmp(&b.rating))
            && best.rating >= 82.0
            && best.age <= 30
        {
            return -8.0;
        }
    }
    clamp((78.0 - rating) / 2.5, -1.5, 3.0) + if age >= 31 { 1.0 } else { 0.0 }
}

/// A class player who signed with a pro team.
struct Signing {
    rec: usize,
    team: usize,
    round: u32,
    contract_years: i32,
}

fn join(
    records: &mut [CareerRecord],
    teams: &mut [Team],
    signed: &mut Vec<Signing>,
    i: usize,
    team: usize,
    round: u32,
    years: i32,
) {
    let r = &mut records[i];
    let member = Member::class_player(r.player.overall(), r.player.age, i);
    teams[team].depth[r.player.position.index()].push(member);
    r.team = Some(team);
    r.pro_teams.push(team);
    signed.push(Signing { rec: i, team, round, contract_years: years });
}

fn run_draft(
    records: &mut [CareerRecord],
    entrants: &[usize],
    teams: &mut [Team],
    draft_slots: u32,
    rng: &mut Rng,
) -> Vec<Signing> {
    // Each team picks from its own board. A team's board is the consensus grade
    // seen through its front office (good GMs see clearly, bad ones reach),
    // plus what the position is worth and what the team needs.
    let order = draft_order(teams, rng);
    let mut by_strength: Vec<usize> = (0..teams.len()).collect();
    by_strength.sort_by(|&a, &b| teams[b].strength().total_cmp(&teams[a].strength()));
    let mut strength_rank = vec![0usize; teams.len()];
    for (k, &t) in by_strength.iter().enumerate() {
        strength_rank[t] = k;
    }
    // Boards rank each prospect against others at his position first, since
    // positions mature at different ages (a 21-year-old quarterback rates lower
    // than a back of the same promise); position value and need come on top.
    let grade_of = |records: &[CareerRecord], i: usize| records[i].draft_grade.unwrap_or(0.0);
    let mut pool: Vec<usize> =
        entrants.iter().copied().filter(|&i| !records[i].player.position.is_specialist()).collect();
    let align = PositionAligner::new(
        &pool.iter().map(|&i| (records[i].player.position, grade_of(records, i))).collect::<Vec<_>>(),
    );
    for &i in &pool {
        let r = &mut records[i];
        r.draft_grade = Some(align.align(r.player.position, r.draft_grade.unwrap_or(0.0)));
    }
    pool.sort_by(|&a, &b| grade_of(records, b).total_cmp(&grade_of(records, a)));
    let mut kickers: Vec<usize> =
        entrants.iter().copied().filter(|&i| records[i].player.position.is_specialist()).collect();
    kickers.sort_by(|&a, &b| grade_of(records, b).total_cmp(&grade_of(records, a)));
    let mut signed: Vec<Signing> = Vec::new();
    let mut boards: Vec<HashMap<usize, f64>> = vec![HashMap::new(); teams.len()];
    let mut board_offset = |team: &Team, i: usize, rng: &mut Rng| -> f64 {
        *boards[team.id].entry(i).or_insert_with(|| rng.normal(0.0, draft_board_noise(team.draft_skill)))
    };
    let mut pick = 1u32;
    while pick <= draft_slots && (!pool.is_empty() || !kickers.is_empty()) {
        let t = order[(pick as usize - 1) % order.len()];
        let round = draft_round(pick, draft_slots);
        // Kickers and punters have their own board: a team with a weak or ageing
        // one takes the best specialist left on day 3, now and then.
        if (round >= 5 || (round == 4 && rng.chance(0.3))) && !kickers.is_empty() {
            let k = kickers[0];
            let kpos = records[k].player.position;
            let (inc_rating, inc_age) = teams[t].weakest_starter(kpos).unwrap_or((0.0, 0));
            let wants = inc_rating < 78.0 || inc_age >= 35;
            if wants && grade_of(records, k) + board_offset(&teams[t], k, rng) >= inc_rating + 1.0 && rng.chance(0.15) {
                kickers.remove(0);
                records[k].draft_pick = Some(pick);
                let what = if kpos == Position::K { "kicker" } else { "punter" };
                let old = if inc_age >= 35 { format!(" and {inc_age}") } else { String::new() };
                let name = teams[t].name;
                records[k].note(format!(
                    "The {name} needed a {what} (theirs was rated {}{old}) and took him with pick {pick} off the specialist board.",
                    round_half_up(inc_rating)
                ));
                join(records, teams, &mut signed, k, t, round, 4);
                pick += 1;
                continue;
            }
        }
        if pool.is_empty() {
            pick += 1;
            continue;
        }
        let mut best_index: Option<usize> = None;
        let mut best_value = f64::NEG_INFINITY;
        let mut best_parts = (0.0, 0.0, 0.0);
        let rf = if round <= 2 {
            1.0
        } else if round <= 4 {
            0.6
        } else {
            0.3
        };
        for (idx, &i) in pool.iter().enumerate().take(60) {
            let r = &records[i];
            if r.college == Some(CollegeLevel::SmallCollege) && round < PIPELINE.small_college_earliest_round {
                continue;
            }
            let pos = r.player.position;
            let n = need(&teams[t], pos) * if round <= 2 { 0.5 } else { 1.0 };
            let pv = position_value(pos) * rf;
            let grade = grade_of(records, i);
            // A strong team with one hole reaches for a rare talent who fills it.
            let reach = if strength_rank[t] < 10 && grade >= 82.0 && n >= 1.0 { 1.5 } else { 0.0 };
            let value = grade
                + board_offset(&teams[t], i, rng)
                + pv
                + if pos == Position::QB && round > 3 { n.max(-1.0) } else { n }
                + reach;
            if value > best_value {
                best_value = value;
                best_index = Some(idx);
                best_parts = (n, pv, reach);
            }
        }
        let Some(best_index) = best_index else {
            pick += 1;
            continue;
        };
        let i = pool.remove(best_index);
        records[i].draft_pick = Some(pick);
        let grade = grade_of(records, i);
        let consensus = entrants
            .iter()
            .filter(|&&e| !records[e].player.position.is_specialist() && grade_of(records, e) > grade)
            .count()
            + 1;
        let pos = records[i].player.position;
        let mut parts: Vec<String> = Vec::new();
        let (n, pv, reach) = best_parts;
        if n >= 1.0 {
            parts.push(format!("they needed a {pos}"));
        }
        if n <= -3.0 {
            parts.push(format!("even with a starting {pos} in place"));
        }
        if pv >= 1.0 {
            parts.push(format!("{pos}s are valued highly"));
        }
        if pv <= -0.5 {
            parts.push(format!("{pos}s are valued less"));
        }
        if reach > 0.0 {
            parts.push("a strong team reaching for a rare talent at its one weak spot".into());
        }
        let tm = &teams[t];
        let why = if parts.is_empty() { String::new() } else { format!(": {}", parts.join(", ")) };
        records[i].note(format!(
            "Drafted {pick}{} (round {round}) by the {} (GM {}) as the {consensus}{} prospect on the consensus board{why}.",
            ordinal(pick),
            tm.name,
            tm.draft_skill,
            ordinal(consensus as u32)
        ));
        join(records, teams, &mut signed, i, t, round, 4);
        pick += 1;
    }
    // Undrafted free agents sign where they see the clearest path to a roster spot.
    let udfa_slots = round_half_up(draft_slots as f64 * PIPELINE.udfa_ratio) as usize;
    let mut signed_udfa = 0;
    let mut rest: Vec<usize> = pool.iter().chain(kickers.iter()).copied().collect();
    rest.sort_by(|&a, &b| grade_of(records, b).total_cmp(&grade_of(records, a)));
    for i in rest {
        if signed_udfa >= udfa_slots {
            break;
        }
        if records[i].player.overall() < 48.0 || records[i].draft_pick.is_some() {
            continue;
        }
        let best = best_opening(&records[i], i, teams, rng, None);
        if records[i].player.position.is_specialist() && best.slot > 0 {
            continue;
        }
        records[i].udfa = true;
        signed_udfa += 1;
        let name = teams[best.team].name;
        records[i]
            .note(format!("Undrafted, he signed with the {name}, where he saw the clearest path to a roster spot."));
        join(records, teams, &mut signed, i, best.team, 8, 3);
    }
    signed
}

struct Opening {
    team: usize,
    slot: usize,
}

/// The team where a player would rank highest on the depth chart, with some noise for interest and money.
fn best_opening(r: &CareerRecord, i: usize, teams: &[Team], rng: &mut Rng, exclude: Option<usize>) -> Opening {
    let ovr = r.player.overall();
    let pos = r.player.position.index();
    let me = Some(i as u32);
    let mut best: Option<(usize, usize, f64)> = None;
    for team in teams {
        if Some(team.id) == exclude {
            continue;
        }
        let seen = ovr + rng.normal(0.0, board_noise(team.draft_skill) * 0.5);
        let slot = team.depth[pos].iter().filter(|m| m.id != me && m.rating > seen).count();
        let v = -(slot as f64) + rng.normal(0.0, 0.8) + (team.organisation() - 10.5) * 0.05;
        if best.is_none_or(|b| v > b.2) {
            best = Some((team.id, slot, v));
        }
    }
    let (team, slot, _) = best.expect("at least one other team");
    Opening { team, slot }
}

// --- Pro careers --------------------------------------------------------------

struct ProState {
    rec: usize,
    /// Current team (the last one while a free agent).
    team: usize,
    round: u32,
    contract_years: i32,
    free_agent: bool,
    unsigned_years: u32,
    first_year: bool,
    new_team: bool,
    done: bool,
    last_perf: Option<f64>,
    /// A bad situation at the current team has been noted.
    stuck_noted: bool,
    /// Released in this year's camp: other teams can still claim him before the season.
    just_released: bool,
}

impl Member {
    fn class_player(rating: f64, age: u32, rec: usize) -> Self {
        Member { rating, age, id: Some(rec as u32), role: None, depth: None }
    }
}

fn member_of(teams: &mut [Team], team: usize, pos: Position, rec: usize) -> Option<&mut Member> {
    teams[team].depth[pos.index()].iter_mut().find(|m| m.id == Some(rec as u32))
}

fn remove_member(teams: &mut [Team], team: usize, pos: Position, rec: usize) {
    let list = &mut teams[team].depth[pos.index()];
    if let Some(k) = list.iter().position(|m| m.id == Some(rec as u32)) {
        list.remove(k);
    }
}

fn depth_name(pos: Position, slot: usize) -> String {
    format!("{pos}{}", slot + 1)
}

/// Every pro season of the class, year by year, inside the league: free
/// agency, camp battles for depth chart spots, development, injuries and
/// retirement. The league's background players age and change around them.
fn simulate_league_years(
    records: &mut [CareerRecord],
    prngs: &mut [Rng],
    signed: Vec<Signing>,
    teams: &mut [Team],
    rng: &mut Rng,
) {
    let mut states: Vec<ProState> = signed
        .into_iter()
        .map(|s| ProState {
            rec: s.rec,
            team: s.team,
            round: s.round,
            contract_years: s.contract_years,
            free_agent: false,
            unsigned_years: 0,
            first_year: true,
            new_team: true,
            done: false,
            last_perf: None,
            stuck_noted: false,
            just_released: false,
        })
        .collect();
    for s in &states {
        records[s.rec].running_qb = is_running_qb(&records[s.rec].player);
    }
    for year in 0..30 {
        if !states.iter().any(|s| !s.done) {
            break;
        }
        if year > 0 {
            advance_league(teams, rng);
        }
        let league_strength = teams.iter().map(Team::strength).sum::<f64>() / teams.len() as f64;
        let live: Vec<usize> = (0..states.len()).filter(|&k| !states[k].done).collect();
        // Free agency: expired contracts and released players find teams. Own team first if it still wants him.
        for &k in &live {
            let s = &mut states[k];
            if !s.free_agent && s.contract_years > 0 {
                continue;
            }
            let i = s.rec;
            let r = &mut records[i];
            let pos = r.player.position;
            let ovr = r.player.overall();
            let own = if s.free_agent { None } else { Some(s.team) };
            let own_slot = own.map_or(usize::MAX, |t| {
                teams[t].depth[pos.index()].iter().filter(|m| m.id != Some(i as u32) && m.rating > ovr).count()
            });
            let market = best_opening(r, i, teams, rng, own);
            let DepthSlots { starters, rotation, roster } = depth_slots(pos);
            let p_chance = |slot: usize| -> f64 {
                if slot < starters {
                    0.95
                } else if slot < starters + rotation {
                    0.85
                } else if slot < roster {
                    0.55
                } else {
                    0.0
                }
            };
            // Teams keep most of their own players; a player leaves mainly for a starting job elsewhere.
            let starts_elsewhere = market.slot < starters && own_slot >= starters && rng.chance(0.7);
            if own.is_some()
                && own_slot < roster
                && !starts_elsewhere
                && rng.chance((p_chance(own_slot) + 0.25).min(0.95))
            {
                s.contract_years = contract_length(own_slot, pos, rng);
                continue;
            }
            if rng.chance(p_chance(market.slot)) {
                if let Some(t) = own {
                    remove_member(teams, t, pos, i);
                }
                let from = if s.free_agent {
                    "As a free agent".to_string()
                } else {
                    format!("When his deal with the {} ended", teams[s.team].name)
                };
                s.team = market.team;
                teams[market.team].depth[pos.index()].push(Member::class_player(ovr, r.player.age, i));
                s.contract_years = contract_length(market.slot, pos, rng);
                s.free_agent = false;
                s.new_team = true;
                s.stuck_noted = false;
                r.pro_teams.push(market.team);
                let tm = &teams[market.team];
                let fresh =
                    if r.notes.iter().any(|n| n.contains("stuck in a bad situation")) && tm.organisation() >= 12.0 {
                        " A better-run team gave him a fresh start."
                    } else {
                        ""
                    };
                r.note(format!(
                    "{from} at {}, he signed with the {} (organisation {}/20).{fresh}",
                    r.player.age,
                    tm.name,
                    round_half_up(tm.organisation())
                ));
            } else if !s.free_agent {
                remove_member(teams, s.team, pos, i);
                s.free_agent = true;
            }
        }
        // Camp: each team ranks everyone at a position on what it sees.
        for t in 0..teams.len() {
            for pos in Position::ALL {
                let ours: Vec<usize> = live
                    .iter()
                    .copied()
                    .filter(|&k| {
                        !states[k].free_agent && states[k].team == t && records[states[k].rec].player.position == pos
                    })
                    .collect();
                if ours.is_empty() {
                    continue;
                }
                camp_battle(&mut teams[t], pos, &ours, &mut states, records, rng);
            }
        }
        // A starter-calibre player stuck behind better players is traded to a team that will start him.
        for &k in &live {
            let s = &mut states[k];
            let i = s.rec;
            let pos = records[i].player.position;
            if s.free_agent {
                continue;
            }
            let role = member_of(teams, s.team, pos, i).and_then(|m| m.role);
            if role == Some(PlayingTime::Starter) || role == Some(PlayingTime::PracticeSquad) {
                continue;
            }
            let r = &mut records[i];
            if r.player.overall() < NFL_LEVELS.starter + 3.0 {
                continue;
            }
            let offer = best_opening(r, i, teams, rng, Some(s.team));
            if offer.slot >= depth_slots(pos).starters || !rng.chance(0.6) {
                continue;
            }
            let from = teams[s.team].name;
            remove_member(teams, s.team, pos, i);
            s.team = offer.team;
            let mut m = Member::class_player(r.player.overall(), r.player.age, i);
            m.role = Some(PlayingTime::Starter);
            m.depth = Some(depth_name(pos, offer.slot));
            teams[offer.team].depth[pos.index()].push(m);
            s.new_team = true;
            s.stuck_noted = false;
            r.pro_teams.push(offer.team);
            r.note(format!(
                "Stuck behind better players with the {from} at {} (rated {}), he was traded to the {}, who started him.",
                r.player.age,
                round_half_up(r.player.overall()),
                teams[offer.team].name
            ));
        }
        // Players released in camp can be claimed by a team with a need before the season.
        for &k in &live {
            let s = &mut states[k];
            if !s.just_released {
                continue;
            }
            s.just_released = false;
            let i = s.rec;
            let r = &mut records[i];
            let pos = r.player.position;
            let claim = best_opening(r, i, teams, rng, Some(s.team));
            let DepthSlots { starters, rotation, roster } = depth_slots(pos);
            // Waiver claims are for younger players; a released veteran has to wait for free agency.
            if claim.slot >= roster || r.player.age >= 30 || !rng.chance(0.6) {
                continue;
            }
            s.team = claim.team;
            let mut m = Member::class_player(r.player.overall(), r.player.age, i);
            m.role = Some(if claim.slot < starters {
                PlayingTime::Starter
            } else if claim.slot < starters + rotation {
                PlayingTime::Rotation
            } else {
                PlayingTime::Backup
            });
            m.depth = Some(depth_name(pos, claim.slot));
            teams[claim.team].depth[pos.index()].push(m);
            s.free_agent = false;
            s.new_team = true;
            s.stuck_noted = false;
            s.contract_years = 1;
            r.pro_teams.push(claim.team);
            r.note(format!("The {} claimed him before the season.", teams[claim.team].name));
        }
        for &k in &live {
            let s = &mut states[k];
            let i = s.rec;
            let r = &mut records[i];
            let pos = r.player.position;
            if s.free_agent && r.player.age <= 32 && rng.chance(0.15) {
                // Injuries open spots during the season; a free agent near the bar gets a call.
                let call = best_opening(r, i, teams, rng, None);
                if call.slot < depth_slots(pos).roster {
                    s.team = call.team;
                    let mut m = Member::class_player(r.player.overall(), r.player.age, i);
                    m.role = Some(PlayingTime::Backup);
                    m.depth = Some("signed mid-season".into());
                    teams[call.team].depth[pos.index()].push(m);
                    s.free_agent = false;
                    s.new_team = true;
                    s.stuck_noted = false;
                    s.contract_years = 1;
                    r.pro_teams.push(call.team);
                    r.note(format!(
                        "Unsigned in camp at {}, he was signed mid-season by the {} after injuries there.",
                        r.player.age, teams[call.team].name
                    ));
                }
            }
            let (role, depth) = if s.free_agent {
                (PlayingTime::Unsigned, "free agent".to_string())
            } else {
                let m = member_of(teams, s.team, pos, i);
                let role = m.as_ref().and_then(|m| m.role).unwrap_or(PlayingTime::Backup);
                let depth = m.and_then(|m| m.depth.clone()).unwrap_or_else(|| "backup".into());
                (role, depth)
            };
            let team = if s.free_agent { None } else { Some(s.team) };
            let coaching = match team {
                Some(t) => clamp(teams[t].coaching + rng.normal(0.0, 1.5), 1.0, 20.0),
                None => 6.0,
            };
            let mut adjustment = 0.0;
            if team.is_some() && (s.first_year || s.new_team) && role != PlayingTime::Unsigned {
                let from = match r.college {
                    Some(CollegeLevel::FCS) => Step::FcsToNfl,
                    Some(CollegeLevel::SmallCollege) => Step::SmallCollegeToNfl,
                    _ => Step::FbsToNfl,
                };
                let adj = first_year_adjustment(&r.player, if s.first_year { from } else { Step::NewTeam }, rng);
                adjustment = adj.dip;
                if s.first_year {
                    note_dip(r, &adj, "the NFL");
                }
            }
            let situation = team.map_or(0.0, |t| situation_at(&teams[t], &r.player, role, league_strength));
            if let Some(t) = team
                && situation <= -0.45
                && r.player.age <= 25
                && s.round <= 3
                && matches!(role, PlayingTime::Starter | PlayingTime::Rotation)
                && !s.stuck_noted
            {
                s.stuck_noted = true;
                let tm = &teams[t];
                r.note(format!(
                        "At {} he was stuck in a bad situation with the {} (organisation {}/20, a weak roster around him), which dragged his play and his development.",
                        r.player.age,
                        tm.name,
                        round_half_up(tm.organisation())
                    ));
            }
            let ctx = SeasonContext { adjustment, situation, ..SeasonContext::new(Level::NFL, coaching, role) };
            let report = develop_season(&mut r.player, &ctx, &mut prngs[i]);
            let row = r.player.history.last_mut().unwrap();
            row.depth = Some(depth);
            if let Some(t) = team {
                row.situation = Some(round_half_up(situation * 100.0) / 100.0);
                row.team = Some(t);
            }
            let perf = row.performance;
            track_peak(r);
            add_wear(r, role, 1.0);
            if matches!(role, PlayingTime::Starter | PlayingTime::Rotation | PlayingTime::Backup) {
                r.nfl_seasons += 1;
            }
            if role == PlayingTime::PracticeSquad {
                r.practice_squad_seasons += 1;
            }
            if !s.free_agent {
                let age = r.player.age;
                if let Some(m) = member_of(teams, s.team, pos, i) {
                    m.rating = report.overall_after;
                    m.age = age;
                }
                s.contract_years -= 1;
                s.unsigned_years = 0;
            } else {
                s.unsigned_years += 1;
            }
            s.last_perf = perf;
            s.first_year = false;
            s.new_team = false;
            if retires(s, r, &report, rng) {
                s.done = true;
                r.pro_exit = Some(if s.free_agent { ProExit::Unsigned } else { ProExit::Retired });
                if !s.free_agent {
                    remove_member(teams, s.team, pos, i);
                }
            }
        }
    }
    for s in &states {
        if !s.done {
            records[s.rec].pro_exit = Some(ProExit::Retired);
        }
    }
}

fn contract_length(slot: usize, pos: Position, rng: &mut Rng) -> i32 {
    if slot < depth_slots(pos).starters { rng.int(2, 4) } else { rng.int(1, 2) }
}

/// Camp and preseason: everyone at a position is ranked on what the staff sees
/// (rating, last season's production, a camp that can go well or badly, read
/// through the GM's eyes). Rookie contracts buy early picks patience, and
/// teams lean young on close calls. The best make the 53, the rest of the
/// class's players go to the practice squad or free agency; background
/// players who lose out are released.
fn camp_battle(
    team: &mut Team,
    pos: Position,
    ours: &[usize],
    states: &mut [ProState],
    records: &mut [CareerRecord],
    rng: &mut Rng,
) {
    let noise = board_noise(team.draft_skill) * 0.5;
    // Camp brings rookies and street free agents from other classes (a 90-man
    // camp roster for 53 places): the class's players have to beat them too.
    let DepthSlots { starters, rotation, roster } = depth_slots(pos);
    // Kickers and punters last into their late thirties, so teams don't favour youth there.
    let youth_bias = if pos.is_specialist() { 0.0 } else { 0.3 };
    let talent = (team.strength() - 78.0) / 3.0;
    let team_name = team.name;
    let members = &mut team.depth[pos.index()];
    for _ in 0..(roster as f64 * 0.7).ceil() as usize {
        let rating = clamp(rng.normal(68.5 + 1.5 * talent, 4.5), 45.0, 85.0);
        let age = rng.int(22, 25) as u32;
        members.push(Member::background(rating, age));
    }
    let state_of = |m: &Member, states: &[ProState]| {
        m.id.and_then(|id| ours.iter().copied().find(|&k| states[k].rec as u32 == id))
    };
    let mut play = vec![0.0; members.len()];
    let mut keep = vec![0.0; members.len()];
    for (j, m) in members.iter().enumerate() {
        // Last year's starter keeps the job unless someone clearly beats him; teams rarely flip quarterbacks.
        // Staffs also expect an ageing veteran to fade and lean towards the younger man.
        let incumbent = if m.role == Some(PlayingTime::Starter) && m.age < 32 {
            if pos == Position::QB { 3.0 } else { 1.5 }
        } else {
            0.0
        };
        let age = m.age as f64;
        let fading = if pos.is_specialist() { 0.0 } else { (age - 30.0).max(0.0) * 0.6 };
        let Some(k) = state_of(m, states) else {
            let v = m.rating + rng.normal(0.0, noise);
            play[j] = v + incumbent - fading;
            keep[j] = v + (27.0 - age) * youth_bias - if age >= 29.0 && youth_bias > 0.0 { 1.0 } else { 0.0 };
            continue;
        };
        let s = &states[k];
        let r = &records[s.rec];
        let p = &r.player;
        let ovr = p.overall();
        // A player still adjusting to the level (or a new scheme) looks it in camp.
        let camp = ovr + rng.normal(0.0, 2.5) + (p.personality.competitiveness - 10.0) * 0.15
            - if s.first_year {
                4.0
            } else if s.new_team {
                1.0
            } else {
                0.0
            };
        // Years of NFL film make a player easier to judge: a proven starter is never badly misread.
        let seen = 0.55 * ovr
            + 0.25 * s.last_perf.unwrap_or(ovr)
            + 0.2 * camp
            + rng.normal(0.0, noise / (1.0 + 0.4 * r.nfl_seasons as f64));
        let protection = if r.nfl_seasons + r.practice_squad_seasons < 3 && s.round <= 8 {
            PIPELINE.rookie_protection[s.round as usize]
        } else {
            0.0
        };
        // Teams give their top picks a real shot at starting.
        play[j] = seen + incumbent - fading + if s.first_year && s.round == 1 { 2.0 } else { 0.0 };
        // A veteran backup costs more than a rookie who plays as well.
        let cost =
            if r.nfl_seasons >= 4 && ovr < NFL_LEVELS.starter { 0.5 * (r.nfl_seasons as f64 - 3.0) } else { 0.0 };
        keep[j] = seen
            + protection
            + if m.role == Some(PlayingTime::Starter) { 3.0 } else { 0.0 }
            + (27.0 - p.age as f64) * youth_bias
            - cost.min(3.0);
    }
    let mut by_keep: Vec<usize> = (0..members.len()).collect();
    by_keep.sort_by(|&a, &b| keep[b].total_cmp(&keep[a]));
    let made = roster.min(by_keep.len());
    let mut by_play: Vec<usize> = by_keep[..made].to_vec();
    by_play.sort_by(|&a, &b| play[b].total_cmp(&play[a]));
    for (slot, &j) in by_play.iter().enumerate() {
        let m = &mut members[j];
        m.role = Some(if slot < starters {
            PlayingTime::Starter
        } else if slot < starters + rotation {
            PlayingTime::Rotation
        } else {
            PlayingTime::Backup
        });
        m.depth = Some(depth_name(pos, slot));
    }
    // Released background players leave; the class's players go to the practice squad or free agency.
    let mut remove = vec![false; members.len()];
    let cut_line = by_keep.get(made.wrapping_sub(1)).map_or(f64::NEG_INFINITY, |&j| keep[j]);
    for &j in &by_keep[made..] {
        let Some(k) = state_of(&members[j], states) else {
            remove[j] = true;
            continue;
        };
        let s = &mut states[k];
        let r = &mut records[s.rec];
        let age = r.player.age;
        // Practice squads are for players with at most two seasons on a roster.
        let young = r.nfl_seasons <= 2 && r.practice_squad_seasons < 3;
        let close = keep[j] >= cut_line - 7.0;
        if young && close && age <= 26 && rng.chance(0.75) {
            members[j].role = Some(PlayingTime::PracticeSquad);
            members[j].depth = Some("practice squad".into());
            if r.practice_squad_seasons == 0 {
                r.note(format!(
                    "Cut from the 53 by the {team_name} at {age}, he stayed on as a practice squad player."
                ));
            }
        } else {
            remove[j] = true;
            s.free_agent = true;
            s.just_released = true;
            s.contract_years = 0;
            r.note(format!(
                "Released by the {team_name} at {age} (rated {}), he went into free agency.",
                round_half_up(r.player.overall())
            ));
        }
    }
    let mut j = 0;
    members.retain(|_| {
        let keep_it = !remove[j];
        j += 1;
        keep_it
    });
}

/// How his surroundings help or hurt him, -1 to 1: roster around him and how well the team is run.
fn situation_at(team: &Team, p: &Player, role: PlayingTime, league_strength: f64) -> f64 {
    if matches!(role, PlayingTime::PracticeSquad | PlayingTime::Unsigned) {
        return 0.0;
    }
    let cast = (team.strength() - league_strength) / 1.8;
    let org = (team.organisation() - 11.5) / 2.5;
    // Quarterbacks and receivers depend most on the people around them.
    let reliance = match p.position {
        Position::QB => 1.0,
        Position::WR | Position::TE | Position::RB => 0.7,
        _ => 0.4,
    };
    clamp(reliance * (0.5 * cast + 0.5 * org), -1.0, 1.0)
}

/// Yearly wear: backs take the most hits, quarterbacks who run take more than pocket passers.
fn add_wear(r: &mut CareerRecord, role: PlayingTime, scale: f64) {
    let share = match role {
        PlayingTime::Starter => 1.0,
        PlayingTime::Rotation => 0.5,
        _ => return,
    };
    let per_season = match r.player.position {
        Position::RB => 1.0,
        Position::QB => {
            if r.running_qb {
                0.6
            } else {
                0.15
            }
        }
        Position::K | Position::P => 0.0,
        _ => 0.35,
    };
    r.player.wear += per_season * share * scale;
}

fn is_running_qb(p: &Player) -> bool {
    p.position == Position::QB
        && (p.attributes[Attr::Speed] + p.attributes[Attr::Acceleration] + p.attributes[Attr::Agility]) / 3.0 >= 78.0
}

/// Retirement: each position has a typical age to walk away, moved by his
/// longevity (rare bodies like Derrick Henry), love of the game, the wear on
/// his body, and whether he is still good. A player who goes unsigned
/// usually retires within a year or two.
fn retires(s: &ProState, r: &mut CareerRecord, report: &SeasonReport, rng: &mut Rng) -> bool {
    let p = &r.player;
    let age = p.age as f64;
    let love = p.personality.love_of_game;
    if s.free_agent {
        if s.unsigned_years >= 2 {
            r.note(format!("Unsigned for two seasons, he retired at {age}."));
            return true;
        }
        let base = if p.position.is_specialist() {
            if age >= 36.0 { 0.6 } else { 0.25 }
        } else if age >= 30.0 {
            0.7
        } else if age >= 27.0 {
            0.5
        } else {
            0.3
        };
        if rng.chance(base * clamp(1.3 - love * 0.03, 0.6, 1.3)) {
            r.note(format!("Unsigned for a season, he retired at {age}."));
            return true;
        }
        return false;
    }
    let wear_drag = clamp(1.0 - 0.15 * p.longevity, 0.5, 1.5);
    let typical = retire_age(p.position)
        + if r.running_qb { PIPELINE.running_qb_retire_offset } else { 0.0 }
        + p.longevity * 0.8
        + (love - 10.0) * 0.2
        - (p.wear - 6.0).max(0.0) * 0.3 * wear_drag;
    let after = report.overall_after;
    // Still being good keeps a player going, but only for a few years past the usual age.
    let mut standing = if after >= 85.0 {
        0.35
    } else if after >= 78.0 {
        0.6
    } else {
        1.0
    };
    if age > typical + 2.0 {
        standing += (1.0 - standing) * ((age - typical - 2.0) / 3.0).min(1.0);
    }
    let major = if report.events.has(Ev::MajorInjury) && age >= 27.0 { 0.12 } else { 0.0 };
    let chance = (0.008 + 0.5 / (1.0 + (-(age - typical) / 1.3).exp())) * standing
        + major
        + (age - typical - 4.0).max(0.0) * 0.08;
    if !rng.chance(chance) {
        return false;
    }
    // Only name the reasons that point the way his career went: why he lasted, or why he left early.
    let mut why: Vec<String> = Vec::new();
    let late = age > typical + 1.0;
    let early = age < typical - 1.0;
    if early && r.running_qb {
        why.push("a quarterback who relied on his legs".into());
    }
    if early && p.wear >= 7.0 {
        why.push(format!("a heavy workload had worn his body down (wear {:.1})", p.wear));
    }
    if early && p.longevity <= -1.5 {
        why.push(format!("a body that aged early (longevity {:.1})", p.longevity));
    }
    if early && love <= 6.0 {
        why.push(format!("little love of the grind ({love}/20)"));
    }
    if late && p.longevity >= 1.5 {
        why.push(format!("a body that aged slowly (longevity +{:.1})", p.longevity));
    }
    if late && love >= 15.0 {
        why.push(format!("a deep love of the game ({love}/20)"));
    }
    if late && after >= 85.0 {
        why.push(format!("still one of the best at his position (rated {})", round_half_up(after)));
    }
    if major > 0.0 {
        why.push("a major injury late in his career".into());
    }
    let odds = if chance >= 0.2 {
        String::new()
    } else {
        format!(", a {}% chance that year", round_half_up(chance * 100.0).max(1.0))
    };
    let reasons = if why.is_empty() { String::new() } else { format!(": {}", why.join(", ")) };
    r.note(format!(
        "Retired at {age}, rated {} (typical for his case: about {}{odds}){reasons}.",
        round_half_up(after),
        round_half_up(typical)
    ));
    true
}

/// Fill in each season's level rating against the national pool for his level
/// and age, using the rating he played that season at. The end-of-year level
/// ranks his rating at the end of the season against everyone else's at the
/// end of theirs.
fn attach_level_ratings(records: &mut [CareerRecord], class_share: f64) {
    type Scales = HashMap<LevelGroup, (LevelScale, PositionAligner)>;
    let build_scales = |records: &[CareerRecord], at_end: bool| -> Scales {
        let mut groups: HashMap<LevelGroup, Vec<(Position, f64)>> = HashMap::new();
        for r in records {
            for s in &r.player.history {
                let Some(key) = level_group(s.level, s.age) else { continue };
                if !sets_level_scale(s.level) {
                    continue;
                }
                groups
                    .entry(key)
                    .or_default()
                    .push((r.player.position, if at_end { s.overall } else { s.overall_start }));
            }
        }
        groups
            .into_par_iter()
            .map(|(key, samples)| {
                let align = PositionAligner::new(&samples);
                let aligned: Vec<f64> = samples.iter().map(|&(pos, x)| align.align(pos, x)).collect();
                let pool = if key == LevelGroup::College { Pool::College } else { Pool::HS };
                (key, (build_level_scale(&aligned, pool, class_share), align))
            })
            .collect()
    };
    let at_start = build_scales(records, false);
    let at_end = build_scales(records, true);
    records.par_iter_mut().for_each(|r| {
        let pos = r.player.position;
        for s in r.player.history.iter_mut() {
            let level = s.level;
            let key = level_group(level, s.age);
            let discount = match level {
                Level::FCS => competition_discount(Some(CollegeLevel::FCS)),
                Level::SmallCollege => competition_discount(Some(CollegeLevel::SmallCollege)),
                _ => 0.0,
            };
            let rate = |scales: &Scales, ovr: f64| {
                let group = key.and_then(|k| scales.get(&k));
                let x = group.map_or(ovr, |(_, align)| align.align(pos, ovr - discount));
                level_rating(x, level, group.map(|g| &g.0))
            };
            s.level_rating = Some(rate(&at_start, s.overall_start));
            s.level_rating_end = Some(rate(&at_end, s.overall));
        }
    });
}

/// Last pick of each round in a real 257-pick draft (2025). Rounds 1 and 2 have
/// 32 picks; compensatory picks make rounds 3-7 longer.
const ROUND_ENDS: [f64; 7] = [32.0, 64.0, 102.0, 138.0, 176.0, 216.0, 257.0];

/// Round of a pick, with the real round sizes scaled to the class's number of picks.
pub fn draft_round(pick: u32, draft_slots: u32) -> u32 {
    let scale = draft_slots as f64 / ROUND_ENDS[ROUND_ENDS.len() - 1];
    ROUND_ENDS
        .iter()
        .position(|&end| pick as f64 <= round_half_up(end * scale))
        .map_or(ROUND_ENDS.len() as u32, |i| i as u32 + 1)
}

/// Rating guidance for the pro game. These are what the user sees as rough
/// bands; actual roster decisions are made by teams through their own eyes.
pub struct NflLevels {
    pub roster: f64,
    pub starter: f64,
    pub pro_bowl: f64,
    pub elite: f64,
}
pub const NFL_LEVELS: NflLevels = NflLevels { roster: 65.0, starter: 75.0, pro_bowl: 85.0, elite: 92.0 };

/// How far off a front office's read of a player on its own roster is, in rating points.
fn board_noise(skill: f64) -> f64 {
    1.5 + (20.0 - skill) * 0.35
}

/// How far off a front office's draft grade is, in rating points: 1 for the
/// best, 3.5 for an average team and about 6 for the worst. Prospects are
/// scouted for years and graded by many people, so an average team's board
/// strays less than its read of a camp body. Real drafts take 1-3 first-
/// rounders a year from outside the consensus top 50.
pub(crate) fn draft_board_noise(skill: f64) -> f64 {
    1.0 + (20.0 - skill) * 0.25
}

/// Yearly chance of off-field or academic trouble in college.
fn trouble_chance(p: &Player) -> f64 {
    let pers = &p.personality;
    PIPELINE.college_trouble.chance
        * clamp(1.0 + (10.0 - pers.work_ethic) * 0.12 + (10.0 - pers.coachability) * 0.06, 0.25, 3.0)
}

#[derive(Clone, Copy, Debug)]
struct DraftScore {
    score: f64,
    overall_seen: f64,
    potential_seen: f64,
}

/// What a draft room thinks of a prospect: current ability plus some projection. Specialists are graded on their own board.
fn draft_score(r: &CareerRecord, rng: &mut Rng) -> DraftScore {
    let level = r.college.map_or(Level::HS, CollegeLevel::level);
    let rep = scout_player(&r.player, ScoutOptions { accuracy: 13.0, level }, rng);
    let youth = clamp((23.0 - r.player.age as f64) / 3.0, 0.0, 1.0);
    let competition = competition_discount(r.college);
    // Off-field and academic red flags cost a player on every board.
    let flags: f64 = r
        .player
        .history
        .iter()
        .map(|s| {
            if s.events.has(Ev::Dismissed) {
                PIPELINE.red_flag_discount.dismissed
            } else if s.events.has(Ev::Suspended) {
                PIPELINE.red_flag_discount.suspended
            } else {
                0.0
            }
        })
        .sum();
    let score = rep.overall + (0.25 + 0.2 * youth) * (rep.potential - rep.overall) - competition - flags;
    DraftScore { score, overall_seen: rep.overall, potential_seen: rep.potential }
}

/// Playing time by rank within each position: a quarterback competes with quarterbacks.
/// Takes each player's position and how he is seen; returns roles in the same order.
fn roles_by_rank(seen: &[(Position, f64)], starter_share: f64, rotation_share: f64) -> Vec<PlayingTime> {
    let mut by_position: [Vec<usize>; 13] = Default::default();
    for (k, &(pos, _)) in seen.iter().enumerate() {
        by_position[pos.index()].push(k);
    }
    let mut roles = vec![PlayingTime::Backup; seen.len()];
    for mut g in by_position {
        g.sort_by(|&a, &b| seen[b].1.total_cmp(&seen[a].1));
        let n = g.len() as f64;
        for (rank, &k) in g.iter().enumerate() {
            let q = rank as f64 / n;
            roles[k] = if q < starter_share {
                PlayingTime::Starter
            } else if q < starter_share + rotation_share {
                PlayingTime::Rotation
            } else {
                PlayingTime::Backup
            };
        }
    }
    roles
}

/// Best rating he played a season at.
fn track_peak(r: &mut CareerRecord) {
    if let Some(last) = r.player.history.last()
        && last.overall_start > r.peak_overall
    {
        r.peak_overall = last.overall_start;
        r.peak_age = last.age;
    }
}

fn ordinal(n: u32) -> &'static str {
    let t = n % 100;
    if (11..=13).contains(&t) {
        return "th";
    }
    match n % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}
