//! Moving players through the world: high school transfers and powerhouse
//! pulls (topic 2), exposure and recruiting (topics 4 and 5), JUCO and prep
//! school (topic 6, 1.6), the draft, the transfer portal, tampering and NIL
//! (topic 8).

use rayon::prelude::*;

use super::background::Decider;
use super::data::{CollegeRow, Division};
use super::program::{GradeCat, Program, record_impact};
use super::season::{SeasonResults, key_normal, key_uniform};
use super::{Alumnus, Person, Stage, World, division_level, geo};
use crate::career::{PIPELINE, Step, position_value};
use crate::player::{Level, PlayingTime};
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::clamp;
use crate::scouting::{ScoutOptions, scout_player};

// --- Data hooks -------------------------------------------------------------------

/// Junior college programs. Placeholder names until the JUCO data pass adds
/// the real programs (topic 6): counts follow where JUCO football is played.
pub(crate) fn juco_rows() -> Vec<CollegeRow> {
    const JUCO_STATES: [(&str, usize); 12] = [
        ("CA", 20),
        ("MS", 14),
        ("KS", 9),
        ("TX", 6),
        ("MN", 6),
        ("IA", 4),
        ("NY", 4),
        ("AZ", 2),
        ("OK", 1),
        ("ND", 1),
        ("MI", 1),
        ("UT", 1),
    ];
    let mut rows = Vec::new();
    for (state, n) in JUCO_STATES {
        for k in 0..n {
            rows.push(CollegeRow {
                name: format!("{state} Junior College {}", k + 1),
                division: Division::JUCO,
                conference: format!("{state} JUCO"),
                city: state.to_string(),
                state: state.to_string(),
                enrollment: 5000,
                national_titles: 0,
                last_5yr_win_pct: Some(0.5),
                stadium_capacity: Some(5000),
                academic_tier: super::data::AcademicTier::Other,
            });
        }
    }
    rows
}

/// Whether a state lets high school players earn NIL money. Filled by the
/// per-state research pass (follow-up answer 5).
pub(crate) fn hs_nil_allowed(code: &str) -> bool {
    super::data::hs_nil(code)
}

// --- High school transfers ----------------------------------------------------------

/// Powerhouses pull standouts (topic 2): exposure, coaching, a better shot at
/// a big offer, sometimes a family move or an NIL perk. Loyalty, family ties,
/// cost, crowded depth charts and a strong local coach hold them back. A kid
/// may also move to escape a bad home (rare). Each state's transfer rule
/// decides whether he sits.
pub(crate) fn high_school_transfers(w: &mut World) {
    let year = w.year;
    w.rebuild_rosters();
    // Standouts by state: rising sophomores to seniors in the top 3% of their age.
    let n = w.people.len();
    let mut by_state: Vec<Vec<usize>> = vec![Vec::new(); 51];
    for i in 0..n {
        let p = &w.people[i];
        if p.stage == Stage::HighSchool && (15..=17).contains(&p.player.age) {
            by_state[p.home_state].push(i);
        }
    }
    let mut moves: Vec<(usize, usize, bool)> = Vec::new();
    for s in 0..51 {
        let list = &by_state[s];
        if list.is_empty() {
            continue;
        }
        let mut ovr: Vec<f64> = list.iter().map(|&i| w.people[i].player.overall()).collect();
        ovr.sort_by(|a, b| b.total_cmp(a));
        let bar = ovr[(ovr.len() as f64 * 0.03) as usize];
        let powerhouses: Vec<usize> =
            w.states[s].schools.iter().copied().filter(|&h| w.high_schools[h].is_powerhouse()).collect();
        let nil_state = w.states[s].hs_nil;
        for &i in list {
            let p = &w.people[i];
            let cur = &w.high_schools[p.hs];
            let standout = p.player.overall() >= bar;
            // Escaping a bad home: a rare pull that isn't about football.
            let escape = p.bg.safety < 0.3 && p.bg.home_stability < 0.35;
            if !standout && !escape {
                continue;
            }
            // Out of state, only the national powerhouses pull, and the family moves with him.
            let national = standout && p.player.overall() >= ovr[0] - 2.0 && w.rng.chance(0.15);
            let target = if national {
                let nat: Vec<usize> =
                    w.high_schools.iter().filter(|h| h.national && h.state != s).map(|h| h.id).collect();
                if nat.is_empty() { None } else { Some(nat[w.rng.int(0, nat.len() as i32 - 1) as usize]) }
            } else if !powerhouses.is_empty() {
                Some(powerhouses[w.rng.int(0, powerhouses.len() as i32 - 1) as usize])
            } else {
                None
            };
            let Some(t) = target else { continue };
            if t == p.hs || cur.is_powerhouse() && !escape {
                continue;
            }
            let dest = &w.high_schools[t];
            let gain = (dest.prestige - cur.prestige).max(0.0);
            let mut pull = 0.3 * gain;
            if nil_state || (national && w.states[dest.state].hs_nil) {
                pull *= 1.4;
            }
            // A strong local coach keeps his stars often.
            if let Some(c) = cur.coach {
                let coach = &w.coaches[c];
                if coach.ability().relationships >= 13.0 && coach.reputation >= 11.0 {
                    pull *= 0.35;
                }
            }
            pull *= 1.0 - 0.45 * p.bg.home_stability;
            if dest.private && !national {
                pull *= 0.45 + 0.55 * p.bg.money;
            }
            // Crowded depth chart at his position.
            let ahead = dest
                .roster
                .iter()
                .filter(|&&j| {
                    w.people[j].player.position == p.player.position
                        && w.people[j].player.overall() > p.player.overall()
                })
                .count();
            if ahead >= crate::league::depth_slots(p.player.position).starters {
                pull *= 0.5;
            }
            pull *= 1.0 + (p.player.personality.competitiveness - 10.0) * 0.04;
            if escape {
                pull += 0.015;
            }
            if w.rng.chance(clamp(pull, 0.0, 0.4)) {
                moves.push((i, t, national || escape && w.rng.chance(0.5)));
            }
        }
    }
    for (i, t, family_moved) in moves {
        let from = w.people[i].hs;
        let from_name = w.school_name(from).to_string();
        let to_name = w.school_name(t).to_string();
        let st = w.high_schools[t].state;
        let rule = w.states[st].transfer;
        let p = &mut w.people[i];
        let first = p.transfers == 0;
        p.sit_out = if family_moved || (rule.free_once && first) { 0.0 } else { rule.sit_out };
        p.transfers += 1;
        p.hs = t;
        if family_moved {
            p.home_state = st;
        }
        let age = p.player.age;
        let sit = if p.sit_out > 0.0 {
            format!(" and had to sit out part of the season under {}'s transfer rule", w.states[st].code)
        } else {
            String::new()
        };
        let why = if p.bg.safety < 0.3 && p.bg.home_stability < 0.35 {
            " to get away from a rough home situation"
        } else {
            ""
        };
        p.note(format!("At {age} he transferred from {from_name} to powerhouse {to_name}{why}{sit}."));
    }
    // Crowded powerhouse depth charts push some kids out again for playing time.
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool
            || !(16..=17).contains(&p.player.age)
            || p.last_role != Some(PlayingTime::Backup)
        {
            continue;
        }
        let cur = &w.high_schools[p.hs];
        if !cur.is_powerhouse() || !w.rng.chance(0.12) {
            continue;
        }
        let state = cur.state;
        let options: Vec<usize> = w.states[state]
            .schools
            .iter()
            .copied()
            .filter(|&h| !w.high_schools[h].is_powerhouse() && w.high_schools[h].prestige > 0.35)
            .collect();
        if options.is_empty() {
            continue;
        }
        let t = options[w.rng.int(0, options.len() as i32 - 1) as usize];
        let from_name = w.school_name(p.hs).to_string();
        let to_name = w.school_name(t).to_string();
        let rule = w.states[state].transfer;
        let p = &mut w.people[i];
        p.sit_out = if rule.free_once && p.transfers == 0 { 0.0 } else { rule.sit_out };
        p.transfers += 1;
        p.hs = t;
        let age = p.player.age;
        p.note(format!("At {age}, buried on {from_name}'s depth chart, he transferred to {to_name} to play."));
    }
    // Reclassifying: a few repeat a year or finish early to time their college entry.
    for p in w.people.iter_mut() {
        if p.stage != Stage::HighSchool || p.player.age != 16 {
            continue;
        }
        let r = p.rng.next();
        if r < 0.004 && p.player.overall() > 52.0 {
            p.class_of -= 1;
            p.note("Reclassified to graduate a year early.");
        } else if r > 0.995 {
            p.class_of += 1;
            p.note("Reclassified to repeat a year of high school.");
        }
    }
    let _ = year;
}

