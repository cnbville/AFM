//! The world's coaches as the figures the setup's roads screen sets you
//! against (update 3, lane 2, coaching C7; board 40): medians and top
//! tenths over every coach in a job at a level, never one coach's numbers.
//! Read from the world you start in, so the bar is that world's own.

use super::World;
use super::coach::{Coach, Employer, GROUP_COUNT, Role};
use super::data::Division;

/// The figures, each on the 1-20 scale every coach has.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoadMarks {
    /// High school head coaches: the median of each one's best teaching
    /// group, and the top tenth rating by rating over his other ratings.
    pub hs_best: f64,
    pub hs_other_top: f64,
    /// Position coaches at the 68 strongest FBS programs, by group: the
    /// median and the top tenth of their own group's teaching.
    pub fbs_group: [(f64, f64); GROUP_COUNT],
    /// Coordinators at the 68 strongest FBS programs: the median scheme.
    pub fbs_coord_scheme: f64,
    /// Coordinators' everything else (the median of each one's mean over
    /// his other ratings): the top 68, FCS, and the small colleges.
    pub coord_other: [f64; 3],
    /// The NFL: (coordinators' median, head coaches' median) of scheme and
    /// of play calling; (head coaches' median evaluation, the best scouting
    /// director's eye).
    pub nfl_scheme: (f64, f64),
    pub nfl_calling: (f64, f64),
    pub nfl_eval: (f64, f64),
}

fn quantile(v: &mut [f64], f: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    let x = v[((v.len() - 1) as f64 * f).round() as usize];
    (x * 10.0).round() / 10.0
}

/// A coach's ratings past teaching, as the engine reads them for an AI
/// coach: scheme, play calling (0.6 scheme + 0.4 game management, as
/// `coach_caller` reads it), game management, adaptation (his game
/// management), motivation, discipline, relationships, recruiting,
/// evaluation and network. Negotiating is his general manager's.
fn others(c: &Coach) -> [f64; 10] {
    let a = &c.ability;
    [
        a.scheme,
        0.6 * a.scheme + 0.4 * a.game_management,
        a.game_management,
        a.game_management,
        a.motivation,
        a.discipline,
        a.relationships,
        a.recruiting,
        c.evaluation(),
        a.network,
    ]
}

pub fn road_marks(w: &World) -> RoadMarks {
    let mut fbs: Vec<usize> = (0..w.programs.len()).filter(|&p| w.programs[p].division == Division::FBS).collect();
    fbs.sort_by(|&a, &b| w.programs[b].prestige.total_cmp(&w.programs[a].prestige).then(a.cmp(&b)));
    let mut top = vec![false; w.programs.len()];
    for &p in fbs.iter().take(68) {
        top[p] = true;
    }
    let mut hs_best = Vec::new();
    let mut hs_other: [Vec<f64>; 10] = Default::default();
    let mut groups: [Vec<f64>; GROUP_COUNT] = Default::default();
    let mut coord_scheme = Vec::new();
    let mut coord_other: [Vec<f64>; 3] = Default::default();
    let (mut nc_scheme, mut nh_scheme, mut nc_call, mut nh_call, mut nh_eval) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for c in w.coaches.iter().filter(|c| !c.retired && !c.is_yours()) {
        let Some(j) = c.job else { continue };
        let o = others(c);
        let mean = o.iter().sum::<f64>() / o.len() as f64;
        let coord = matches!(j.role, Role::OffensiveCoordinator | Role::DefensiveCoordinator);
        match j.employer {
            Employer::HighSchool(_) if j.role == Role::HeadCoach => {
                hs_best.push(c.ability.teaching.iter().copied().fold(0.0, f64::max));
                for (k, v) in o.iter().enumerate() {
                    hs_other[k].push(*v);
                }
            }
            Employer::College(p) => {
                let d = w.programs[p].division;
                if let Role::PositionCoach(g) = j.role
                    && top[p]
                {
                    groups[g as usize].push(c.ability.teaching[g as usize]);
                }
                if coord {
                    if top[p] {
                        coord_scheme.push(c.ability.scheme);
                        coord_other[0].push(mean);
                    } else if d == Division::FCS {
                        coord_other[1].push(mean);
                    } else if d != Division::FBS {
                        coord_other[2].push(mean);
                    }
                }
            }
            Employer::Nfl(_) => {
                let call = 0.6 * c.ability.scheme + 0.4 * c.ability.game_management;
                if coord {
                    nc_scheme.push(c.ability.scheme);
                    nc_call.push(call);
                } else if j.role == Role::HeadCoach {
                    nh_scheme.push(c.ability.scheme);
                    nh_call.push(call);
                    nh_eval.push(c.evaluation());
                }
            }
            _ => {}
        }
    }
    let best_director = w
        .clubs
        .iter()
        .flat_map(|c| c.scouts.iter())
        .filter(|s| s.role == super::pro::scouts::Role::Director)
        .map(|s| s.judge as f64)
        .fold(0.0, f64::max);
    let hs_other_top = {
        let tops: Vec<f64> = hs_other.iter_mut().map(|v| quantile(v, 0.9)).collect();
        (tops.iter().sum::<f64>() / tops.len() as f64 * 10.0).round() / 10.0
    };
    RoadMarks {
        hs_best: quantile(&mut hs_best, 0.5),
        hs_other_top,
        fbs_group: std::array::from_fn(|g| {
            let v = &mut groups[g];
            (quantile(v, 0.5), quantile(v, 0.9))
        }),
        fbs_coord_scheme: quantile(&mut coord_scheme, 0.5),
        coord_other: std::array::from_fn(|k| quantile(&mut coord_other[k], 0.5)),
        nfl_scheme: (quantile(&mut nc_scheme, 0.5), quantile(&mut nh_scheme, 0.5)),
        nfl_calling: (quantile(&mut nc_call, 0.5), quantile(&mut nh_call, 0.5)),
        nfl_eval: (quantile(&mut nh_eval, 0.5), best_director),
    }
}