// --- College exits: awards, the draft ------------------------------------------------

/// What a position is worth on draft day, in board points (note 7). Teams pay
/// for quarterbacks, pass rushers and tackles; an elite one at the top of
/// the board is worth far more than an elite guard or safety. Calibrated
/// against the real 2000-2025 drafts (nflverse): about 73% of number 1
/// picks were quarterbacks, the rest edge rushers and tackles.
fn draft_position_value(pos: Position, grade: f64) -> f64 {
    // How much of the premium applies: full for the best prospects.
    // Teams reach for a quarterback: his premium starts lower down the board.
    let top = clamp((grade - if pos == Position::QB { 74.0 } else { 78.0 }) / 8.0, 0.0, 1.0);
    let (base, premium) = match pos {
        Position::QB => (0.3, 11.0),
        Position::EDGE => (1.6, 5.5),
        Position::OT => (0.6, 3.0),
        Position::WR => (0.4, 0.0),
        Position::CB => (1.0, 0.0),
        Position::DT => (0.6, 0.5),
        Position::RB => (-0.4, 0.0),
        Position::S => (-0.5, -3.0),
        Position::LB => (-0.3, -2.5),
        Position::IOL => (-1.6, -3.0),
        Position::TE => (-0.6, -3.0),
        Position::K | Position::P => (-25.0, 0.0),
    };
    base + premium * top
}

fn competition_discount(d: Division) -> f64 {
    match d {
        Division::FBS => 0.0,
        Division::FCS => 1.0,
        Division::D2 | Division::JUCO => 5.0,
    }
}

/// Credits an impact player to his program's pipelines and to the staff who
/// coached him: their ties to his high school travel with them (note 4).
fn credit_impact(w: &mut World, i: usize, c: usize, weight: f64, year: i32) {
    if !w.recording {
        return;
    }
    let (state, hs) = (w.people[i].home_state, w.people[i].hs);
    let area = w.high_schools[hs].area;
    record_impact(&mut w.programs[c], state, hs, area, weight, year);
    let group = super::coach::Group::of(w.people[i].player.position);
    let prog = &w.programs[c];
    let coord = if group.offense() { prog.oc } else { prog.dc };
    let staff = [prog.head_coach, coord, prog.position_coaches[group as usize]];
    for coach in staff.into_iter().flatten() {
        let t = w.coaches[coach].ties.entry(hs).or_insert(0.0);
        *t = clamp(*t + 0.2 * weight * (1.0 - *t), 0.0, 1.0);
    }
    // His position coach and coordinator get the visible credit (note 15B).
    for (coach, share) in [(prog.position_coaches[group as usize], 1.0), (coord, 0.5)] {
        if let Some(coach) = coach {
            w.coaches[coach].units[group as usize].moved_up += share * weight;
        }
    }
}

/// All-conference teams, national honors, then the draft. Only impact grows
/// a pipeline: all-conference or better, any draft pick, an undrafted pro
/// signing (topic 4, note 4).
pub(crate) fn college_exits(w: &mut World, _r: &SeasonResults) {
    let year = w.year;
    let draft_year = year + 1;
    // --- Impact seasons ---
    let mut honors: Vec<(usize, f64, &'static str)> = Vec::new();
    for division in [Division::FBS, Division::FCS, Division::D2] {
        let mut perf: Vec<(f64, usize)> = w
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.stage == Stage::College
                    && p.last_role == Some(PlayingTime::Starter)
                    && p.college.is_some_and(|c| w.programs[c].division == division)
            })
            .filter_map(|(i, p)| p.last_performance.map(|v| (v, i)))
            .collect();
        perf.sort_by(|a, b| b.0.total_cmp(&a.0));
        let leaders = (perf.len() as f64 * 0.02).ceil() as usize;
        let awards = (perf.len() as f64 * 0.003).ceil() as usize;
        for (k, &(_, i)) in perf.iter().take(leaders).enumerate() {
            if k < awards {
                honors.push((i, 1.0, "won national honors"));
            } else {
                honors.push((i, 0.75, "was among his level's top performers"));
            }
        }
        // All-conference: about the top tenth of the conference's starters,
        // judged against the conference, not the nation.
        let mut by_conf: std::collections::BTreeMap<&str, Vec<(f64, usize)>> = std::collections::BTreeMap::new();
        for &(v, i) in &perf {
            let c = w.people[i].college.unwrap();
            by_conf.entry(w.programs[c].conference.as_str()).or_default().push((v, i));
        }
        for list in by_conf.values() {
            let n = (list.len() as f64 * 0.1).ceil() as usize;
            for &(_, i) in list.iter().take(n) {
                if !honors.iter().any(|h| h.0 == i) {
                    honors.push((i, 0.5, "was named all-conference"));
                }
            }
        }
    }
    for (i, weight, what) in honors {
        let c = w.people[i].college.unwrap();
        credit_impact(w, i, c, weight, year);
        let age = w.people[i].player.age - 1;
        let school = w.program_name(c).to_string();
        w.people[i].note(format!("At {age} he {what} at {school}."));
    }

    // --- Declaring ---
    let mut entrants: Vec<(usize, f64)> = Vec::new();
    // The consensus draft grade of each entrant, before position and need.
    let mut grades: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    let seed = w.options.seed as u64;
    let n = w.people.len();
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::College {
            continue;
        }
        let Some(c) = p.college else { continue };
        let division = w.programs[c].division;
        if division == Division::JUCO {
            continue;
        }
        let years = draft_year - p.class_of;
        if years < 3 {
            continue;
        }
        let level = division_level(division);
        let p = &mut w.people[i];
        // Pro scouts list him where they see him playing on Sundays.
        if let Some(from) = crate::athlete::reconsider(&mut p.player, crate::athlete::Eye::NFL_SCOUTS, 0.005) {
            let to = p.player.position;
            p.note(format!("Played {from} in college; pro scouts project him at {to}."));
        }
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 13.0, level }, &mut p.rng);
        let youth = clamp((23.0 - p.player.age as f64) / 3.0, 0.0, 1.0);
        let flags: f64 = p
            .player
            .history
            .iter()
            .map(|s| {
                if s.events.has(crate::player::DevelopmentEvent::Dismissed) {
                    PIPELINE.red_flag_discount.dismissed
                } else if s.events.has(crate::player::DevelopmentEvent::Suspended) {
                    PIPELINE.red_flag_discount.suspended
                } else {
                    0.0
                }
            })
            .sum();
        let grade =
            rep.overall + (0.25 + 0.2 * youth) * (rep.potential - rep.overall) - competition_discount(division) - flags;
        grades.insert(i, grade);
        let score = grade + 0.5 * position_value(p.player.position)
            - if matches!(p.player.position, Position::K | Position::P) { 10.0 } else { 0.0 };
        let exhausted = p.college_seasons >= 4 || years >= 5;
        entrants.push((i, if exhausted { score } else { f64::NAN }));
        if !exhausted {
            // Underclassmen declare when the projection is good; a few gamble.
            entrants.last_mut().unwrap().1 = -score;
        }
    }
    // Underclassmen: projected high picks go; the rest return.
    let mut board: Vec<f64> = entrants.iter().map(|&(_, s)| s.abs()).collect();
    board.sort_by(|a, b| b.total_cmp(a));
    let slots = (PIPELINE.draft_picks * w.options.scale).round().max(1.0) as usize;
    let round2 = board.get((slots * 64 / 257).min(board.len().saturating_sub(1))).copied().unwrap_or(f64::MAX);
    let round4 = board.get((slots * 138 / 257).min(board.len().saturating_sub(1))).copied().unwrap_or(f64::MAX);
    let mut declared: Vec<(usize, f64)> = Vec::new();
    for &(i, s) in &entrants {
        if s >= 0.0 {
            declared.push((i, s));
            continue;
        }
        let score = -s;
        let p = &mut w.people[i];
        let chance = if score >= round2 {
            0.85
        } else if score >= round4 {
            0.35 + 0.02 * (p.family.decider_wants.money * 10.0)
        } else {
            0.04
        };
        if p.rng.chance(chance) {
            declared.push((i, score));
        }
    }
    // --- The draft: team by team (note 7) ---
    // Each team picks from its own board: the consensus grade (aligned across
    // positions, since positions mature at different ages), seen through its
    // front office, plus what the position is worth and what the team needs.
    let declared_ids: Vec<usize> = declared.iter().map(|d| d.0).collect();
    let align = crate::level_rating::PositionAligner::new(
        &declared_ids.iter().map(|&i| (w.people[i].player.position, grades[&i])).collect::<Vec<_>>(),
    );
    let mut pool: Vec<(usize, f64)> = declared_ids
        .iter()
        .map(|&i| {
            let g = align.align(w.people[i].player.position, grades[&i])
                + 1.0 * key_normal(seed ^ draft_year as u64, w.people[i].player.id as u64);
            (i, g)
        })
        .collect();
    pool.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut order = crate::career::draft_order(&w.nfl, &mut w.rng);
    let mut boards: Vec<std::collections::HashMap<usize, f64>> = vec![Default::default(); w.nfl.len()];
    let mut picked: Vec<(usize, u32, usize, String)> = Vec::new();
    for pick in 1..=slots as u32 {
        if pool.is_empty() {
            break;
        }
        // A team with a young franchise QB holding a top-5 pick in a class with a
        // first-round QB trades down to a team that needs one (the Rams for Goff
        // in 2016, the Panthers for Young in 2023).
        let mut traded_up = false;
        if pick <= 5 {
            let slot = pick as usize - 1;
            let holder = order[slot];
            let best_qb = pool.iter().take(60).find(|e| w.people[e.0].player.position == Position::QB).map(|e| e.1);
            if crate::career::need(&w.nfl[holder], Position::QB) <= -8.0 && best_qb.is_some_and(|g| g >= 80.0) {
                let end = (slot + 11).min(order.len());
                if let Some(k) = (slot + 1..end).find(|&k| crate::career::need(&w.nfl[order[k]], Position::QB) >= 0.0) {
                    order.swap(slot, k);
                    traded_up = true;
                }
            }
        }
        let t = order[(pick as usize - 1) % order.len()];
        let round = crate::career::draft_round(pick, slots as u32);
        let rf = match round {
            1 | 2 => 1.0,
            3 | 4 => 0.6,
            _ => 0.3,
        };
        let noise_sd = crate::career::draft_board_noise(w.nfl[t].draft_skill);
        let mut best: Option<(usize, f64, f64, f64)> = None;
        for (idx, &(i, g)) in pool.iter().enumerate().take(60) {
            let pos = w.people[i].player.position;
            let off = *boards[t].entry(i).or_insert_with(|| noise_sd * w.rng.normal(0.0, 1.0));
            let need = crate::career::need(&w.nfl[t], pos) * if round <= 2 { 0.5 } else { 1.0 };
            // A quarterback's premium is about need: a team happy with its starter
            // doesn't spend a first-round pick on another.
            // At the very top any team without a franchise QB takes the best one.
            let top_qb = clamp(1.0 - (pick as f64 - 1.0) / 8.0, 0.0, 1.0);
            let qb_gate = if pos == Position::QB {
                clamp((crate::career::need(&w.nfl[t], pos) + 2.0) / 4.0, 0.2, 1.0).max(top_qb)
            } else {
                1.0
            };
            let pv = draft_position_value(pos, g) * rf * qb_gate;
            // At the very top, a great player at a lesser position is worth less than
            // his grade says: a generational WR or safety rarely goes first.
            let lesser = !matches!(pos, Position::QB | Position::EDGE | Position::OT);
            let top = clamp(1.0 - (pick as f64 - 1.0) / 10.0, 0.0, 1.0);
            let damp = if lesser { 0.35 * (g + off - 85.0).max(0.0) * top } else { 0.0 };
            let v = g + off + pv + need - damp;
            if best.is_none_or(|b| v > b.1) {
                best = Some((idx, v, pv, need));
            }
        }
        let Some((idx, _, pv, need)) = best else { break };
        let (i, g) = pool.remove(idx);
        let pos = w.people[i].player.position;
        let team = &w.nfl[t];
        let why = if traded_up && pos == Position::QB {
            format!("they traded up to pick {pick} to take their quarterback")
        } else if need >= 1.0 {
            let weakest = team.weakest_starter(pos).map_or(0.0, |x| x.0);
            format!("they needed a {} (their weakest starter there was rated {weakest:.0})", pos.name())
        } else if pv >= 2.0 {
            format!("a {} is a premium position", pos.name())
        } else {
            "he was the best player left on their board".to_string()
        };
        let why = format!("{why}; board grade {g:.1}");
        let player = &w.people[i].player;
        w.nfl[t].depth[pos.index()].push(crate::league::Member::background(player.overall(), player.age));
        picked.push((i, pick, t, why));
    }
    let udfa = (slots as f64 * PIPELINE.udfa_ratio).round() as usize;
    // Each signee, with his pick, team and why (None for an undrafted signing).
    type Drafted = Option<(u32, usize, String)>;
    let mut signed: Vec<(usize, Drafted)> = picked.into_iter().map(|(i, pk, t, why)| (i, Some((pk, t, why)))).collect();
    for (k, &(i, _)) in pool.iter().enumerate() {
        if k < udfa {
            signed.push((i, None));
        } else {
            let p = &mut w.people[i];
            p.stage = Stage::Done;
            p.note("Went undrafted and unsigned, and his football career ended.");
        }
    }
    for (i, how) in signed {
        let c = w.people[i].college.unwrap();
        let school = w.program_name(c).to_string();
        let pick = how.as_ref().map(|h| h.0);
        let round = pick.map(|pk| crate::career::draft_round(pk, slots as u32));
        if let Some(r) = round {
            w.programs[c].draft_picks.push((draft_year, r));
        }
        let weight = match round {
            Some(1) => 1.0,
            Some(2 | 3) => 0.8,
            Some(_) => 0.5,
            None => 0.25,
        };
        credit_impact(w, i, c, weight, year);
        let team_name = how.as_ref().map(|h| w.nfl[h.1].name);
        let p = &mut w.people[i];
        p.stage = Stage::Pro;
        let age = p.player.age;
        match (how, round, team_name) {
            (Some((pk, _, why)), Some(r), Some(team)) => {
                p.note(format!("Drafted by the {team} in round {r} (pick {pk}) out of {school} at {age}: {why}."))
            }
            _ => p.note(format!("Signed as an undrafted free agent out of {school} at {age}.")),
        }
        if w.recording {
            let mut a = Alumnus::from_person(p, draft_year, pick, false);
            a.nfl_team = team_name;
            w.alumni.push(a);
        }
    }
    crate::league::advance_league(&mut w.nfl, &mut w.rng);
    // Exhausted players who didn't make a board leave football.
    for p in w.people.iter_mut() {
        if p.stage == Stage::College && (draft_year - p.class_of >= 5 || p.college_seasons >= 4) {
            p.stage = Stage::Done;
            p.note("His college eligibility ran out and his playing days ended.");
        }
    }
}

// --- Recruiting ---------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Recruit {
    person: usize,
    /// How good his film is, in rating points, aligned by position.
    film: f64,
    /// The services' shared guess at his upside.
    projection: f64,
    visibility: f64,
    gpa: f64,
    juco_mover: bool,
    viral: bool,
}

/// Out-of-state offers a junior college makes in a year (proposed, not confirmed).
const JUCO_OUT_OF_STATE_OFFERS: usize = 15;

/// NCAA initial eligibility (note 6). Division I: a 2.3 core GPA qualifies;
/// 2.0-2.299 is an academic redshirt (he can sign, get aid and practice, but
/// not play in year one); under 2.0 he can't enroll. Division II: 2.2 and a
/// 2.0 partial qualifier. Junior colleges take anyone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Eligibility {
    Qualifier,
    AcademicRedshirt,
    NonQualifier,
}

pub(crate) fn ncaa_tier(d: Division, gpa: f64) -> Eligibility {
    let full = match d {
        Division::JUCO => return Eligibility::Qualifier,
        Division::D2 => 2.2,
        _ => 2.3,
    };
    if gpa >= full {
        Eligibility::Qualifier
    } else if gpa >= 2.0 {
        Eligibility::AcademicRedshirt
    } else {
        Eligibility::NonQualifier
    }
}

/// A school's academic support, 0-1: tutors, summer school, learning
/// specialists. It follows the athletic department's resources.
fn academic_support(prog: &Program) -> f64 {
    clamp(0.3 + 0.7 * prog.facilities, 0.0, 1.0)
}

fn roster_cap(d: Division) -> usize {
    match d {
        // Roster limits since the 2025 House settlement: 105 in FBS.
        Division::FBS => 105,
        Division::FCS => 100,
        Division::D2 => 110,
        Division::JUCO => 80,
    }
}

/// What a starter at his position earns at a major program, middle of the
/// real range (CBS via SI, 2024; `data/real-world/rates.json`). Tight ends
/// and specialists aren't in the data (proposed, not confirmed).
fn position_nil(pos: Position) -> f64 {
    match pos {
        Position::QB => 650_000.0,
        Position::OT | Position::IOL => 425_000.0,
        Position::EDGE | Position::DT => 425_000.0,
        Position::RB => 250_000.0,
        Position::WR => 187_500.0,
        Position::TE => 150_000.0,
        Position::LB => 200_000.0,
        Position::CB | Position::S => 172_500.0,
        Position::K | Position::P => 25_000.0,
    }
}

/// A collective budget that pays major-program market rates.
fn nil_scale(d: Division) -> f64 {
    match d {
        Division::FBS => 10.0e6,
        Division::FCS => 0.4e6,
        _ => 0.1e6,
    }
}

fn level_value(d: Division) -> f64 {
    match d {
        Division::FBS => 1.0,
        Division::FCS => 0.55,
        Division::D2 => 0.3,
        Division::JUCO => 0.35,
    }
}

/// How much a school appeals to a recruit, from the wants of whoever decides (topic 5).
fn utility(p: &Person, wants: &super::background::Wants, prog: &Program, nil_offer: f64, eval_rank: f64) -> f64 {
    let g = |c: GradeCat| prog.grades[c as usize] / 100.0;
    let miles = geo::miles(p.home_state, prog.state);
    let close = clamp(1.0 - miles / 1500.0, 0.0, 1.0);
    let crisis = p.bg.story.iter().rev().take(1).any(|s| s.contains("death") || s.contains("lost a job"));
    let close_w = wants.close_to_home + if crisis { 0.15 } else { 0.0 };
    let prestige = (g(GradeCat::TitleChances)
        + g(GradeCat::StadiumAtmosphere)
        + g(GradeCat::Tradition)
        + g(GradeCat::BrandExposure))
        / 4.0;
    // Money on a real scale (note 13): $3.5M is clearly more than $2M. How
    // much it matters is his family's need, which ambition pulls against.
    let money = (nil_offer.max(0.0) / 1.0e6).powf(0.6);
    let need = clamp(0.25 + 0.75 * (1.0 - p.bg.money) + if crisis { 0.2 } else { 0.0 }, 0.1, 1.2);
    let drive = clamp((p.family.ambition - 10.0) / 10.0, -1.0, 1.0);
    let money_w = wants.money * need * (1.0 - 0.35 * drive);
    // What the ambitious kid weighs instead: the best path to the pros.
    let path_w = 0.35 * drive.max(0.0) * wants.money;
    let playing = g(GradeCat::PlayingTime) * (0.6 + 0.4 * eval_rank);
    let academics = g(GradeCat::Academics) * (0.5 + 0.5 * clamp((p.bg.gpa - 2.5) / 1.5, 0.0, 1.0));
    let development = (g(GradeCat::ProPipeline) + g(GradeCat::CoachPrestige)) / 2.0;
    // Every recruit wants the biggest name he can get at a level; how much
    // more depends on what he (or his decider) wants.
    let mut u = 1.6 * level_value(prog.division) * (0.5 + wants.prestige + wants.development)
        + (0.25 + wants.prestige) * prog.prestige_rel * (0.4 + 0.6 * level_value(prog.division))
        + 0.1 * g(GradeCat::Facilities)
        + wants.prestige * prestige
        + money_w * money * 1.5
        + close_w * close
        + wants.playing_time * playing
        + wants.academics * academics
        + (wants.development + path_w) * development
        + path_w * playing;
    // No athletic scholarships: the Pioneer League gives none (the family pays),
    // the Ivy League only need-based aid. A scholarship offer elsewhere usually wins.
    match prog.conference.as_str() {
        "PFL" => u -= 0.8 * (1.0 - 0.6 * p.bg.money),
        "Ivy" => u -= 0.4,
        _ => {}
    }
    // Teammates and older kids from his school already there: a pipeline school feels familiar.
    if let Some(&(v, _)) = prog.hs_pipeline.get(&p.hs) {
        u += 0.12 * v;
    }
    if p.family.older_brother_at == Some(prog.id) {
        u += if p.family.follows_brother { 0.15 } else { -0.05 };
    }
    u
}

/// Signing day: HS seniors, prep-school kids and JUCO players looking to move up.
pub(crate) fn signing_day(w: &mut World) {
    let year = w.year;
    let sign_year = year + 1;
    let seed = w.options.seed as u64 ^ (sign_year as u64) << 20;
    let n = w.people.len();
    let mut pool: Vec<Recruit> = Vec::new();
    // Film aligned by position: each position's top looks the same.
    let mut pos_stats: [(f64, f64, f64); POSITION_COUNT] = [(0.0, 0.0, 0.0); POSITION_COUNT];
    let mut candidates: Vec<usize> = Vec::new();
    for i in 0..n {
        let p = &w.people[i];
        let senior = p.stage == Stage::HighSchool && p.class_of <= sign_year;
        let prep = p.stage == Stage::PrepSchool;
        let juco = p.stage == Stage::College
            && p.college.is_some_and(|c| w.programs[c].division == Division::JUCO)
            && p.college_seasons >= 1;
        if senior || prep || juco {
            candidates.push(i);
            if senior {
                let f = p.last_performance.unwrap_or(p.player.overall());
                let s = &mut pos_stats[p.player.position.index()];
                s.0 += 1.0;
                s.1 += f;
                s.2 += f * f;
            }
        }
    }
    let pos_mean_sd: Vec<(f64, f64)> = pos_stats
        .iter()
        .map(|&(k, s, s2)| {
            if k < 2.0 {
                (45.0, 6.0)
            } else {
                let m = s / k;
                (m, ((s2 / k - m * m).max(1.0)).sqrt())
            }
        })
        .collect();
    let all_mean_sd = {
        let (k, s, s2) = pos_stats.iter().fold((0.0, 0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
        let m = s / k.max(1.0);
        (m, ((s2 / k.max(1.0) - m * m).max(1.0)).sqrt())
    };
    let culture: Vec<f64> = w.states.iter().map(|s| s.culture).collect();
    let schools = &w.high_schools;
    let coaches = &w.coaches;
    let programs = &w.programs;
    let recruits: Vec<Recruit> = candidates
        .par_iter()
        .map(|&i| {
            // (Visibility and film read only; the person's own stream isn't touched here.)
            let p = &w.people[i];
            let raw = p.last_performance.unwrap_or(p.player.overall());
            let (m, sd) = pos_mean_sd[p.player.position.index()];
            let film = all_mean_sd.0 + all_mean_sd.1 * (raw - m) / sd;
            let juco_mover = p.stage == Stage::College;
            let hs = &schools[p.hs];
            let school_exposure = if juco_mover {
                0.7
            } else if p.stage == Stage::PrepSchool {
                0.6
            } else {
                hs.prestige
            };
            let network = if juco_mover {
                p.college.and_then(|c| programs[c].head_coach).map_or(0.5, |c| coaches[c].ability().network / 20.0)
            } else {
                hs.coach.map_or(0.4, |c| coaches[c].ability().network / 20.0)
            };
            let camps = 0.3 + 0.7 * p.bg.money;
            let tape = clamp((film - all_mean_sd.0) / (3.0 * all_mean_sd.1) + 0.3, 0.0, 1.0);
            let traffic = 0.1 * (culture[p.home_state] + 0.6);
            let viral_odds =
                if p.family.parent_style == super::background::ParentStyle::SocialMedia { 0.06 } else { 0.02 };
            // A viral clip gets him a first look only: staffs still judge his film.
            let went_viral = !juco_mover
                && key_normal(seed ^ 11, p.player.id as u64) > 0.0
                && key_normal(seed ^ 13, p.player.id as u64).abs() < viral_odds * 2.5;
            let viral = if went_viral { 0.25 } else { 0.0 };
            let bloodline = match p.family.bloodline {
                super::background::Bloodline::NflPlayer => 0.15,
                super::background::Bloodline::CollegePlayer => 0.05,
                super::background::Bloodline::None => 0.0,
            };
            let visibility = clamp(
                0.05 + 0.3 * school_exposure + 0.2 * network + 0.15 * camps + 0.25 * tape + traffic + viral + bloodline,
                0.0,
                1.0,
            );
            Recruit { person: i, film, projection: 0.0, visibility, gpa: p.bg.gpa, juco_mover, viral: went_viral }
        })
        .collect();
    pool.extend(recruits);
    // Older brothers already playing college football, from the same high school (topic 5).
    let mut alumni_of: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for p in w.people.iter() {
        if p.stage == Stage::College
            && let Some(c) = p.college
        {
            alumni_of.entry(p.hs).or_default().push(c);
        }
    }
    for r in pool.iter() {
        if r.juco_mover {
            continue;
        }
        let p = &mut w.people[r.person];
        if let Some(list) = alumni_of.get(&p.hs)
            && p.family.older_brother_at.is_none()
            && p.rng.chance(0.04)
        {
            let c = list[p.rng.int(0, list.len() as i32 - 1) as usize];
            p.family.older_brother_at = Some(c);
        }
    }
    let dbg_t = std::time::Instant::now();
    // Services' upside guess: one shared projection.
    for r in pool.iter_mut() {
        let p = &mut w.people[r.person];
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 12.0, level: Level::HS }, &mut p.rng);
        r.projection = rep.potential;
        p.visibility = r.visibility;
        p.bg.exposure_network = r.visibility;
        if r.viral {
            let age = p.player.age;
            p.note(format!("At {age} a highlight clip went viral and got him a first look from recruiters."));
        }
    }

    // --- Star ratings: the services only rank kids they have seen -------------------------
    let seniors: Vec<usize> = (0..pool.len()).filter(|&k| !pool[k].juco_mover).collect();
    let mut by_star: Vec<usize> = seniors.clone();
    let star_score = |r: &Recruit| 0.25 * r.film + 0.75 * r.projection - 12.0 * (1.0 - r.visibility).powi(2);
    by_star.sort_by(|&a, &b| star_score(&pool[b]).total_cmp(&star_score(&pool[a])));
    let class = 250_000.0 * w.options.scale;
    let mut cursor = 0;
    for (stars, share) in PIPELINE.star_shares {
        let count = (share * class).round() as usize;
        for &k in by_star.iter().skip(cursor).take(count) {
            let i = pool[k].person;
            if w.people[i].player.position.is_specialist() && stars >= 4 {
                continue;
            }
            w.people[i].stars = stars;
        }
        cursor += count;
    }

    // Senior year: borderline students with offers coming get pushed over an
    // NCAA line by summer school and retaken courses (note 6). Big programs
    // chase this hardest for blue-chips.
    for &k in &seniors {
        let i = pool[k].person;
        let p = &mut w.people[i];
        if p.stars < 3 {
            continue;
        }
        let gpa = p.bg.gpa;
        let line = if (1.85..2.0).contains(&gpa) {
            2.0
        } else if (2.15..2.3).contains(&gpa) {
            2.3
        } else {
            continue;
        };
        let chance = 0.3
            + 0.03 * (p.player.personality.work_ethic - 10.0)
            + if p.stars >= 4 { 0.15 } else { 0.0 }
            + 0.15 * p.bg.school_resources;
        if p.rng.chance(clamp(chance, 0.05, 0.85)) {
            p.bg.gpa = line + p.rng.range(0.0, 0.05);
            pool[k].gpa = p.bg.gpa;
            let age = p.player.age;
            p.note(format!(
                "At {age}, summer school and retaken courses lifted his core GPA from {gpa:.2} to {:.2}.",
                p.bg.gpa
            ));
        }
    }

    if w.recording {
        let mut counts = [0u32; 6];
        for &k in &seniors {
            counts[w.people[pool[k].person].stars as usize] += 1;
        }
        w.signing_classes.push((sign_year, counts));
    }

    let dbg_lap = |what: &str| {
        if std::env::var("AFM_DEBUG").is_ok() {
            eprintln!("    {what}: {:.2}s", dbg_t.elapsed().as_secs_f64());
        }
    };
    dbg_lap("pool and stars");
    // --- Who each program sees and wants -----------------------------------------------
    // Only kids with real college film or upside get looked at.
    let mut interest: Vec<usize> = (0..pool.len()).collect();
    interest.sort_by(|&a, &b| {
        let ka = pool[a].film.max(pool[a].projection - 6.0);
        let kb = pool[b].film.max(pool[b].projection - 6.0);
        kb.total_cmp(&ka)
    });
    interest.truncate(((class * 0.25) as usize).max(50).min(pool.len()));
    // A compact copy of what staffs look at, so the scan stays in cache.
    struct Cand {
        k: usize,
        state: usize,
        hs: usize,
        area: usize,
        stars: u8,
        vis: f64,
        gpa: f64,
        /// A strict school's special admit: grades trending up, good character
        /// reads, a stable home or a mentor, no off-field trouble (note 6).
        grows_out: bool,
        film: f64,
        proj: f64,
        id: u64,
        specialist: bool,
        juco_mover: bool,
    }
    let cands: Vec<Cand> = interest
        .iter()
        .map(|&k| {
            let r = &pool[k];
            let p = &w.people[r.person];
            Cand {
                k,
                state: p.home_state,
                hs: p.hs,
                area: w.high_schools[p.hs].area,
                stars: p.stars,
                vis: r.visibility,
                gpa: r.gpa,
                grows_out: {
                    let pers = &p.player.personality;
                    let read = |k: u64| 1.5 * key_normal(seed ^ k, p.player.id as u64);
                    let trouble = p.player.history.iter().any(|h| {
                        h.events.has(crate::player::DevelopmentEvent::Suspended)
                            || h.events.has(crate::player::DevelopmentEvent::Dismissed)
                    });
                    p.gpa_trend > 0.02
                        && pers.work_ethic + read(31) >= 11.0
                        && pers.coachability + read(37) >= 10.0
                        && (p.bg.home_stability >= 0.5 || p.bg.mentor)
                        && !trouble
                },
                film: r.film,
                proj: r.projection,
                id: p.player.id as u64,
                specialist: p.player.position.is_specialist(),
                juco_mover: r.juco_mover,
            }
        })
        .collect();
    let programs = &w.programs;
    let coaches = &w.coaches;
    let lists: Vec<Vec<(usize, f64, bool)>> = programs
        .par_iter()
        .map(|prog| {
            let rec_skill = prog.head_coach.map_or(10.0, |c| coaches[c].ability().recruiting);
            // A head coach on the hot seat: recent results under expectation.
            let pressure = prog
                .head_coach
                .map_or(0.0, |c| clamp(-3.0 * coaches[c].recent.last().copied().unwrap_or(0.0), 0.0, 1.0));
            let noise_sd = clamp(3.5 - 0.15 * (rec_skill - 10.0), 1.0, 6.0);
            let pseed = seed ^ (prog.id as u64).wrapping_mul(0x1000_0001);
            // The staff's own ties to high schools, carried from their old jobs (note 4).
            let mut ties: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
            for c in [prog.head_coach, prog.oc, prog.dc].into_iter().chain(prog.position_coaches).flatten() {
                for (&h, &t) in &coaches[c].ties {
                    let e = ties.entry(h).or_insert(0.0);
                    *e = e.max(t);
                }
            }
            let mut seen: Vec<(usize, f64, bool)> = Vec::new();
            for c in &cands {
                // A JUCO player moves up to a four-year school or nowhere.
                if c.juco_mover && prog.division == Division::JUCO {
                    continue;
                }
                // Staffs don't waste offers on blue-chips far above their level.
                if (prog.division == Division::D2 && c.stars >= 4) || (prog.division == Division::FCS && c.stars >= 5) {
                    continue;
                }
                // A blue-chip is seen by everyone; others only inside a footprint.
                let bonus = if c.stars >= 4 { 1.0 } else { 0.15 * c.stars as f64 };
                // Pipelines to his school and his area, and the staff's ties, open doors.
                let hs_pipe = prog.hs_pipeline.get(&c.hs).map_or(0.0, |v| v.0);
                let area_pipe = prog.area_pipeline.get(&c.area).map_or(0.0, |v| v.0);
                let tie = ties.get(&c.hs).copied().unwrap_or(0.0);
                let local = hs_pipe.max(0.6 * area_pipe).max(0.7 * tie);
                let see = (prog.footprint[c.state] + 1.2 * local) * c.vis * 1.6 + bonus;
                if see < 1.0 && key_uniform(pseed, c.id) > see {
                    continue;
                }
                // The NCAA floor can't be bent; a school's own bar can, a few times a class.
                let tier = ncaa_tier(prog.division, c.gpa);
                if tier == Eligibility::NonQualifier {
                    continue;
                }
                let special = c.gpa < prog.admissions_bar;
                if special && (tier != Eligibility::Qualifier || !c.grows_out || prog.special_admits == 0) {
                    continue;
                }
                let read = |k: u64| noise_sd * key_normal(pseed.wrapping_add(k), c.id);
                // Staffs offer on film more than on a service's projection.
                let mut eval =
                    0.6 * (c.film + read(1)) + 0.4 * (c.proj + read(2)) - if c.specialist { 8.0 } else { 0.0 };
                // A borderline student can lose his eligibility in college. Strong
                // academic support takes that risk more easily, and so does a
                // coach who has to win now.
                let risk = clamp((2.6 - c.gpa) / 0.6, 0.0, 1.0);
                eval -= 4.0 * risk * (1.0 - academic_support(prog)) * (1.0 - 0.6 * pressure);
                seen.push((c.k, eval, special));
            }
            seen.sort_by(|a, b| b.1.total_cmp(&a.1));
            // Junior college leagues cap out-of-state players (Mississippi and
            // Kansas both do), so one JUCO can't sign every national blue-chip.
            if prog.division == Division::JUCO {
                let mut out_of_state = 0;
                seen.retain(|&(k, _, _)| {
                    if w.people[pool[k].person].home_state == prog.state {
                        return true;
                    }
                    out_of_state += 1;
                    out_of_state <= JUCO_OUT_OF_STATE_OFFERS
                });
            }
            seen.truncate(prog.class_size * 40);
            seen
        })
        .collect();

    dbg_lap("lists");
    // Slots: a class, bent by roster room.
    let slots: Vec<usize> = programs
        .iter()
        .map(|prog| {
            let room = roster_cap(prog.division).saturating_sub(prog.roster.len());
            (room as f64).min(prog.class_size as f64 * 1.4) as usize
        })
        .collect();

    // --- Deferred acceptance: programs offer down their boards; each recruit holds the best offer.
    let pn = programs.len();
    let mut next = vec![0usize; pn];
    let mut held: Vec<Option<(usize, f64, bool)>> = vec![None; pool.len()];
    let mut taken = vec![0usize; pn];
    let mut special_taken = vec![0u32; pn];
    let mut offers: Vec<Vec<usize>> = vec![Vec::new(); pool.len()];
    // NIL offers per player (note 13): his position's market value and how
    // good he is, scaled by what the collective can pay. A program on the rise
    // with money to spend reaches this year (the new-era reach).
    let reach: Vec<bool> = programs
        .iter()
        .map(|prog| {
            let rising = prog.head_coach.is_some_and(|c| w.coaches[c].recent.last().is_some_and(|&r| r > 0.25));
            let rich = prog.collective.budget / nil_scale(prog.division) > 1.15 * prog.prestige_rel;
            rising && rich && key_uniform(seed ^ 404, prog.id as u64) < 0.6
        })
        .collect();
    let offer_for = |j: usize, k: usize, rank: f64| -> f64 {
        let prog = &programs[j];
        let p = &w.people[pool[k].person];
        let talent = match p.stars {
            5 => 2.5,
            4 => 1.0,
            3 => 0.3,
            2 => 0.08,
            _ => 0.03,
        } * (1.3 - 0.6 * rank);
        let pay = (prog.collective.budget / 10.0e6).powf(0.8);
        let cap = prog.collective.budget * 0.6 * if reach[j] { 0.35 } else { 0.25 };
        (position_nil(p.player.position) * talent * pay * if reach[j] { 1.6 } else { 1.0 }).min(cap)
    };
    let mut held_offer = vec![0.0f64; pool.len()];
    let mut active = true;
    while active {
        active = false;
        for j in 0..pn {
            while taken[j] < slots[j] && next[j] < lists[j].len() {
                let (k, _, special) = lists[j][next[j]];
                let rank = next[j] as f64 / lists[j].len().max(1) as f64;
                next[j] += 1;
                active = true;
                if special && special_taken[j] >= programs[j].special_admits {
                    continue;
                }
                let p = &w.people[pool[k].person];
                let wants = p.family.decider_wants;
                // The staff wins him over in person (note 15C): the head coach's
                // recruiting and relationships, and his position coach's recruiting.
                let prog = &programs[j];
                let group = super::coach::Group::of(p.player.position);
                let hc = prog.head_coach.map(|c| *w.coaches[c].ability());
                let pc = prog.position_coaches[group as usize].map(|c| w.coaches[c].ability().recruiting);
                let skill = 0.5 * hc.map_or(9.0, |a| a.recruiting)
                    + 0.25 * hc.map_or(9.0, |a| a.relationships)
                    + 0.25 * pc.unwrap_or(9.0);
                let bond = 0.015 * (skill - 10.0) + 0.03 * key_normal(seed ^ 91 ^ j as u64, p.player.id as u64);
                let mut offer = offer_for(j, k, rank);
                let base_u = bond + 0.04 * key_normal(seed ^ 77 ^ j as u64, p.player.id as u64);
                let mut u = utility(p, &wants, prog, offer, 1.0 - rank) + base_u;
                // Bidding (note 13): a school that wants him near the top of its
                // board raises its offer to beat the one he holds, within its money.
                if let Some((_, cu, _)) = held[k]
                    && cu >= u
                    && rank < 0.3
                {
                    let cap = prog.collective.budget * 0.6 * if reach[j] { 0.35 } else { 0.25 };
                    let raised = (offer * 1.5).min(cap);
                    let ru = utility(p, &wants, prog, raised, 1.0 - rank) + base_u;
                    if ru > cu {
                        offer = raised;
                        u = ru;
                    }
                }
                // Staffs don't waste an offer on a kid already holding a far better one.
                if held[k].is_some_and(|(_, cu, _)| cu > u + 0.15) {
                    continue;
                }
                offers[k].push(j);
                match held[k] {
                    Some((_, cu, _)) if cu >= u => {}
                    Some((cur, _, cur_special)) => {
                        taken[cur] -= 1;
                        if cur_special {
                            special_taken[cur] -= 1;
                        }
                        held[k] = Some((j, u, special));
                        held_offer[k] = offer;
                        taken[j] += 1;
                        special_taken[j] += special as u32;
                    }
                    None => {
                        held[k] = Some((j, u, special));
                        held_offer[k] = offer;
                        taken[j] += 1;
                        special_taken[j] += special as u32;
                    }
                }
            }
        }
    }

    dbg_lap("deferred acceptance");
    if std::env::var("AFM_DEBUG").is_ok() {
        let mut by_div = [0usize; 4];
        for h in held.iter().flatten() {
            by_div[programs[h.0].division as usize] += 1;
        }
        let want: Vec<usize> = (0..4)
            .map(|d| programs.iter().enumerate().filter(|(_, p)| p.division as usize == d).map(|(j, _)| slots[j]).sum())
            .collect();
        let listed: usize = lists.iter().map(|l| l.len()).sum();
        eprintln!(
            "signing {sign_year}: pool {} interest {} listed {listed} signed {:?} slots {:?}",
            pool.len(),
            interest.len(),
            by_div,
            want
        );
    }
    // --- Outcomes ---------------------------------------------------------------------
    for (k, r) in pool.iter().enumerate() {
        let i = r.person;
        let from_hs = w.school_name(w.people[i].hs).to_string();
        match held[k] {
            Some((j, _, special)) => {
                let prog_name = w.program_name(j).to_string();
                let division = w.programs[j].division;
                // The offers he had, biggest names first.
                let mut offer_list: Vec<usize> = offers[k].clone();
                offer_list.sort_by(|&a, &b| {
                    (w.programs[b].prestige + 0.3 * level_value(w.programs[b].division))
                        .total_cmp(&(w.programs[a].prestige + 0.3 * level_value(w.programs[a].division)))
                });
                offer_list.dedup();
                let offer_names: Vec<String> = offer_list.iter().map(|&o| w.program_name(o).to_string()).collect();
                // Was he steered? Compare with where he himself would have gone among his offers.
                let p = &w.people[i];
                let steered = p.family.decider != Decider::Himself && {
                    let own = p.family.own_wants;
                    let mine = utility(p, &own, &w.programs[j], 1e5, 0.5);
                    // His own favorite among the programs that wanted him.
                    offers[k].iter().any(|&jj| jj != j && utility(p, &own, &w.programs[jj], 1e5, 0.5) > mine + 0.06)
                };
                if w.recording && !r.juco_mover && division == Division::FBS {
                    w.fbs_signees_by_state[w.people[i].home_state] += 1;
                }
                let p = &mut w.people[i];
                let step = match (r.juco_mover, division) {
                    (true, Division::JUCO) => Step::Sideways,
                    (true, _) => Step::Up,
                    (false, Division::FBS) => Step::HsToFbs,
                    (false, Division::FCS) => Step::HsToFcs,
                    (false, _) => Step::HsToSmallCollege,
                };
                p.pending_dip = Some(World::adjustment(p, step));
                let age = p.player.age;
                let stars = if p.stars > 0 { format!("a {}-star", p.stars) } else { "an unranked recruit".to_string() };
                if r.juco_mover {
                    p.note(format!("At {age} his JUCO film earned a move up to {prog_name} ({}).", division.name()));
                    p.college_seasons = p.college_seasons.min(2);
                } else {
                    let gpa = p.bg.gpa;
                    p.gpa_at_signing = Some(gpa);
                    p.note(format!(
                        "Signed with {prog_name} ({}) at {age} as {stars} out of {from_hs} (core GPA {gpa:.2}).",
                        division.name()
                    ));
                    let nil = held_offer[k];
                    if nil >= 10_000.0 {
                        let amount = if nil >= 1.0e6 {
                            format!("${:.1}M", nil / 1.0e6)
                        } else {
                            format!("${:.0}K", nil / 1.0e3)
                        };
                        let why = if reach[j] { " in a big reach by a rising program" } else { "" };
                        p.note(format!("His NIL deal there: about {amount} a year{why}."));
                    }
                    if offer_names.len() > 1 {
                        let n = offer_names.len();
                        let top: Vec<&str> = offer_names.iter().take(4).map(String::as_str).collect();
                        p.note(format!("He had {n} offers, the biggest from {}.", top.join(", ")));
                    }
                    p.college_seasons = 0;
                    p.redshirt_used = false;
                    match ncaa_tier(division, gpa) {
                        Eligibility::AcademicRedshirt if division != Division::JUCO => {
                            p.academic_redshirt = true;
                            p.note("He signed as an academic redshirt: on scholarship and practicing, but he can't play in his first year.");
                        }
                        _ => {}
                    }
                    if division == Division::JUCO && gpa < 2.0 {
                        p.note(format!(
                            "His core GPA ({gpa:.2}) was under the NCAA's 2.0 floor, so four-year schools couldn't take him."
                        ));
                    }
                    if special {
                        p.note(format!(
                            "{prog_name} used one of its few special admissions on him: his grades were below the school's bar, but trending up."
                        ));
                    }
                }
                if division == Division::JUCO {
                    p.juco = true;
                }
                if steered {
                    p.steered = true;
                    let who = match p.family.decider {
                        Decider::Parent => "a parent",
                        Decider::Grandparent => "his grandparents",
                        Decider::Handler => "a handler close to the family",
                        Decider::Himself => "himself",
                    };
                    p.note(format!("The choice was {who}'s more than his own."));
                }
                p.stage = Stage::College;
                p.college = Some(j);
                p.path.push(j);
                p.on_notice = false;
                // His high school coach gets the credit for sending him up.
                let (hs, group) = (p.hs, super::coach::Group::of(p.player.position));
                let up = match w.programs[j].division {
                    Division::FBS => 1.0,
                    Division::FCS => 0.5,
                    _ => 0.25,
                };
                if w.recording && let Some(c) = w.high_schools[hs].coach {
                    w.coaches[c].units[group as usize].moved_up += up;
                }
            }
            None => {
                let p = &mut w.people[i];
                if r.juco_mover {
                    if sign_year - p.class_of >= 2 {
                        p.stage = Stage::Done;
                        p.note("Never earned a move up from JUCO and left football.");
                    }
                    continue;
                }
                if p.stage == Stage::PrepSchool {
                    p.stage = Stage::Done;
                    p.note("A prep school year didn't bring an offer; he left football.");
                    continue;
                }
                // A kid with college film but bad grades can spend a year at prep school.
                let good = r.film >= all_mean_sd.0 + 2.2 * all_mean_sd.1;
                if good && p.bg.gpa < 2.3 && p.rng.chance(0.25 + 0.4 * p.bg.money) {
                    p.stage = Stage::PrepSchool;
                    p.bg.gpa = clamp(p.bg.gpa + p.rng.range(0.1, 0.6), 0.0, 4.0);
                    let age = p.player.age;
                    p.note(format!("At {age} he went to prep school for a year to fix his grades."));
                } else {
                    p.stage = Stage::Done;
                    // A ranked recruit who never signs gets a reason (trial 1 flag 5).
                    if p.stars >= 3 {
                        let gpa = p.bg.gpa;
                        let why = if gpa < 2.0 {
                            format!(
                                "With a core GPA of {gpa:.2}, under the NCAA's 2.0 floor, no four-year school could take him, and no junior college place came through; he stopped playing."
                            )
                        } else if offers[k].is_empty() {
                            "No program offered him, and he stopped playing after high school.".to_string()
                        } else {
                            "None of his offers worked out, and he stopped playing after high school.".to_string()
                        };
                        p.note(why);
                    }
                }
            }
        }
    }
    // Seniors not in the pool's interest list simply finish high school.
    for p in w.people.iter_mut() {
        if p.stage == Stage::HighSchool && p.class_of <= sign_year {
            p.stage = Stage::Done;
        }
    }
    let _ = n;
}

// --- The transfer portal --------------------------------------------------------------

/// Players short of snaps, after a coaching change, steered kids now free to
/// choose, standouts moving up (often after quiet tampering), and dismissed
/// players starting over. NIL money is then shared out.
pub(crate) fn portal(w: &mut World, _r: &SeasonResults) {
    let year = w.year;
    let seed = w.options.seed as u64 ^ ((year as u64) << 24) ^ 0xA5;
    let n = w.people.len();
    // Position medians of starters' play by division, the bar to move up.
    let mut starters: [[Vec<f64>; POSITION_COUNT]; 4] = Default::default();
    for p in w.people.iter() {
        if p.stage == Stage::College
            && p.last_role == Some(PlayingTime::Starter)
            && let (Some(c), Some(perf)) = (p.college, p.last_performance)
        {
            starters[w.programs[c].division as usize][p.player.position.index()].push(perf);
        }
    }
    for d in starters.iter_mut() {
        for v in d.iter_mut() {
            v.sort_by(|a, b| a.total_cmp(b));
        }
    }
    let median = |d: Division, pos: Position| -> f64 {
        let v = &starters[d as usize][pos.index()];
        if v.is_empty() { 99.0 } else { v[v.len() / 2] }
    };
    let new_staff: Vec<bool> = w
        .programs
        .iter()
        .map(|p| p.head_coach.is_some_and(|c| w.coaches[c].job.is_some_and(|j| j.since == year)))
        .collect();

    #[derive(Clone, Copy, PartialEq)]
    enum Why {
        PlayingTime,
        CoachingChange,
        Steered,
        MoveUp,
        Dismissed,
        Money,
    }
    let mut entrants: Vec<(usize, Why)> = Vec::new();
    let mut steered_stays: Vec<usize> = Vec::new();
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::College {
            continue;
        }
        let Some(c) = p.college else { continue };
        let div = w.programs[c].division;
        if p.player.history.last().is_some_and(|h| h.events.has(crate::player::DevelopmentEvent::Dismissed)) {
            entrants.push((i, Why::Dismissed));
            continue;
        }
        if p.college_seasons >= 4 || div == Division::JUCO {
            continue;
        }
        let pos = p.player.position;
        let perf = p.last_performance.unwrap_or(0.0);
        let up = match div {
            Division::FCS => Some(Division::FBS),
            Division::D2 => Some(Division::FCS),
            _ => None,
        };
        let u = 0.5 + 0.5 * (key_normal(seed, p.player.id as u64) / 3.0).clamp(-1.0, 1.0);
        // Standouts below FBS get noticed; richer programs quietly reach out first (never punished).
        if let Some(up) = up
            && perf >= median(up, pos) + 1.0
        {
            let tampered = w.programs[c].collective.budget < 2.0e6;
            let chance = PIPELINE.transfer_chance * if tampered { 1.6 } else { 1.0 };
            if u < chance {
                entrants.push((i, Why::MoveUp));
                continue;
            }
        }
        // A kid steered to the wrong school decides once, after his first year, whether to leave.
        if p.steered && p.player.age >= 18 {
            if u < 0.45 {
                entrants.push((i, Why::Steered));
                continue;
            }
            steered_stays.push(i);
        }
        let base = match p.last_role {
            Some(PlayingTime::Backup) => PIPELINE.portal_entry.backup,
            Some(PlayingTime::Rotation) => PIPELINE.portal_entry.rotation,
            _ => 0.0,
        };
        let changed = new_staff[c];
        let chance = base
            * if changed { PIPELINE.portal_entry.coaching_change } else { 1.0 }
            * (1.0 + (p.player.personality.competitiveness - 10.0) * 0.04);
        if u < chance {
            entrants.push((i, if changed { Why::CoachingChange } else { Why::PlayingTime }));
            continue;
        }
        // A greedy starter at a poor program can be bought away.
        if p.last_role == Some(PlayingTime::Starter)
            && w.programs[c].collective.budget < 1.0e6
            && div == Division::FBS
            && u < 0.06 * p.family.own_wants.money * 4.0
        {
            entrants.push((i, Why::Money));
        }
    }

    for i in steered_stays {
        w.people[i].steered = false;
    }
    // Each entrant shops among programs that would take him.
    w.rebuild_rosters();
    let mut adds = vec![0usize; w.programs.len()];
    let pn = w.programs.len();
    for (i, why) in entrants {
        let c = w.people[i].college.unwrap();
        let from = w.programs[c].division;
        let pos = w.people[i].player.position;
        let perf = w.people[i].last_performance.unwrap_or(w.people[i].player.overall());
        let serious = why == Why::Dismissed && w.people[i].rng.chance(PIPELINE.college_trouble.serious_share);
        if serious && w.people[i].rng.chance(PIPELINE.college_trouble.serious_quit_share) {
            let p = &mut w.people[i];
            p.stage = Stage::Done;
            p.note("After a serious incident no program would take him, and he never played again.");
            continue;
        }
        let mut best: Option<(usize, f64)> = None;
        for _ in 0..40 {
            let j = w.rng.int(0, pn as i32 - 1) as usize;
            if j == c || adds[j] >= 10 || w.programs[j].roster.len() + adds[j] >= roster_cap(w.programs[j].division) {
                continue;
            }
            let d = w.programs[j].division;
            // A dismissed player starts over a level down or at JUCO.
            let allowed = match why {
                Why::Dismissed => d as usize > from as usize || (d == from && !serious && perf >= median(d, pos)),
                Why::MoveUp => (d as usize) < from as usize,
                _ => d as usize >= from as usize || d == Division::FBS,
            };
            if !allowed || perf + 2.0 < median(d, pos) - if d as usize > from as usize { 3.0 } else { 0.0 } {
                continue;
            }
            let p = &w.people[i];
            let u = utility(p, &p.family.own_wants, &w.programs[j], w.programs[j].collective.budget / 40.0, 0.6);
            if best.is_none_or(|(_, bu)| u > bu) {
                best = Some((j, u));
            }
        }
        let age = w.people[i].player.age - 1;
        let from_name = w.program_name(c).to_string();
        let what = match why {
            Why::PlayingTime => "short of snaps",
            Why::CoachingChange => "after a coaching change",
            Why::Steered => "free at last to choose for himself",
            Why::MoveUp => "after standing out a level below",
            Why::Dismissed => "after his dismissal",
            Why::Money => "for a bigger NIL deal",
        };
        let Some((j, _)) = best else {
            let p = &mut w.people[i];
            p.stage = Stage::Done;
            p.note(format!("Entered the portal at {age} {what}, found no school and left football."));
            continue;
        };
        adds[j] += 1;
        let to = w.programs[j].division;
        let to_name = w.program_name(j).to_string();
        let step = if (to as usize) < from as usize {
            Step::Up
        } else if to == from {
            Step::Sideways
        } else {
            Step::Down
        };
        let p = &mut w.people[i];
        p.pending_dip = Some(World::adjustment(p, step));
        p.college = Some(j);
        p.path.push(j);
        p.transfers += 1;
        p.steered = false;
        p.on_notice = false;
        if to == Division::JUCO {
            p.juco = true;
        }
        p.note(format!("Entered the portal at {age} {what} and moved from {from_name} to {to_name} ({}).", to.name()));
    }

    // --- NIL: each collective pays its roster; big money can backfire ----------------------
    w.rebuild_rosters();
    for j in 0..pn {
        let budget = w.programs[j].collective.budget * 0.6;
        let roster = w.programs[j].roster.clone();
        if roster.is_empty() {
            continue;
        }
        let value = |p: &Person| {
            let role = match p.last_role {
                Some(PlayingTime::Starter) => 3.0,
                Some(PlayingTime::Rotation) => 1.2,
                _ => 0.3,
            };
            let pos = 1.0 + 0.25 * position_value(p.player.position).max(-2.0);
            role * pos * (1.0 + 0.1 * p.stars as f64)
        };
        let total: f64 = roster.iter().map(|&i| value(&w.people[i])).sum();
        let mut paid: Vec<(f64, usize)> =
            roster.iter().map(|&i| (budget * value(&w.people[i]) / total.max(1e-9), i)).collect();
        paid.sort_by(|a, b| b.0.total_cmp(&a.0));
        let team_median = {
            let mut v: Vec<f64> = roster.iter().filter_map(|&i| w.people[i].last_performance).collect();
            v.sort_by(|a, b| a.total_cmp(b));
            if v.is_empty() { 50.0 } else { v[v.len() / 2] }
        };
        let mut jealousy = 0.0;
        for (k, &(money, i)) in paid.iter().enumerate() {
            let p = &mut w.people[i];
            p.nil_income = money;
            // The top earners on a team: payouts can bring jealousy, complacency or pressure.
            if k < 3 && money > 250_000.0 {
                let perf = p.last_performance.unwrap_or(team_median);
                if perf < team_median {
                    p.bg.stability = clamp(p.bg.stability - 1.0, 1.0, 20.0);
                    jealousy += 0.2;
                    let age = p.player.age;
                    p.note(format!(
                        "At {age}, paid among the most on his team, he struggled and took heavy criticism."
                    ));
                } else if p.rng.chance(0.08) {
                    p.player.personality.work_ethic = clamp(p.player.personality.work_ethic - 1.0, 1.0, 20.0);
                    let age = p.player.age;
                    p.note(format!("At {age} a big NIL payday made him a little complacent."));
                }
            }
        }
        if jealousy > 0.0
            && let Some(hc) = w.programs[j].head_coach
        {
            w.coaches[hc].reputation = clamp(w.coaches[hc].reputation - 0.3 * jealousy, 1.0, 20.0);
        }
    }
    let _ = Level::HS;
}
