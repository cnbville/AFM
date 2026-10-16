//! Moving players through the world: high school transfers and powerhouse
//! pulls (topic 2), exposure and recruiting (topics 4 and 5), JUCO and prep
//! school (topic 6, 1.6), the draft, the transfer portal, tampering and NIL
//! (topic 8).

use rayon::prelude::*;

use super::background::Decider;
use super::data::{CollegeRow, Division};
use super::program::{GradeCat, Program, record_impact};
use super::season::{key_normal, key_uniform};
use super::{Alumnus, CommitTiming, Person, Stage, World, division_level, geo};
use crate::calendar::{Date, Event, Kind, Level as CalLevel, Loudness, Moment};
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
    let mut moves: Vec<(usize, usize, bool, bool)> = Vec::new();
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
            // A boarding academy (IMG, topic 4): about $90k a year, so family
            // money matters (the very best get aid), and a kid who wants to
            // stay near home rarely goes. Proposed, not confirmed.
            if boarding(dest) {
                let aid = p.player.overall() >= ovr[0] - 1.0;
                pull *= if aid { 0.5 + 0.5 * p.bg.money } else { 0.15 + 0.85 * p.bg.money };
                pull *= clamp(1.0 - 2.5 * (p.family.own_wants.close_to_home - 0.15), 0.3, 1.0);
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
                // A boarder's family stays home; a national move otherwise moves the family.
                let family_moves = (national && !boarding(dest)) || escape && w.rng.chance(0.5);
                let free = national;
                moves.push((i, t, family_moves, free));
            }
        }
    }
    for (i, t, family_moved, national) in moves {
        let from = w.people[i].hs;
        let from_name = w.school_name(from).to_string();
        let to_name = w.school_name(t).to_string();
        let st = w.high_schools[t].state;
        let rule = w.states[st].transfer;
        let boards = boarding(&w.high_schools[t]);
        let p = &mut w.people[i];
        let first = p.transfers == 0;
        // An out-of-state boarder is a new enrollee, not a transfer inside the state.
        p.sit_out = if family_moved || (national && boards) || (rule.free_once && first) { 0.0 } else { rule.sit_out };
        if boards {
            let age = p.player.age;
            p.note(format!("At {age} he left home to board at {to_name}."));
        }
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
    // Boarding away from home costs homesick kids (topic 4): one who wants
    // to be near home may go back after a year (proposed, not confirmed: up
    // to 30% a year for the most homesick).
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool || p.player.age > 17 || !boarding(&w.high_schools[p.hs]) {
            continue;
        }
        let homesick = clamp((p.family.own_wants.close_to_home - 0.12) * 2.5, 0.0, 0.3);
        if p.home_state == w.high_schools[p.hs].state || !w.rng.chance(homesick) {
            continue;
        }
        let home = p.home_state;
        let options: Vec<usize> = w.states[home].schools.clone();
        if options.is_empty() {
            continue;
        }
        let t = options[w.rng.int(0, options.len() as i32 - 1) as usize];
        let from_name = w.school_name(p.hs).to_string();
        let to_name = w.school_name(t).to_string();
        let p = &mut w.people[i];
        p.hs = t;
        p.sit_out = 0.0;
        let age = p.player.age;
        p.note(format!("Homesick at {age}, he left {from_name} and came home to play for {to_name}."));
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

/// A boarding academy (IMG Academy): students come from anywhere and live
/// on campus; tuition is about $90k a year.
pub(crate) fn boarding(h: &super::high_school::HighSchool) -> bool {
    h.national && h.name.starts_with("IMG Academy")
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
        Position::K | Position::P | Position::LS => (-25.0, 0.0),
    };
    base + premium * top
}

/// A flat discount on kickers, punters and long snappers on draft day, on
/// top of the position value (which shrinks in later rounds). Sized so
/// about two kickers and two punters a year are drafted, and a long snapper
/// every two or three years (real 2000-2025, nflverse: 2.15, 1.96 and 0.35
/// a year; proposed, not confirmed).
fn specialist_draft_discount(pos: Position) -> f64 {
    match pos {
        Position::K | Position::P => 2.0,
        Position::LS => 5.0,
        _ => 0.0,
    }
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

/// All-conference teams and national honors, the day the All-America teams
/// come out (A25). Only impact grows a pipeline: all-conference or better,
/// any draft pick, an undrafted pro signing (topic 4, note 4).
pub(crate) fn honors(w: &mut World) {
    let year = w.year;
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
    let n_all_america = honors.iter().filter(|h| h.1 >= 1.0).count();
    for (i, weight, what) in honors {
        let c = w.people[i].college.unwrap();
        credit_impact(w, i, c, weight, year);
        let age = w.people[i].player.age - 1;
        let school = w.program_name(c).to_string();
        w.people[i].note(format!("At {age} he {what} at {school}."));
    }
    let text = format!("All-America teams are named: {n_all_america} players honored nationally.");
    w.calendar.post(Event::new(w.today, CalLevel::College, Kind::Award, Loudness::Medium, text));
}

/// When a player has used up his college football: four seasons played (five
/// under the five-in-five change), or five years since high school.
fn exhausted(p: &Person, draft_year: i32, five_in_five: bool) -> bool {
    let seasons = if five_in_five { 5 } else { 4 };
    p.college_seasons >= seasons || draft_year - p.class_of >= 5
}

/// December draft reads (T3.1, T3.2): NFL staffs and the advisory committee
/// read every draft-eligible player once the regular season ends. The read
/// drives bowl opt-outs and each underclassman's January choice, which he
/// makes now and acts on by the deadlines.
pub(crate) fn draft_reads(w: &mut World) {
    let year = w.year;
    let draft_year = year + 1;
    let five = w.changes.rules(draft_year).five_in_five;
    let mut rows: Vec<(usize, f64, f64)> = Vec::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        if p.stage != Stage::College {
            continue;
        }
        let Some(c) = p.college else { continue };
        let division = w.programs[c].division;
        if division == Division::JUCO || draft_year - p.class_of < 3 {
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
        let score = grade + 0.5 * position_value(p.player.position)
            - if p.player.position.is_specialist() { 10.0 } else { 0.0 };
        rows.push((i, grade, score));
    }
    let mut board: Vec<f64> = rows.iter().map(|r| r.2).collect();
    board.sort_by(|a, b| b.total_cmp(a));
    let slots = (PIPELINE.draft_picks * w.options.scale).round().max(1.0) as usize;
    for &(i, grade, score) in &rows {
        let rank = board.partition_point(|&v| v > score) + 1;
        let round = if rank <= slots { crate::career::draft_round(rank as u32, slots as u32) } else { 8 };
        let p = &mut w.people[i];
        p.cal.draft_read = Some(super::DraftRead { grade, score, round });
        p.cal.declare_plan = None;
        if exhausted(p, draft_year, five) {
            continue;
        }
        // An underclassman's choice (T3.1): projected high picks go; the
        // rest mostly return. Family money pulls a mid-round prospect. About
        // 80-150 underclassmen declare a year, and only a handful of them
        // with no draft grade (NFL special-eligibility lists, 2019-2025;
        // proposed, not confirmed).
        let chance = match round {
            1 | 2 => 0.85,
            3 | 4 => 0.35 + 0.02 * (p.family.decider_wants.money * 10.0),
            5..=7 => 0.08,
            _ => 0.0005,
        };
        p.cal.declare_plan = Some(p.rng.chance(chance));
    }
}

/// The January draft decisions (T3.1): some declare early (the soft date),
/// the rest by the hard deadline, when seniors' college careers end too; a
/// few withdraw after the advisory read, by the NCAA's date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declare {
    Soft,
    Deadline,
    Withdraw,
}

pub(crate) fn declarations(w: &mut World, phase: Declare) {
    let year = w.year;
    let draft_year = year + 1;
    let date = w.today;
    let five = w.changes.rules(draft_year).five_in_five;
    // Teams still playing: their players declare once their season is over.
    let playing: std::collections::HashSet<usize> = w.college.as_ref().map_or_else(Default::default, |c| {
        c.games.iter().filter(|g| g.played.is_none()).flat_map(|g| [g.home, g.away]).collect()
    });
    let seed = w.options.seed as u64 ^ ((year as u64) << 16) ^ 0xDEC1;
    let mut events: Vec<(usize, String, Loudness)> = Vec::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        match phase {
            Declare::Soft | Declare::Deadline => {
                if p.stage != Stage::College || p.cal.portal.is_some() {
                    continue;
                }
                let Some(c) = p.college else { continue };
                let Some(read) = p.cal.draft_read else {
                    // Junior college careers end by signing with a four-year school, or here.
                    if phase == Declare::Deadline
                        && w.programs[c].division == Division::JUCO
                        && draft_year - p.class_of >= 5
                    {
                        let p = &mut w.people[i];
                        p.stage = Stage::Done;
                        p.note("His junior college years ran out and his playing days ended.");
                    }
                    continue;
                };
                // A team still in the playoff declares after its last game
                // (T3: the title game's players get until about January 24).
                if phase == Declare::Deadline && playing.contains(&c) {
                    continue;
                }
                let senior = exhausted(p, draft_year, five);
                if senior {
                    if phase == Declare::Deadline {
                        let p = &mut w.people[i];
                        p.stage = Stage::Declared;
                        let age = p.player.age;
                        p.note(format!("His college career over at {age}, he entered the draft pool."));
                    }
                    continue;
                }
                if p.cal.declare_plan != Some(true) {
                    continue;
                }
                // The decisive ones go on the soft date once their season is over.
                let early = !playing.contains(&c)
                    && key_uniform(seed, p.player.id as u64)
                        < 0.5 + 0.03 * (p.player.personality.competitiveness - 10.0);
                if phase == Declare::Soft && !early {
                    continue;
                }
                let p = &mut w.people[i];
                p.stage = Stage::Declared;
                let age = p.player.age;
                let round = read.round;
                let proj =
                    if round <= 7 { format!("projected round {round}") } else { "projected to go undrafted".into() };
                p.note(format!("Declared early for the NFL draft at {age} ({proj})."));
                w.log.declared += 1;
                let loud = if round == 1 { Loudness::Medium } else { Loudness::Small };
                events.push((
                    c,
                    format!("{} of {} declares for the NFL draft ({proj}).", p.name, w.programs[c].real_name),
                    loud,
                ));
            }
            Declare::Withdraw => {
                if p.stage != Stage::Declared || exhausted(p, draft_year, five) {
                    continue;
                }
                let Some(read) = p.cal.draft_read else { continue };
                // The advisory committee's read comes back: a poor one sends some home.
                let advisory = read.round as f64 + 1.5 * key_normal(seed ^ 0xAD, p.player.id as u64);
                if advisory >= 5.0 && key_uniform(seed ^ 0xAE, p.player.id as u64) < 0.3 {
                    let p = &mut w.people[i];
                    p.stage = Stage::College;
                    let age = p.player.age;
                    p.note(format!(
                        "Withdrew from the draft at {age} after a poor advisory grade and returned to school."
                    ));
                    w.log.withdrew += 1;
                    if let Some(c) = p.college {
                        events.push((
                            c,
                            format!("{} withdraws from the draft and returns to {}.", p.name, w.programs[c].real_name),
                            Loudness::Small,
                        ));
                    }
                }
            }
        }
    }
    for (c, text, loud) in events {
        w.calendar.post(
            Event::new(date, CalLevel::College, Kind::DraftChoice, loud, text)
                .program(c)
                .rule("NFL: underclassman deadline"),
        );
    }
    if phase == Declare::Withdraw {
        w.rebuild_rosters();
    }
}

/// The NFL combine invite list (A10): the best of the draft pool, by the
/// staffs' December reads (about 320 a year).
pub(crate) fn combine_invites(w: &mut World) {
    let mut pool: Vec<(f64, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::Declared)
        .filter_map(|(i, p)| p.cal.draft_read.map(|r| (r.score, i)))
        .collect();
    pool.sort_by(|a, b| b.0.total_cmp(&a.0));
    let n = ((320.0 * w.options.scale).round() as usize).max(1).min(pool.len());
    for &(_, i) in pool.iter().take(n) {
        w.people[i].cal.combine_invite = true;
    }
    w.log.combine_invites += n as u32;
    let text = format!("The NFL invites {n} prospects to the scouting combine.");
    w.calendar.post(Event::new(w.today, CalLevel::Nfl, Kind::Date, Loudness::Medium, text));
}

/// The combine (A10, A11): measured speed, explosion and strength move a
/// prospect's grade a little: a workout warrior rises, a slow tester falls.
/// Proposed, not confirmed: up to about 1.5 grade points either way.
pub(crate) fn combine(w: &mut World) {
    use crate::attributes::Attr;
    let invited: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].cal.combine_invite).collect();
    if invited.is_empty() {
        return;
    }
    let test = |p: &Person| {
        let a = &p.player.attributes;
        (a[Attr::Speed] + a[Attr::Acceleration] + a[Attr::Agility] + a[Attr::Strength]) / 4.0
    };
    let mut by_pos: std::collections::HashMap<Position, Vec<f64>> = std::collections::HashMap::new();
    for &i in &invited {
        by_pos.entry(w.people[i].player.position).or_default().push(test(&w.people[i]));
    }
    let stats: std::collections::HashMap<Position, (f64, f64)> = by_pos
        .into_iter()
        .map(|(k, v)| {
            let m = v.iter().sum::<f64>() / v.len() as f64;
            let sd = (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt().max(0.5);
            (k, (m, sd))
        })
        .collect();
    let mut risers = Vec::new();
    for &i in &invited {
        let p = &mut w.people[i];
        let (m, sd) = stats[&p.player.position];
        let z = (test(p) - m) / sd;
        if let Some(r) = p.cal.draft_read.as_mut() {
            let shift = clamp(0.6 * z, -1.5, 1.5);
            r.grade += shift;
            r.score += shift;
            if z >= 1.8 {
                risers.push(i);
                let age = p.player.age;
                p.note(format!("Tested among the best at his position at the combine at {age}."));
            }
        }
    }
    if let Some(&i) = risers.first() {
        let text = format!("{} lights up the combine.", w.people[i].name);
        w.calendar.post(Event::new(w.today, CalLevel::Nfl, Kind::News, Loudness::Small, text));
    }
}

/// College all-star games (A9): who is invited, by the staffs' reads.
/// The Senior Bowl takes the best seniors, the Shrine Bowl and the NFLPA
/// Collegiate Bowl the next tiers, the HBCU Legacy Bowl (2022 on) players
/// from HBCU programs. Real rosters are filled from fall invitations, so a
/// senior whose college season is over is invited on his December draft
/// read, before the January deadline puts him in the draft pool.
pub(crate) fn all_star_invites(w: &mut World, game: &'static str) {
    let year = w.year;
    if (game == "HBCU Legacy Bowl" && year + 1 < 2022) || (game == "NFLPA Collegiate Bowl" && year + 1 < 2012) {
        return;
    }
    let draft_year = year + 1;
    let five = w.changes.rules(draft_year).five_in_five;
    let hbcu = |c: usize| matches!(w.programs[c].conference.as_str(), "SWAC" | "MEAC");
    // Teams still playing: their seniors are invited once their season is over.
    let playing: std::collections::HashSet<usize> = w.college.as_ref().map_or_else(Default::default, |c| {
        c.games.iter().filter(|g| g.played.is_none()).flat_map(|g| [g.home, g.away]).collect()
    });
    let done_senior = |p: &Person| {
        p.stage == Stage::College
            && p.cal.portal.is_none()
            && p.college.is_some_and(|c| !playing.contains(&c))
            && exhausted(p, draft_year, five)
    };
    let mut pool: Vec<(f64, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| (p.stage == Stage::Declared || done_senior(p)) && p.cal.all_star.is_none())
        .filter(|(_, p)| {
            exhausted(p, draft_year, five) || game == "Senior Bowl" && p.cal.draft_read.is_some_and(|r| r.round <= 3)
        })
        .filter(|(_, p)| game != "HBCU Legacy Bowl" || p.college.is_some_and(hbcu))
        .filter_map(|(i, p)| p.cal.draft_read.map(|r| (r.score, i)))
        .collect();
    pool.sort_by(|a, b| b.0.total_cmp(&a.0));
    // The best seniors skip the lesser games.
    let skip = match game {
        "East-West Shrine Bowl" | "NFLPA Collegiate Bowl" => (60.0 * w.options.scale).round() as usize,
        _ => 0,
    };
    let size = match game {
        "Senior Bowl" => 110.0,
        "East-West Shrine Bowl" => 110.0,
        "HBCU Legacy Bowl" => 90.0,
        _ => 100.0,
    };
    let n = ((size * w.options.scale).round() as usize).max(1);
    let mut count = 0;
    for &(_, i) in pool.iter().skip(skip).take(n) {
        w.people[i].cal.all_star = Some(game);
        count += 1;
    }
    w.log.all_star_invites += count;
    if count > 0 {
        let text = format!("The {game} names its rosters ({count} players).");
        w.calendar.post(Event::new(w.today, CalLevel::College, Kind::Date, Loudness::Small, text));
    }
}

/// An all-star game's practice week: scouts get a close look, so a player's
/// grade moves toward what he really is. A senior invited before the
/// January deadline plays as a college player still.
pub(crate) fn all_star_game(w: &mut World, game: &'static str) {
    let mut n = 0;
    for p in w
        .people
        .iter_mut()
        .filter(|p| p.cal.all_star == Some(game) && matches!(p.stage, Stage::Declared | Stage::College))
    {
        let level = p.college.map_or(Level::FBS, |_| Level::FBS);
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 16.0, level }, &mut p.rng);
        if let Some(r) = p.cal.draft_read.as_mut() {
            let shift = 0.3 * (rep.overall - (r.grade - 0.0));
            let shift = clamp(shift, -2.0, 2.0);
            r.grade += shift;
            r.score += shift;
        }
        let age = p.player.age;
        p.note(format!("Played in the {game} at {age}."));
        n += 1;
    }
    if n > 0 {
        let text = format!("The {game} is played; scouts watched {n} prospects up close all week.");
        w.calendar.post(Event::new(w.today, CalLevel::College, Kind::Game, Loudness::Small, text));
    }
}

/// The NFL draft (T2): three days in April, team by team (note 7), from the
/// pool that declared in January, in the order of last season's standings.
/// Each team picks from its own board: the consensus grade (aligned across
/// positions, since positions mature at different ages), seen through its
/// front office, plus what the position is worth and what the team needs.
pub(crate) fn draft(w: &mut World) {
    let draft_year = w.year;
    let season = draft_year - 1;
    let seed = w.options.seed as u64;
    let y = w.dates.clone();
    let mut declared: Vec<(usize, f64)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage == Stage::Declared
            && let Some(r) = p.cal.draft_read
        {
            declared.push((i, r.grade));
        }
    }
    let grades: std::collections::HashMap<usize, f64> = declared.iter().copied().collect();
    let slots = (PIPELINE.draft_picks * w.options.scale).round().max(1.0) as usize;
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
    let mut order = super::nfl_year::draft_order(w).unwrap_or_else(|| crate::career::draft_order(&w.nfl, &mut w.rng));
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
            // Kickers, punters and long snappers: a team drafts one only when its
            // own is below par or aging, and even then most are found after the
            // draft (fix list 1.11; proposed, not confirmed).
            if pos.is_specialist() && crate::career::need(&w.nfl[t], pos) <= 0.0 {
                continue;
            }
            let pv = draft_position_value(pos, g) * rf * qb_gate - specialist_draft_discount(pos);
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
    // What it took to go in rounds 1-2 and 3-4 this year, as overall ratings:
    // staffs read their own underclassmen against it (fix list 1.2).
    let mut drafted: Vec<f64> = picked.iter().map(|x| w.people[x.0].player.overall()).collect();
    drafted.sort_by(|a, b| b.total_cmp(a));
    if !drafted.is_empty() {
        let at = |n: usize| drafted[(slots * n / 257).min(drafted.len() - 1)];
        w.draft_bars = Some((at(64), at(138)));
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
    // The picks on their hours (T1.5): round 1 Thursday night, 2-3 Friday,
    // 4-7 Saturday, then the undrafted rush.
    let pick_time = |pick: u32| -> Moment {
        let round = crate::career::draft_round(pick, slots as u32);
        let per_round = (slots as f64 / 7.0).max(1.0);
        let into = (pick as f64 - 1.0) - per_round * (round as f64 - 1.0);
        match round {
            1 => y.draft[0].add_minutes((into * 10.0) as i32),
            2 | 3 => y.draft[1].add_minutes(((into + per_round * (round as f64 - 2.0)) * 7.0) as i32),
            _ => y.draft[2].add_minutes(((into + per_round * (round as f64 - 4.0)) * 5.0) as i32),
        }
    };
    let mut events: Vec<Event> = Vec::new();
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
        credit_impact(w, i, c, weight, season);
        let team_name = how.as_ref().map(|h| w.nfl[h.1].name);
        let team_id = how.as_ref().map(|h| h.1);
        let p = &mut w.people[i];
        p.stage = Stage::Pro;
        let age = p.player.age;
        let when = pick.map_or(y.udfa_rush, pick_time);
        match (how, round, team_name) {
            (Some((pk, _, why)), Some(r), Some(team)) => {
                p.note(format!("Drafted by the {team} in round {r} (pick {pk}) out of {school} at {age}: {why}."));
                let loud = match pk {
                    1 => Loudness::Huge,
                    2..=10 => Loudness::Big,
                    _ if r == 1 => Loudness::Medium,
                    _ => Loudness::Small,
                };
                let text = format!("Pick {pk}: the {team} take {} ({}, {school}).", p.name, p.player.position);
                let mut e = Event::new(when.date, CalLevel::Nfl, Kind::DraftPick, loud, text).at(when).program(c);
                if pk == 1 {
                    e = e.memorable();
                }
                if let Some(t) = team_id {
                    e = e.team(t);
                }
                events.push(e);
            }
            _ => p.note(format!("Signed as an undrafted free agent out of {school} at {age}.")),
        }
        if w.recording {
            let mut a = Alumnus::from_person(p, draft_year, pick, false);
            a.nfl_team = team_name;
            w.alumni.push(a);
        }
    }
    for e in events {
        w.calendar.post(e);
    }
    let udfa_text = format!("The undrafted free agent rush: {udfa} players sign within hours of the last pick.");
    w.calendar
        .post(Event::new(y.udfa_rush.date, CalLevel::Nfl, Kind::Signing, Loudness::Medium, udfa_text).at(y.udfa_rush));
    crate::league::advance_league(&mut w.nfl, &mut w.rng);
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

/// The core GPA the NCAA reads (A30): through the class of 2020 a test score
/// had to match the GPA on the Division I sliding scale (2.3 with a 980 SAT,
/// 3.55 with 400; the 2021 and 2022 classes had tests waived, and they were
/// dropped in 2023). A score short of the scale drops him a tier: a would-be
/// qualifier becomes an academic redshirt (a partial qualifier), and below
/// that he can't enroll.
pub(crate) fn eligibility_gpa(gpa: f64, test: Option<u32>, sign_year: i32) -> f64 {
    let Some(score) = test else { return gpa };
    if sign_year > 2020 {
        return gpa;
    }
    let needed = 2.3 + (980.0 - score as f64) / 464.0;
    if gpa >= needed {
        gpa
    } else if gpa >= 2.3 {
        2.29
    } else {
        gpa.min(1.99)
    }
}

/// A school's academic support, 0-1: tutors, summer school, learning
/// specialists. It follows the athletic department's resources.
fn academic_support(prog: &Program) -> f64 {
    clamp(0.3 + 0.7 * prog.facilities, 0.0, 1.0)
}

/// Real players per FBS roster by position, 2022-24 (fix list topic 1:
/// ESPN rosters with CFBD recruiting links, about 134 teams a year). A staff
/// scales them to its own roster limit. The data lists most linemen and
/// backs only as OL, DL or DB; the teams that list the exact spot carry 48%
/// tackles among linemen, 52% edges in the front and 46% corners in the
/// secondary, which splits the groups (proposed, not confirmed).
fn real_roster(pos: Position) -> f64 {
    match pos {
        Position::QB => 5.8,
        Position::RB => 8.5,
        Position::WR => 16.2,
        Position::TE => 7.4,
        Position::OT => 19.5 * 0.48,
        Position::IOL => 19.5 * 0.52,
        Position::EDGE => 17.5 * 0.52,
        Position::DT => 17.5 * 0.48,
        Position::LB => 14.6,
        Position::CB => 22.2 * 0.46,
        Position::S => 22.2 * 0.54,
        Position::K => 3.5,
        Position::P => 2.0,
        Position::LS => 2.5,
    }
}

/// The real FBS roster these counts add up to.
const REAL_ROSTER_TOTAL: f64 = 119.7;

/// Specialists a staff keeps on scholarship (fix list 1.6): about one kicker
/// and one punter. The other kickers, the second punter and the long
/// snappers walk on (real: 65-82% of FBS specialists have no recruiting
/// record). Other levels do the same (proposed, not confirmed).
fn specialist_scholarships(pos: Position) -> f64 {
    match pos {
        Position::K | Position::P => 1.0,
        _ => 0.0,
    }
}

/// A staff's class plan (fix list 1.1, 1.2, 1.6): how many it signs at each
/// position this year, on scholarship and as walk-ons, and the room it has.
#[derive(Clone, Debug, Default)]
struct ClassPlan {
    scholarship: [usize; POSITION_COUNT],
    walk_on: [usize; POSITION_COUNT],
    /// Scholarship signees it can take this year.
    slots: usize,
    /// Roster places it keeps for walk-ons.
    walk_on_room: usize,
}

/// When the staff reads its roster for a signing stage (T5.2): what this
/// season still adds to everyone's count, whether the winter portal is still
/// ahead, and the most seasons a player gets.
#[derive(Clone, Copy, Debug)]
struct RosterRead {
    stage: SignStage,
    /// Seasons a player gets (five under the five-in-five change).
    limit: u32,
}

impl RosterRead {
    /// Seasons this season still adds: in the summer it hasn't been played,
    /// in December it counts unless a redshirt keeps it (settled January 2).
    fn pending(self, p: &Person) -> u32 {
        match self.stage {
            SignStage::Summer(_) => 1,
            SignStage::Early => p.cal.count_pending as u32,
            _ => 0,
        }
    }

    /// Whether the winter transfer window is still to come.
    fn portal_ahead(self) -> bool {
        matches!(self.stage, SignStage::Summer(_) | SignStage::Early)
    }
}

/// The staff's read of how likely a player is to leave before a season:
/// out of eligibility, an early NFL entrant, or a transfer out (fix list
/// 1.2). `seasons` is what he will have used by then, `draft_year` the draft
/// he would enter. `bars` are the overalls of the 64th and 138th best
/// players taken in the last draft: a player at that level now is read like
/// a round 1-2 or round 3-4 prospect, with the chances of declaring the
/// draft itself uses; a December read (`plan`) replaces the guess.
#[allow(clippy::too_many_arguments)]
fn leaving(
    p: &Person,
    d: Division,
    seasons: u32,
    draft_year: i32,
    limit: u32,
    bars: Option<(f64, f64)>,
    plan: Option<bool>,
    portal: bool,
) -> f64 {
    if d == Division::JUCO {
        // Junior college players move up or stop after two seasons.
        return if seasons >= 2 { 1.0 } else { 0.0 };
    }
    if seasons >= limit || draft_year - p.class_of >= 5 {
        return 1.0;
    }
    let early = if draft_year - p.class_of >= 3 {
        match plan {
            Some(true) => 0.9,
            Some(false) => 0.03,
            None => {
                let ovr = p.player.overall();
                match bars {
                    Some((r2, _)) if ovr >= r2 => 0.85,
                    Some((_, r4)) if ovr >= r4 => 0.35,
                    _ => 0.04,
                }
            }
        }
    } else {
        0.0
    };
    let transfer = if portal {
        match p.last_role {
            Some(PlayingTime::Backup) => PIPELINE.portal_entry.backup,
            Some(PlayingTime::Rotation) => PIPELINE.portal_entry.rotation,
            _ => 0.0,
        }
    } else {
        0.0
    };
    1.0 - (1.0 - early) * (1.0 - transfer)
}

/// The chances a player is gone before next fall, and before the fall after
/// (fix list 1.2, T5.2). In the summer and December the seniors, early
/// entrants and winter transfers are still on the roster: the staff signs
/// for the roster it will have, not the one it has.
fn gone(p: &Person, d: Division, sign_year: i32, bars: Option<(f64, f64)>, read: RosterRead) -> (f64, f64) {
    let seasons = p.college_seasons + read.pending(p);
    // By February the entrants have left; those still here are staying.
    let declared = read.stage != SignStage::Final;
    let next = leaving(
        p,
        d,
        seasons,
        sign_year,
        read.limit,
        bars,
        if declared { p.cal.declare_plan } else { Some(false) },
        read.portal_ahead(),
    );
    let after = leaving(p, d, seasons + 1, sign_year + 1, read.limit, bars, None, true);
    (next, 1.0 - (1.0 - next) * (1.0 - after))
}

/// How many to sign toward a target: this year's hole, or half the hole two
/// years out (the other half is next year's class), whichever is bigger.
/// The two-year half rounds down, so a small room (two snappers) isn't
/// topped up a year early every year (proposed, not confirmed).
fn plan_count(target: f64, now: f64, staying: f64) -> usize {
    let this_year = crate::rng::round_half_up(target - now);
    let ahead = ((target - staying) / 2.0).floor();
    this_year.max(ahead).max(0.0) as usize
}

fn class_plan(
    prog: &Program,
    people: &[Person],
    sign_year: i32,
    seed: u64,
    bars: Option<(f64, f64)>,
    read: RosterRead,
) -> ClassPlan {
    let (limit, signees) = super::ncaa::roster_limits(prog, sign_year);
    let scale = limit as f64 / REAL_ROSTER_TOTAL;
    // [scholarship, walk-on] players expected next fall, and those the staff
    // expects to stay two years.
    let mut now = [[0.0; 2]; POSITION_COUNT];
    let mut stay = [[0.0; 2]; POSITION_COUNT];
    let mut returning = 0.0;
    for &i in &prog.roster {
        let p = &people[i];
        if p.stage != Stage::College {
            continue;
        }
        let (k, wo) = (p.player.position.index(), p.walk_on as usize);
        let (next, two) = gone(p, prog.division, sign_year, bars, read);
        now[k][wo] += 1.0 - next;
        stay[k][wo] += 1.0 - two;
        returning += 1.0 - next;
    }
    let mut plan = ClassPlan::default();
    for pos in Position::ALL {
        let k = pos.index();
        let target = real_roster(pos) * scale;
        if pos.is_specialist() {
            // Each staff has its own habit: 3.1 kickers a roster means some carry 3, some 4.
            let habit = key_uniform(seed ^ 0x5EC1_A115 ^ k as u64, prog.id as u64);
            let total = (target + habit).floor();
            let on_scholarship = specialist_scholarships(pos).min(total);
            plan.scholarship[k] = plan_count(on_scholarship, now[k][0], stay[k][0]);
            plan.walk_on[k] = plan_count(total - on_scholarship, now[k][1], stay[k][1]);
        } else {
            plan.scholarship[k] = plan_count(target, now[k][0] + now[k][1], stay[k][0] + stay[k][1]);
        }
    }
    let room = limit.saturating_sub(returning.round() as usize);
    plan.walk_on_room = plan.walk_on.iter().sum::<usize>().min(room);
    // The era's yearly signing cap (11.4) bounds the class.
    plan.slots = (room - plan.walk_on_room).min(signees);
    plan
}

/// What a kicking camp measures (fix list 1.4): leg power, accuracy and the
/// mental side, weighted as the position rates them, with the mental side
/// read the way the match engine uses it (composure with poise). A long
/// snapper is measured on his snap.
fn specialist_grade(p: &crate::player::Player) -> f64 {
    use crate::attributes::Attr;
    let a = &p.attributes;
    let mental = 0.6 * 5.0 * p.game.composure + 0.4 * a[Attr::Poise];
    match p.position {
        Position::K => (5.0 * a[Attr::KickAccuracy] + 4.0 * a[Attr::KickPower] + mental) / 10.0,
        Position::P => (5.0 * a[Attr::KickPower] + 3.0 * a[Attr::KickAccuracy] + mental) / 9.0,
        _ => p.overall(),
    }
}

/// Camp star tiers for specialists (fix list 1.5), by national rank at each
/// spot in a class: 5 stars for the top 25 (ProKicker's own definition, a
/// top-25 prospect in the country), 4 to 100, 3 to 250, 2 to 500. About
/// 100 kickers a year join FBS rosters, 250 FBS and FCS, 500 any college
/// (proposed, not confirmed).
const CAMP_TIERS: [(u8, f64); 4] = [(5, 25.0), (4, 100.0), (3, 250.0), (2, 500.0)];

/// The most stars the recruiting services give a kicker, punter or long
/// snapper (fix list 1.5): the general sites rarely rate one above 3 stars.
const SERVICE_SPECIALIST_MAX: u8 = 3;

/// The services' stars down a class ranked best first (`specialist` marks
/// the kickers, punters and long snappers): each tier of `quotas` (stars,
/// players) goes to that many players at the regular positions, and a
/// specialist ranked among them gets the tier's stars up to
/// `SERVICE_SPECIALIST_MAX` without using a place.
fn service_stars(specialist: &[bool], quotas: &[(u8, usize)]) -> Vec<u8> {
    let mut out = vec![0u8; specialist.len()];
    let (mut tier, mut used) = (0, 0);
    for (k, &spec) in specialist.iter().enumerate() {
        while tier < quotas.len() && used >= quotas[tier].1 {
            tier += 1;
            used = 0;
        }
        let Some(&(stars, _)) = quotas.get(tier) else { break };
        if spec {
            out[k] = stars.min(SERVICE_SPECIALIST_MAX);
        } else {
            out[k] = stars;
            used += 1;
        }
    }
    out
}

/// The stars staffs recruit a player on (fix list 1.5): the services' for
/// most, the camps' own for a kicker, punter or long snapper.
fn recruiting_stars(p: &Person) -> u8 {
    if p.player.position.is_specialist() { p.camp_stars } else { p.stars }
}

/// What a walk-on spot costs a family next to a scholarship: tuition, as at
/// the Pioneer League schools, which give no athletic aid. Schools that give
/// none anyway cost nothing extra.
fn walk_on_cost(p: &Person, prog: &Program) -> f64 {
    match prog.conference.as_str() {
        "PFL" | "Ivy" => 0.0,
        _ => 0.8 * (1.0 - 0.6 * p.bg.money),
    }
}

/// What a starter at his position earns at a major program, middle of the
/// real range (CBS via SI, 2024; `data/real-world/rates.json`). Tight ends
/// and specialists aren't in the data; long snappers take the specialists'
/// rate (proposed, not confirmed).
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
        Position::K | Position::P | Position::LS => 25_000.0,
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

/// A recruit's own feel for each school (dynasty item 11.1): the campus, the
/// people he met, a friend there, how a visit went. It differs from kid to
/// kid and school to school, so two 5-stars rank the same schools
/// differently. The spread (sd, in utility points; the old one was 0.04)
/// is sized so the top program signs about 14% of a class's FBS 5-stars,
/// as in 247Sports' 2024 and 2025 classes (Ohio State 5 of 36; Georgia and
/// Texas 4 of 32), not the 30-36% of long worlds before (proposed, not
/// confirmed).
const FEEL_SD: f64 = 0.07;

/// How deep down each position board a staff recruits all year (11.18):
/// three targets per spot in its plan, plus three, the way real staffs
/// offer about three recruits per place in a class (proposed, not confirmed).
const RECRUITED_PER_SPOT: usize = 3;
const RECRUITED_EXTRA: usize = 3;

/// Official visits a recruit can take (the NCAA's limit of five).
const OFFICIAL_VISITS: usize = 5;

/// A redshirt's portal rate before his stars (11.17), on the same scale as
/// the backup's 0.18 and the rotation player's 0.07 in `PIPELINE`. These
/// rates go through the portal's bell-shaped draw, so the real yearly
/// chance is far smaller: about 1.1% here against 2.7% for a backup. A
/// redshirt freshman usually waits a year, so he leaves less often than an
/// older backup (proposed, not confirmed).
pub(crate) const REDSHIRT_PORTAL: f64 = 0.12;

/// How much more a recruit cares about playing early, by his stars (dynasty
/// item 11.3): a top recruit expects to play by his second year and leave
/// for the NFL after his third. 1.0 up to two stars, then +0.08 a star, so
/// a 5-star weighs his position room 1.24 times as much. At +0.25 a star
/// 5-stars scattered to 22 programs a class, far more than the real 15-17
/// (proposed, not confirmed).
pub(crate) fn early_weight(stars: u8) -> f64 {
    1.0 + 0.08 * (stars as f64 - 2.0).max(0.0)
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

/// What a recruit or transfer sees of a roster beyond the school's grades
/// (trial 3 item 4, Raichel 2026-10-14): the room at his own position, and
/// the units his position leans on. A QB wants receivers and a line, a corner
/// wants a pass rush. Weights are proposed, not confirmed.
///
/// The room is live (dynasty item 11.2): a recruit who commits, or a
/// transfer who lands, joins it at once, so the next recruit at his position
/// sees him. A recruit who flips leaves it again.
struct RosterView {
    /// Per program and position: (overall, college seasons played, person) of each player.
    players: Vec<Vec<Vec<(f64, u32, usize)>>>,
    /// Per program and position: the starters' average overall.
    unit: Vec<[f64; POSITION_COUNT]>,
    /// Per division and position: the typical starters' average.
    norm: [[f64; POSITION_COUNT]; 4],
    division: Vec<usize>,
}

/// The units each position leans on, with weights.
fn linked(pos: Position) -> &'static [(Position, f64)] {
    use Position::*;
    match pos {
        QB => &[(WR, 0.35), (OT, 0.25), (IOL, 0.2), (TE, 0.2)],
        RB => &[(IOL, 0.45), (OT, 0.35), (TE, 0.2)],
        WR => &[(QB, 0.75), (OT, 0.25)],
        TE => &[(QB, 0.7), (OT, 0.3)],
        OT | IOL => &[(QB, 0.4), (RB, 0.3), (IOL, 0.15), (OT, 0.15)],
        EDGE => &[(DT, 0.5), (CB, 0.3), (LB, 0.2)],
        DT => &[(EDGE, 0.5), (LB, 0.5)],
        LB => &[(DT, 0.6), (EDGE, 0.4)],
        CB => &[(EDGE, 0.5), (S, 0.3), (DT, 0.2)],
        S => &[(CB, 0.5), (LB, 0.3), (EDGE, 0.2)],
        K | P | LS => &[],
    }
}

impl RosterView {
    /// The rosters as they stand, without the players in `gone` (portal entrants on their way out).
    fn new(programs: &[Program], people: &[Person], gone: &std::collections::HashSet<usize>) -> Self {
        let mut players = Vec::with_capacity(programs.len());
        let mut unit = Vec::with_capacity(programs.len());
        let mut sums = [[(0.0, 0.0); POSITION_COUNT]; 4];
        for prog in programs {
            let mut r: Vec<Vec<(f64, u32, usize)>> = vec![Vec::new(); POSITION_COUNT];
            for &i in prog.roster.iter().filter(|i| !gone.contains(i)) {
                let q = &people[i];
                r[q.player.position.index()].push((q.player.overall(), q.college_seasons, i));
            }
            let mut u = [0.0; POSITION_COUNT];
            for pos in Position::ALL {
                let v = &mut r[pos.index()];
                v.sort_by(|a, b| b.0.total_cmp(&a.0));
                let n = crate::game::roster::starters(pos).min(v.len());
                if n > 0 {
                    u[pos.index()] = v[..n].iter().map(|x| x.0).sum::<f64>() / n as f64;
                    let e = &mut sums[prog.division as usize][pos.index()];
                    e.0 += u[pos.index()];
                    e.1 += 1.0;
                }
            }
            players.push(r);
            unit.push(u);
        }
        let norm = sums.map(|d| d.map(|(t, n)| if n > 0.0 { t / n } else { 0.0 }));
        let division = programs.iter().map(|p| p.division as usize).collect();
        RosterView { players, unit, norm, division }
    }

    /// A commitment or a transfer joins program `j`'s room (11.2).
    fn join(&mut self, j: usize, pos: Position, who: usize, overall: f64, seasons: u32) {
        self.players[j][pos.index()].push((overall, seasons, who));
    }

    /// A recruit who flips leaves his old room (11.2).
    fn leave(&mut self, j: usize, pos: Position, who: usize) {
        self.players[j][pos.index()].retain(|e| e.2 != who);
    }

    /// How a program's roster suits him, in utility points. `me` is how good
    /// he expects to be when he wants to start, `within` how many seasons the
    /// players ahead must have left to still be there, and `their_growth`
    /// how much they improve meanwhile. Recruits judge themselves two years
    /// on (about 3 points a year against 1 for older players). `early` is how
    /// much more than usual he cares about playing early (11.3). `who` is
    /// himself, left out of the room.
    #[allow(clippy::too_many_arguments)]
    fn fit(
        &self,
        j: usize,
        who: usize,
        p: &Person,
        wants: &super::background::Wants,
        me: f64,
        within: u32,
        their_growth: f64,
        early: f64,
    ) -> f64 {
        let pos = p.player.position;
        let n = crate::game::roster::starters(pos) as f64;
        let ahead = self.players[j][pos.index()]
            .iter()
            .filter(|&&(o, seasons, i)| i != who && seasons <= within && o + their_growth > me)
            .count() as f64;
        let room = if ahead < n { 1.0 - 0.5 * ahead / n } else { -((ahead - n + 1.0) / n).min(1.0) };
        let d = self.division[j];
        let cast: f64 = linked(pos)
            .iter()
            .map(|&(q, wt)| wt * clamp((self.unit[j][q.index()] - self.norm[d][q.index()]) / 4.0, -1.0, 1.0))
            .sum();
        (0.06 + 0.2 * wants.playing_time) * early * room + (0.04 + 0.15 * wants.development) * cast
    }
}

/// What staffs judge a recruit on before they align it by position: his
/// season's play, or for a kicker or punter what the kicking camps measure
/// (fix list 1.4).
fn film_raw(p: &Person) -> f64 {
    if p.player.position.kicks() {
        specialist_grade(&p.player)
    } else {
        p.last_performance.unwrap_or(p.player.overall())
    }
}

/// Kicking and snapping camps (fix list 1.5, 1.7). Kickers and punters, and
/// each high school's snapper who isn't a ranked prospect at his own
/// position, are seen at camps (more often when the family can pay, as in
/// visibility) and ranked against their own spot by what the camp measures.
/// Stars follow the national rank at the spot (`CAMP_TIERS`) and are the
/// camps' own (`camp_stars`), apart from the services'. Kickers and punters
/// are re-ranked at each rating, the latest standing. Snappers are found
/// once a class, at its first rating (`find_snappers`): a ranked snapper is
/// recruited as a long snapper from then on and keeps those stars.
fn camp_rankings(
    w: &mut World,
    pool: &mut [Recruit],
    seniors: &[usize],
    seed: u64,
    film_scale: (f64, f64),
    find_snappers: bool,
) {
    // Each high school's snapper: the best snapper not at QB, K or P, as on game day.
    let mut snapper: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let schools = if find_snappers { &w.high_schools[..] } else { &[] };
    for s in schools {
        let best = s
            .roster
            .iter()
            .copied()
            .filter(|&i| !matches!(w.people[i].player.position, Position::QB | Position::K | Position::P))
            .max_by(|&a, &b| {
                let f = |i: usize| {
                    let p = &w.people[i].player;
                    p.game.snapping + 0.02 * p.attributes[crate::attributes::Attr::Awareness]
                };
                f(a).total_cmp(&f(b))
            });
        if let Some(i) = best {
            snapper.insert(s.id, i);
        }
    }
    let spots = [Position::K, Position::P, Position::LS];
    let mut lists: [Vec<(f64, usize)>; 3] = Default::default();
    for &k in seniors {
        let i = pool[k].person;
        // A new rating replaces the camps' last one, except a long snapper's.
        if find_snappers || w.people[i].player.position != Position::LS {
            w.people[i].camp_stars = 0;
        }
        let p = &w.people[i];
        let spot = if p.player.position.kicks() {
            p.player.position
        } else if snapper.get(&p.hs) == Some(&i) && p.stars == 0 {
            Position::LS
        } else {
            continue;
        };
        let id = p.player.id as u64;
        if key_uniform(seed ^ 0xCA4D, id) >= 0.3 + 0.7 * p.bg.money {
            continue;
        }
        let grade = if spot == Position::LS {
            crate::positions::long_snapper_rating(p.player.game.snapping, &p.player.attributes)
        } else {
            specialist_grade(&p.player)
        };
        let s = spots.iter().position(|&x| x == spot).unwrap_or(0);
        lists[s].push((grade + 1.5 * key_normal(seed ^ 0xCA4E, id), k));
    }
    let scale = w.options.scale;
    let mut snappers: Vec<usize> = Vec::new();
    for (s, list) in lists.iter_mut().enumerate() {
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (rank, &(_, k)) in list.iter().enumerate() {
            let Some(&(stars, _)) = CAMP_TIERS.iter().find(|t| (rank as f64) < (t.1 * scale).round().max(1.0)) else {
                break;
            };
            let p = &mut w.people[pool[k].person];
            p.camp_stars = stars;
            if spots[s] == Position::LS {
                let from = p.player.position;
                crate::athlete::set_position(&mut p.player, Position::LS);
                let age = p.player.age;
                p.note(format!(
                    "At {age} the snapping camps ranked him a {stars}-star long snapper; colleges recruited him there, not at {from}."
                ));
                snappers.push(k);
            }
        }
    }
    // A new long snapper's film is his snap, aligned like everyone else's,
    // and the services take a new look at his upside.
    let grades: Vec<f64> = snappers.iter().map(|&k| w.people[pool[k].person].player.overall()).collect();
    if grades.len() >= 2 {
        let m = grades.iter().sum::<f64>() / grades.len() as f64;
        let sd = (grades.iter().map(|g| (g - m).powi(2)).sum::<f64>() / grades.len() as f64).sqrt().max(1.0);
        for (&k, &g) in snappers.iter().zip(&grades) {
            pool[k].film = film_scale.0 + film_scale.1 * (g - m) / sd;
        }
    }
    for &k in &snappers {
        let p = &mut w.people[pool[k].person];
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 12.0, level: Level::HS }, &mut p.rng);
        pool[k].projection = rep.potential;
    }
}

/// The day's commitment announcements (T5.2): the verbal commitments
/// decided at the last wave, each on the recruit's own day.
pub(crate) fn announce_commitments(w: &mut World, d: Date) {
    if w.announcements.is_empty() {
        return;
    }
    let (today, later): (Vec<_>, Vec<_>) = w.announcements.iter().copied().partition(|a| a.0 <= d);
    w.announcements = later;
    let month = d.month() as usize - 1;
    for (_, i, id, j) in today {
        let Some(p) = w.people.get(i) else { continue };
        if p.player.id != id || p.stage != Stage::HighSchool || p.cal.commit.is_some() {
            continue;
        }
        let prog_name = w.program_name(j).to_string();
        // His senior season: once his state's season has started, the
        // yearly step has aged him, and the note gives that season's age,
        // the age on its row (as the all-state notes do).
        let started =
            w.hs_season.as_ref().is_some_and(|h| h.states.get(w.high_schools[p.hs].state).is_some_and(|s| s.developed));
        let p = &mut w.people[i];
        p.cal.commit = Some((j, d));
        w.log.commits += 1;
        w.log.commits_by_month[month] += 1;
        let age = p.player.age - started as u32;
        let stars = p.stars;
        let name = p.name.clone();
        let when = if started || d.month() >= 9 { "during his senior season" } else { "before his senior season" };
        p.note(format!("Committed to {prog_name} at {age}, {when}."));
        if stars >= 4 {
            let text = format!("{stars}-star {} {name} commits to {prog_name}.", w.people[i].player.position);
            let loud = if stars == 5 { Loudness::Medium } else { Loudness::Small };
            w.calendar.post(Event::new(d, CalLevel::Recruiting, Kind::Commitment, loud, text).program(j));
        }
    }
}

/// Whether a recruit of class `class_of` may take an official visit on day
/// `d` of football year `season` (T5, NCAA rules): only the class signing
/// next, from the first day of classes of his senior year (September 1
/// here) in every era, and from 2018 (`junior_officials`) also in the
/// junior spring, April 1 to the end of June (the dead period from late
/// June closes it in practice). Nobody else takes one.
pub(crate) fn official_visit_allowed(season: i32, d: Date, class_of: i32, junior_officials: bool) -> bool {
    if class_of != season + 1 {
        return false;
    }
    if d >= Date::ymd(season, 9, 1) {
        return true;
    }
    junior_officials && d >= Date::ymd(season, 4, 1) && d <= Date::ymd(season, 6, 30)
}

/// The day's recruiting actions (T5.3, T5.4), only where the NCAA period
/// allows them: official visits on Fridays outside dead periods, for the
/// class signing next once it may take them (`official_visit_allowed`); in
/// the fall they go to a program at home that weekend, the way fall
/// officials are built around home games. Game-day unofficial visits to
/// nearby programs that offered, and staff drop-ins at high school games in
/// evaluation weeks, most of all for their own commitments. Proposed, not
/// confirmed: a weekend official visit chance of 12% for a recruit with
/// offers left to see, 25% for a nearby home game, 20% a week for a drop-in
/// (40% for a commitment).
pub(crate) fn recruiting_day(w: &mut World, d: Date) {
    use super::Visit;
    use crate::calendar::Weekday;
    let y = &w.dates;
    let period = crate::calendar::recruiting_period(y, d);
    let wd = d.weekday();
    let fall = (9..=11).contains(&d.month());
    let official_day = wd == Weekday::Fri && period.official_visits();
    let game_day = wd == Weekday::Sat && period.unofficial_visits() && fall;
    let drop_day = wd == Weekday::Fri && period == crate::calendar::Period::Evaluation;
    if !(official_day || game_day || drop_day) {
        return;
    }
    let season = y.season;
    let sign_year = season + 1;
    let junior_officials = w.changes.rules(season).junior_officials;
    // Programs at home on a day.
    let home_on = |day: Date| -> std::collections::HashSet<usize> {
        w.college
            .as_ref()
            .map_or_else(Default::default, |c| c.games.iter().filter(|g| g.date == day).map(|g| g.home).collect())
    };
    let home_today = if game_day { home_on(d) } else { Default::default() };
    // A fall official visit is the weekend of a home game: Friday night in,
    // Saturday's game, home on Sunday.
    let fall_officials = official_day && fall;
    let home_tomorrow = if fall_officials { home_on(d.add(1)) } else { Default::default() };
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 20 ^ 0x5649;
    let mut posts = Vec::new();
    let (mut off, mut unoff, mut drops) = (0u32, 0u32, 0u32);
    let n = w.people.len();
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool
            || p.cal.signed.is_some()
            || p.cal.offers.is_empty()
            || p.class_of > sign_year + 1
        {
            continue;
        }
        // The class signing next: seniors from September, juniors before.
        let senior = p.class_of == sign_year && fall;
        let id = p.player.id as u64;
        let mut new: Vec<(usize, Visit)> = Vec::new();
        if official_day && official_visit_allowed(season, d, p.class_of, junior_officials) {
            let taken = p.cal.visits.iter().filter(|v| v.2 == Visit::Official).count();
            let next = p.cal.offers.iter().find(|&&j| {
                !p.cal.visits.iter().any(|v| v.0 == j && v.2 == Visit::Official)
                    && (!fall_officials || home_tomorrow.contains(&j))
            });
            if taken < OFFICIAL_VISITS
                && let Some(&j) = next
                && key_uniform(seed ^ 1, id) < 0.12
            {
                new.push((j, Visit::Official));
            }
        }
        if game_day {
            for &j in p.cal.offers.iter().filter(|&&j| home_today.contains(&j)) {
                if geo::miles(p.home_state, w.programs[j].state) <= 250.0 && key_uniform(seed ^ 2 ^ j as u64, id) < 0.25
                {
                    new.push((j, Visit::Unofficial));
                    break;
                }
            }
        }
        if drop_day && senior {
            for &j in &p.cal.offers {
                let committed = p.cal.commit.is_some_and(|c| c.0 == j);
                let near = geo::miles(p.home_state, w.programs[j].state) <= 250.0;
                if (near || committed) && key_uniform(seed ^ 3 ^ j as u64, id) < if committed { 0.4 } else { 0.2 } {
                    new.push((j, Visit::DropIn));
                }
            }
        }
        if new.is_empty() {
            continue;
        }
        let stars = p.stars;
        let name = p.name.clone();
        let p = &mut w.people[i];
        for (j, v) in new {
            p.cal.visits.push((j, d, v));
            match v {
                Visit::Official => {
                    off += 1;
                    if stars >= 4 {
                        posts.push((
                            j,
                            format!("{stars}-star {name} takes an official visit to {}.", w.programs[j].real_name),
                        ));
                    }
                }
                Visit::Unofficial => unoff += 1,
                Visit::DropIn => drops += 1,
            }
        }
    }
    w.log.official_visits += off;
    w.log.unofficial_visits += unoff;
    w.log.drop_ins += drops;
    for (j, text) in posts {
        w.calendar.post(
            Event::new(d, CalLevel::Recruiting, Kind::Visit, Loudness::Small, text)
                .program(j)
                .rule("NCAA: official visits"),
        );
    }
}

/// The recruiting stages of a class (T5): summer commitments before senior
/// seasons, the early signing period in December (from the class of 2018)
/// or, before it, January enrollees signing aid agreements, and February
/// signing day. Each stage runs the same matching on the recruits still
/// unsigned; only the recruits whose own timing (T5.2) has come decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SignStage {
    /// A commitment wave before the senior season ends (T5.2): 0 to 3.
    Summer(u8),
    Early,
    JanuaryEnrollees,
    Final,
}

/// When a recruit makes up his mind (T5.2), from his personality and family:
/// a kid who decides for himself and loves the game commits early; a family
/// that enjoys the attention, or a handler shopping him, waits; an older
/// brother at a school pulls a commitment early. Proposed, not confirmed:
/// about 60% commit before the early period (10% by late April, 20% by late
/// June after official visits, 18% by late August, 12% in the fall), 28%
/// decide at the early period, 12% wait for February. The real FBS classes
/// sign 80-90% in December, most of them committed months before.
fn commit_timing(p: &Person, seed: u64, juco: bool) -> CommitTiming {
    let id = p.player.id as u64;
    if juco {
        return if key_uniform(seed ^ 0x7132, id) < 0.6 { CommitTiming::Early } else { CommitTiming::Late };
    }
    let f = &p.family;
    let z = 0.8 * key_normal(seed ^ 0x7131, id) + if f.decider == Decider::Himself { 0.25 } else { 0.0 }
        - if f.parent_style == super::background::ParentStyle::SocialMedia { 0.3 } else { 0.0 }
        - if f.decider == Decider::Handler { 0.3 } else { 0.0 }
        + 0.03 * (p.player.personality.love_of_game - 10.0)
        + if f.older_brother_at.is_some() { 0.15 } else { 0.0 }
        - if recruiting_stars(p) >= 4 { 0.1 } else { 0.0 };
    if z > 1.1 {
        CommitTiming::Summer(0)
    } else if z > 0.45 {
        CommitTiming::Summer(1)
    } else if z > 0.05 {
        CommitTiming::Summer(2)
    } else if z > -0.21 {
        CommitTiming::Summer(3)
    } else if z > -1.0 {
        CommitTiming::Early
    } else {
        CommitTiming::Late
    }
}

/// The share of signees who enroll in January (proposed, not confirmed):
/// a few in the 1990s, about 15% of FBS signees by 2016, then 30-45% with
/// the early signing period; blue-chips and good students most often.
fn early_enrollee_chance(sign_year: i32, stars: u8, gpa: f64, division: Division) -> f64 {
    let era = match sign_year {
        ..=1999 => 0.03,
        2000..=2017 => 0.03 + 0.12 * (sign_year - 2000) as f64 / 17.0,
        _ => (0.30 + 0.025 * (sign_year - 2018) as f64).min(0.45),
    };
    let level = match division {
        Division::FBS => 1.0,
        Division::FCS => 0.4,
        _ => 0.0,
    };
    let stars = match stars {
        5 => 1.6,
        4 => 1.3,
        3 => 1.0,
        _ => 0.6,
    };
    let grades = if gpa >= 2.6 { 1.0 } else { 0.35 };
    clamp(era * level * stars * grades, 0.0, 0.9)
}

/// How hard a committed recruit holds to his school (T5.1), in utility
/// points: loyal kids hold on; a head coach leaving since he committed
/// halves it.
fn stickiness(p: &Person, prog: &Program, coaches: &[super::coach::Coach], year: i32) -> f64 {
    let pers = &p.player.personality;
    let loyal = clamp(0.6 + 0.04 * (pers.love_of_game - 10.0) + 0.03 * (pers.coachability - 10.0), 0.3, 1.0);
    let changed = prog.head_coach.is_some_and(|c| coaches[c].job.is_some_and(|j| j.since == year));
    0.18 * loyal * if changed { 0.5 } else { 1.0 }
}

/// A signing stage for the class that signs after `season` (T5). Its
/// counters (commitments, flips, decommits, signees) go to that season's
/// calendar log even when February signing day falls after the Super Bowl,
/// in the next football year, so each log holds one signing day.
pub(crate) fn signing_stage(w: &mut World, stage: SignStage, season: i32) {
    let past = if w.log.year == season { None } else { w.log_history.iter().rposition(|l| l.year == season) };
    if let Some(k) = past {
        std::mem::swap(&mut w.log, &mut w.log_history[k]);
    }
    run_signing_stage(w, stage, season);
    if let Some(k) = past {
        std::mem::swap(&mut w.log, &mut w.log_history[k]);
    }
}

fn run_signing_stage(w: &mut World, stage: SignStage, season: i32) {
    let year = season;
    let sign_year = year + 1;
    let today = w.today;
    // The stage that rates the whole class and records its stars.
    let rating = matches!(stage, SignStage::Early | SignStage::JanuaryEnrollees);
    let summer = matches!(stage, SignStage::Summer(_));
    let seed = w.options.seed as u64 ^ (sign_year as u64) << 20;
    let n = w.people.len();
    let mut pool: Vec<Recruit> = Vec::new();
    // Film aligned by position: each position's top looks the same.
    let mut pos_stats: [(f64, f64, f64); POSITION_COUNT] = [(0.0, 0.0, 0.0); POSITION_COUNT];
    let mut candidates: Vec<usize> = Vec::new();
    for i in 0..n {
        let p = &w.people[i];
        let unsigned = p.cal.signed.is_none();
        let in_class = p.stage == Stage::HighSchool && p.class_of <= sign_year;
        let senior = in_class && unsigned;
        let prep = p.stage == Stage::PrepSchool && unsigned && stage == SignStage::Final;
        let juco = p.stage == Stage::College
            && unsigned
            && !matches!(stage, SignStage::Summer(_))
            && p.college.is_some_and(|c| w.programs[c].division == Division::JUCO)
            && p.college_seasons + p.cal.count_pending as u32 >= 1;
        if senior || prep || juco {
            candidates.push(i);
        }
        // Film is aligned against the whole class, signed or not.
        if in_class {
            let f = film_raw(p);
            let s = &mut pos_stats[p.player.position.index()];
            s.0 += 1.0;
            s.1 += f;
            s.2 += f * f;
        }
    }
    if candidates.is_empty() {
        return;
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
            let raw = film_raw(p);
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
            // All-state honors (A32) put a name in front of every staff in the state.
            let honors = 0.04 * p.cal.all_state.min(2) as f64;
            let visibility = clamp(
                0.05 + 0.3 * school_exposure
                    + 0.2 * network
                    + 0.15 * camps
                    + 0.25 * tape
                    + traffic
                    + viral
                    + bloodline
                    + honors,
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
        if r.juco_mover || !rating {
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
        if r.viral && rating {
            let age = p.player.age;
            p.note(format!("At {age} a highlight clip went viral and got him a first look from recruiters."));
        }
    }

    // --- Star ratings: the services only rank kids they have seen -------------------------
    let seniors: Vec<usize> = (0..pool.len()).filter(|&k| !pool[k].juco_mover).collect();
    let class = 250_000.0 * w.options.scale;
    // The services rate the class at each summer wave and again in the
    // senior season. Each rating replaces the last, so a kid holds only the
    // stars of the latest one (February adds no new stars).
    let rate_now = matches!(stage, SignStage::Summer(_)) || rating;
    if rate_now {
        let mut by_star: Vec<usize> = seniors.clone();
        let star_score = |r: &Recruit| 0.25 * r.film + 0.75 * r.projection - 12.0 * (1.0 - r.visibility).powi(2);
        by_star.sort_by(|&a, &b| star_score(&pool[b]).total_cmp(&star_score(&pool[a])));
        let specialist: Vec<bool> =
            by_star.iter().map(|&k| w.people[pool[k].person].player.position.is_specialist()).collect();
        let quotas: Vec<(u8, usize)> =
            PIPELINE.star_shares.iter().map(|&(stars, share)| (stars, (share * class).round() as usize)).collect();
        for (&k, stars) in by_star.iter().zip(service_stars(&specialist, &quotas)) {
            w.people[pool[k].person].stars = stars;
        }
        // Kickers, punters and long snappers get their own stars from the
        // camps (fix list 1.5); the camps find the snappers once a class,
        // at its first rating.
        camp_rankings(w, &mut pool, &seniors, seed, all_mean_sd, stage == SignStage::Summer(0));
    }
    // When each recruit makes up his mind (T5.2).
    for r in pool.iter() {
        let p = &mut w.people[r.person];
        if p.cal.commit_timing.is_none() {
            p.cal.commit_timing = Some(commit_timing(p, seed, r.juco_mover));
        }
    }

    // Senior year: borderline students with offers coming get pushed over an
    // NCAA line by summer school and retaken courses (note 6). Big programs
    // chase this hardest for blue-chips.
    for &k in seniors.iter().filter(|_| rating) {
        let i = pool[k].person;
        let p = &mut w.people[i];
        let stars = recruiting_stars(p);
        if stars < 3 {
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
            + if stars >= 4 { 0.15 } else { 0.0 }
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

    if w.recording && rating {
        // The general services' ranking; the specialists' camp stars are their own scale.
        let mut counts = [0u32; 6];
        for &k in &seniors {
            let p = &w.people[pool[k].person];
            if !p.player.position.is_specialist() {
                counts[p.stars as usize] += 1;
            }
        }
        w.signing_classes.push((sign_year, counts));
    }
    // Rogue boosters pick the recruits they will pay (11.6).
    if rating {
        super::ncaa::before_matching(w, &seniors.iter().map(|&k| pool[k].person).collect::<Vec<_>>());
    }
    // Committed and signed blue-chips pull their high school teammates (T5.3).
    let mut friends: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for p in w.people.iter() {
        if p.stars >= 4 && p.stage == Stage::HighSchool && p.class_of == sign_year {
            let j = p.cal.signed.map(|s| s.program).or(p.cal.commit.map(|c| c.0));
            if let Some(j) = j {
                friends.entry(p.hs).or_default().push(j);
            }
        }
    }
    let junior_officials = w.changes.rules(year).junior_officials;

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
    // A recruit with a verbal commitment stays in view of his school.
    let mut looked_at = vec![false; pool.len()];
    for &k in &interest {
        looked_at[k] = true;
    }
    interest.extend((0..pool.len()).filter(|&k| !looked_at[k] && w.people[pool[k].person].cal.commit.is_some()));
    // A compact copy of what staffs look at, so the scan stays in cache.
    struct Cand {
        k: usize,
        state: usize,
        hs: usize,
        area: usize,
        stars: u8,
        vis: f64,
        gpa: f64,
        /// The GPA the NCAA reads, with the test score in the test years.
        elig_gpa: f64,
        /// A strict school's special admit: grades trending up, good character
        /// reads, a stable home or a mentor, no off-field trouble (note 6).
        grows_out: bool,
        film: f64,
        proj: f64,
        id: u64,
        pos: Position,
        juco_mover: bool,
        /// The program he holds a verbal commitment to.
        committed: Option<usize>,
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
                stars: recruiting_stars(p),
                vis: r.visibility,
                gpa: r.gpa,
                elig_gpa: eligibility_gpa(r.gpa, p.cal.test_score, sign_year),
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
                pos: p.player.position,
                juco_mover: r.juco_mover,
                committed: p.cal.commit.map(|x| x.0),
            }
        })
        .collect();
    let programs = &w.programs;
    let coaches = &w.coaches;
    // Each staff's class plan per position (fix list 1.1, 1.2, 1.6).
    let habit_seed = w.options.seed as u64;
    let bars = w.draft_bars;
    let read = RosterRead { stage, limit: if w.changes.rules(sign_year).five_in_five { 5 } else { 4 } };
    let plans: Vec<ClassPlan> =
        programs.par_iter().map(|prog| class_plan(prog, &w.people, sign_year, habit_seed, bars, read)).collect();
    // A board per position (fix list 1.1): each staff ranks the recruits it
    // sees against the others at the same spot. Kickers, punters and long
    // snappers are ranked on what the camps measure (fix list 1.4).
    type Board = Vec<(usize, f64, bool)>;
    let boards: Vec<Vec<Board>> = programs
        .par_iter()
        .zip(plans.par_iter())
        .map(|(prog, plan)| {
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
            let mut seen: Vec<(usize, f64, bool, Position)> = Vec::new();
            for c in &cands {
                // A JUCO player moves up to a four-year school or nowhere.
                if c.juco_mover && prog.division == Division::JUCO {
                    continue;
                }
                // A staff always keeps its own commitments in view.
                let ours = c.committed == Some(prog.id);
                // Staffs don't waste offers on blue-chips far above their level.
                if !ours
                    && ((prog.division == Division::D2 && c.stars >= 4)
                        || (prog.division == Division::FCS && c.stars >= 5))
                {
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
                if !ours && see < 1.0 && key_uniform(pseed, c.id) > see {
                    continue;
                }
                // The NCAA floor can't be bent; a school's own bar can, a few times a class.
                let tier = ncaa_tier(prog.division, c.elig_gpa);
                if tier == Eligibility::NonQualifier {
                    continue;
                }
                let special = c.gpa < prog.admissions_bar;
                if special && (tier != Eligibility::Qualifier || !c.grows_out || prog.special_admits == 0) {
                    continue;
                }
                let read = |k: u64| noise_sd * key_normal(pseed.wrapping_add(k), c.id);
                // Staffs offer on film more than on a service's projection.
                let mut eval = 0.6 * (c.film + read(1)) + 0.4 * (c.proj + read(2));
                // A borderline student can lose his eligibility in college. Strong
                // academic support takes that risk more easily, and so does a
                // coach who has to win now.
                let risk = clamp((2.6 - c.gpa) / 0.6, 0.0, 1.0);
                eval -= 4.0 * risk * (1.0 - academic_support(prog)) * (1.0 - 0.6 * pressure);
                // A staff honors its own commitments at every stage, signing
                // days included: five film standard deviations keep a
                // committed recruit above anyone new, so he loses his place
                // only when the spots at his position run out.
                if ours {
                    eval += 5.0 * all_mean_sd.1;
                }
                seen.push((c.k, eval, special, c.pos));
            }
            seen.sort_by(|a, b| b.1.total_cmp(&a.1));
            // Junior college leagues cap out-of-state players (Mississippi and
            // Kansas both do), so one JUCO can't sign every national blue-chip.
            if prog.division == Division::JUCO {
                let mut out_of_state = 0;
                seen.retain(|&(k, _, _, _)| {
                    let p = &w.people[pool[k].person];
                    if p.home_state == prog.state || p.cal.commit.is_some_and(|x| x.0 == prog.id) {
                        return true;
                    }
                    out_of_state += 1;
                    out_of_state <= JUCO_OUT_OF_STATE_OFFERS
                });
            }
            let mut b: Vec<Board> = vec![Vec::new(); POSITION_COUNT];
            for (k, eval, special, pos) in seen {
                let i = pos.index();
                if b[i].len() < 40 * (plan.scholarship[i] + plan.walk_on[i] + 1) {
                    b[i].push((k, eval, special));
                }
            }
            b
        })
        .collect();

    dbg_lap("lists");

    // --- Deferred acceptance: programs offer down their boards; each recruit holds the best offer.
    // Each staff offers to the best recruit left at a position its plan still
    // has open. A clearly better recruit still gets an offer at a full
    // position (fix list 1.3): one about a quarter of a film standard
    // deviation better than the best left at an open spot at a top program,
    // a full one at the bottom (proposed, not confirmed), and at most two
    // over the plan at a spot. Specialists follow the plan exactly.
    #[derive(Clone, Copy)]
    struct Held {
        program: usize,
        /// His utility for it without the position room, which can change
        /// as others commit there (11.2).
        base: f64,
        special: bool,
        walk_on: bool,
        pos: usize,
    }
    const OVER_PLAN: usize = 2;
    let pn = programs.len();
    let mut cursor = vec![[0usize; POSITION_COUNT]; pn];
    let mut held: Vec<Option<Held>> = vec![None; pool.len()];
    let mut taken = vec![0usize; pn];
    let mut taken_walk_on = vec![0usize; pn];
    let mut taken_at = vec![[[0usize; 2]; POSITION_COUNT]; pn];
    let mut special_taken = vec![0u32; pn];
    let mut offers: Vec<Vec<usize>> = vec![Vec::new(); pool.len()];
    let film_sd = all_mean_sd.1;
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
        let talent = match recruiting_stars(p) {
            5 => 2.5,
            4 => 1.0,
            3 => 0.3,
            2 => 0.08,
            _ => 0.03,
        } * (1.3 - 0.6 * rank);
        let pay = (prog.collective.budget / 10.0e6).powf(0.8);
        let cap = prog.collective.budget * 0.6 * if reach[j] { 0.35 } else { 0.25 };
        // Within the real range for his position (11.8). Before NIL a rogue
        // booster's cash is the only money (11.6).
        (position_nil(p.player.position) * talent * pay * if reach[j] { 1.6 } else { 1.0 })
            .min(cap)
            .min(super::ncaa::position_nil_max(p.player.position))
            + super::ncaa::booster_cash(prog, pool[k].person)
    };
    // The next offer a staff makes: (position, walk-on spot).
    let next_offer = |j: usize,
                      cursor: &[[usize; POSITION_COUNT]],
                      taken: &[usize],
                      taken_walk_on: &[usize],
                      taken_at: &[[[usize; 2]; POSITION_COUNT]]|
     -> Option<(usize, bool)> {
        let plan = &plans[j];
        let head = |i: usize| boards[j][i].get(cursor[j][i]).map(|e| e.1);
        let sch_room = taken[j] < plan.slots;
        let mut best: Option<(usize, f64, bool)> = None;
        // The best recruit left at a regular position still open: the bar a full position must clear.
        let mut open_bar: Option<f64> = None;
        for pos in Position::ALL {
            let i = pos.index();
            let Some(e) = head(i) else { continue };
            let sch_open = sch_room && taken_at[j][i][0] < plan.scholarship[i];
            let walk_on_open = taken_walk_on[j] < plan.walk_on_room && taken_at[j][i][1] < plan.walk_on[i];
            if !sch_open && !walk_on_open {
                continue;
            }
            if sch_open && !pos.is_specialist() {
                open_bar = Some(open_bar.map_or(e, |b: f64| b.max(e)));
            }
            if best.is_none_or(|b| e > b.1) {
                best = Some((i, e, !sch_open));
            }
        }
        // Talent beats need (fix list 1.3).
        if let Some(bar) = open_bar {
            let margin = film_sd * (1.0 - 0.75 * programs[j].prestige_rel);
            for pos in Position::ALL.into_iter().filter(|p| !p.is_specialist()) {
                let i = pos.index();
                let Some(e) = head(i) else { continue };
                let full = taken_at[j][i][0] >= plan.scholarship[i];
                if full
                    && taken_at[j][i][0] < plan.scholarship[i] + OVER_PLAN
                    && e >= bar + margin
                    && best.is_none_or(|b| e > b.1)
                {
                    best = Some((i, e, false));
                }
            }
        }
        best.map(|b| (b.0, b.2))
    };
    // --- Relationships built over the year (dynasty item 11.18) -------------------------
    // Staffs recruit the top of each position board all year: the position
    // coach calls and visits every target, the head coach gives his own time
    // to the few at the very top. A recruit takes official visits to the five
    // programs he likes best among those recruiting him (the NCAA's limit),
    // and unofficial ones to any within 250 miles. What a staff builds
    // depends on how long it has known him (better-seen kids are found
    // earlier), how much of the head coach's time he got, whether he saw the
    // campus, and the coaches' recruiting and relationship skills.
    let feel =
        |j: usize, k: usize| FEEL_SD * key_normal(seed ^ 77 ^ j as u64, w.people[pool[k].person].player.id as u64);
    let chemistry =
        |j: usize, k: usize| 0.03 * key_normal(seed ^ 91 ^ j as u64, w.people[pool[k].person].player.id as u64);
    // Per recruit: (program, relationship).
    let relationships: Vec<Vec<(usize, f64)>> = {
        // (program, head coach's own target, board rank share) per recruit.
        let mut targets: Vec<Vec<(usize, bool, f64)>> = vec![Vec::new(); pool.len()];
        for j in 0..pn {
            for i in 0..POSITION_COUNT {
                let plan = plans[j].scholarship[i] + plans[j].walk_on[i];
                let board = &boards[j][i];
                for (r, &(k, _, _)) in board.iter().take(RECRUITED_PER_SPOT * plan + RECRUITED_EXTRA).enumerate() {
                    let share = r as f64 / board.len().max(1) as f64;
                    targets[k].push((j, r <= plans[j].scholarship[i], share));
                }
            }
        }
        targets
            .par_iter()
            .enumerate()
            .map(|(k, list)| {
                if list.is_empty() {
                    return Vec::new();
                }
                let p = &w.people[pool[k].person];
                let wants = p.family.decider_wants;
                // The five he likes best before signing day get his official visits.
                let mut liked: Vec<(f64, usize)> = list
                    .iter()
                    .map(|&(j, _, share)| {
                        (utility(p, &wants, &programs[j], offer_for(j, k, share), 1.0 - share) + feel(j, k), j)
                    })
                    .collect();
                liked.sort_by(|a, b| b.0.total_cmp(&a.0));
                // Official visits before he decides (T5): a summer committer
                // could take none before juniors were allowed them (2018).
                // An April committer has had no time for them; before 2018
                // only a fall committer had.
                let officials = match p.cal.commit_timing {
                    Some(CommitTiming::Summer(0)) => 0,
                    Some(CommitTiming::Summer(1 | 2)) if !junior_officials => 0,
                    _ => OFFICIAL_VISITS,
                };
                // Official visits he really took on their dates (T5.4); the
                // rest of his allowance goes to the schools he likes best.
                let mut official: Vec<usize> =
                    p.cal.visits.iter().filter(|v| v.2 == super::Visit::Official).map(|v| v.0).collect();
                for x in liked.iter() {
                    if official.len() >= officials {
                        break;
                    }
                    if !official.contains(&x.1) {
                        official.push(x.1);
                    }
                }
                let known = clamp(0.35 + 0.65 * pool[k].visibility, 0.35, 1.0);
                list.iter()
                    .map(|&(j, own, _)| {
                        let prog = &programs[j];
                        // Dated contact builds the bond: local kids see a staff
                        // more and for less (T5.3).
                        let contact = p.cal.visits.iter().filter(|v| v.0 == j).count().min(5) as f64;
                        let visited =
                            official.contains(&j) || contact > 0.0 || geo::miles(p.home_state, prog.state) <= 250.0;
                        let group = super::coach::Group::of(p.player.position);
                        let hc = prog.head_coach.map(|c| *w.coaches[c].ability());
                        let hc_skill =
                            0.6 * hc.map_or(9.0, |a| a.recruiting) + 0.4 * hc.map_or(9.0, |a| a.relationships);
                        let pc_skill =
                            prog.position_coaches[group as usize].map_or(9.0, |c| w.coaches[c].ability().recruiting);
                        let hc_time = if own { 1.0 } else { 0.35 } * if visited { 1.0 } else { 0.4 };
                        let pc_time = if visited { 1.0 } else { 0.75 };
                        // A staff member who knows his high school coach starts with trust.
                        let tie = [prog.head_coach, prog.oc, prog.dc]
                            .into_iter()
                            .chain(prog.position_coaches)
                            .flatten()
                            .filter_map(|c| w.coaches[c].ties.get(&p.hs).copied())
                            .fold(0.0, f64::max);
                        // A staff babysits its commitments: calls, visits, drop-ins (T5.3).
                        let babysat = if p.cal.commit.is_some_and(|c| c.0 == j) { 0.02 } else { 0.0 };
                        let rel = known
                            * (0.025 * hc_time
                                + 0.006 * (hc_skill - 10.0) * hc_time
                                + 0.003 * (pc_skill - 10.0) * pc_time)
                            + 0.03 * tie
                            + babysat
                            + 0.004 * contact
                            + chemistry(j, k) * if visited { 1.0 } else { 0.5 };
                        (j, rel)
                    })
                    .collect()
            })
            .collect()
    };
    // A late offer from deep on a board comes with no relationship behind it.
    let relationship =
        |j: usize, k: usize| relationships[k].iter().find(|e| e.0 == j).map_or_else(|| 0.5 * chemistry(j, k), |e| e.1);
    dbg_lap("relationships");

    let mut view = RosterView::new(programs, &w.people, &std::collections::HashSet::new());
    // Signed recruits not yet on campus hold their places (an NLI binds).
    for (who, p) in w.people.iter().enumerate() {
        if let Some(sg) = p.cal.signed {
            let (j, i) = (sg.program, p.player.position.index());
            if sg.walk_on {
                taken_walk_on[j] += 1;
            } else {
                taken[j] += 1;
            }
            taken_at[j][i][sg.walk_on as usize] += 1;
            view.join(j, p.player.position, who, p.player.overall() + 4.0, 0);
        }
    }
    // What a commitment and friends add to a school for him (T5.1, T5.3).
    let pull = |j: usize, k: usize| -> f64 {
        let p = &w.people[pool[k].person];
        let mut u = 0.0;
        if p.cal.commit.is_some_and(|c| c.0 == j) {
            u += stickiness(p, &programs[j], &w.coaches, year);
            // Until the early period a commitment holds its place: he
            // weighs a switch only when he can sign (T5.1).
            if summer {
                u += 10.0;
            }
        }
        if friends.get(&p.hs).is_some_and(|v| v.contains(&j)) {
            u += 0.04;
        }
        u
    };
    let mut held_offer = vec![0.0f64; pool.len()];
    let mut active = true;
    while active {
        active = false;
        for j in 0..pn {
            while let Some((i, walk_on)) = next_offer(j, &cursor, &taken, &taken_walk_on, &taken_at) {
                let board = &boards[j][i];
                let (k, _, special) = board[cursor[j][i]];
                let rank = cursor[j][i] as f64 / board.len().max(1) as f64;
                cursor[j][i] += 1;
                active = true;
                if special && special_taken[j] >= programs[j].special_admits {
                    continue;
                }
                let who = pool[k].person;
                let p = &w.people[who];
                let wants = p.family.decider_wants;
                let prog = &programs[j];
                // A walk-on spot carries no money and costs the family tuition (fix list 1.6).
                let mut offer = if walk_on { 0.0 } else { offer_for(j, k, rank) };
                // His room there as it stands today, commitments included (11.2),
                // weighed more by a better recruit (11.3).
                let me = p.player.overall() + 6.0;
                let early = early_weight(recruiting_stars(p));
                let fit = view.fit(j, who, p, &wants, me, 2, 2.0, early);
                // The relationship the staff built over the year (11.18) and his
                // own feel for the school (11.1).
                let base_u =
                    relationship(j, k) + feel(j, k) + pull(j, k) - if walk_on { walk_on_cost(p, prog) } else { 0.0 };
                let mut u = utility(p, &wants, prog, offer, 1.0 - rank) + base_u + fit;
                // What the school he holds is worth to him now: a recruit at his
                // position may have committed there since (11.2).
                let held_u = held[k].map(|h| h.base + view.fit(h.program, who, p, &wants, me, 2, 2.0, early));
                // Bidding (note 13): a school that wants him near the top of its
                // board raises its offer to beat the one he holds, within its money.
                if let Some(hu) = held_u
                    && hu >= u
                    && rank < 0.3
                    && !walk_on
                {
                    let cap = prog.collective.budget * 0.6 * if reach[j] { 0.35 } else { 0.25 };
                    let raised = (offer * 1.5).min(cap).min(super::ncaa::position_nil_max(p.player.position));
                    let ru = utility(p, &wants, prog, raised, 1.0 - rank) + base_u + fit;
                    if ru > hu {
                        offer = raised;
                        u = ru;
                    }
                }
                // Staffs don't waste an offer on a kid already holding a far better one.
                if held_u.is_some_and(|hu| hu > u + 0.15) {
                    continue;
                }
                offers[k].push(j);
                if held_u.is_some_and(|hu| hu >= u) {
                    continue;
                }
                let (pos, overall) = (p.player.position, p.player.overall());
                if let Some(h) = held[k] {
                    // He trades up: his old program's spot opens again.
                    let c = h.program;
                    view.leave(c, pos, who);
                    if h.walk_on {
                        taken_walk_on[c] -= 1;
                    } else {
                        taken[c] -= 1;
                    }
                    taken_at[c][h.pos][h.walk_on as usize] -= 1;
                    if h.special {
                        special_taken[c] -= 1;
                    }
                }
                held[k] = Some(Held { program: j, base: u - fit, special, walk_on, pos: i });
                // His commitment is news: the next recruit at his position sees
                // him in this room (11.2). He counts as a recruit two years on,
                // the way each recruit judges himself.
                view.join(j, pos, who, overall + 4.0, 0);
                held_offer[k] = offer;
                if walk_on {
                    taken_walk_on[j] += 1;
                } else {
                    taken[j] += 1;
                }
                taken_at[j][i][walk_on as usize] += 1;
                special_taken[j] += special as u32;
            }
        }
    }
    // A recruit whose commitment fell through on signing day (his school's
    // spots at his position ran out) still signs where there is room, as
    // real decommitted recruits nearly always do: of the programs that have
    // him on their boards and a place left, the one he likes best, with no
    // NIL money this late. A scholarship needs a slot in the class and his
    // position no more than two over the plan (a specialist follows the
    // plan exactly); a walk-on needs a walk-on place the plan keeps at his
    // position. The best recruits choose first (proposed, not confirmed).
    let mut scrambled = vec![false; pool.len()];
    if stage == SignStage::Final {
        let mut dropped: Vec<usize> =
            (0..pool.len()).filter(|&k| held[k].is_none() && w.people[pool[k].person].cal.commit.is_some()).collect();
        // Per dropped recruit: (program, special admit, share down its board).
        let mut listed: Vec<Vec<(usize, bool, f64)>> = vec![Vec::new(); pool.len()];
        let mut is_dropped = vec![false; pool.len()];
        for &k in &dropped {
            is_dropped[k] = true;
        }
        if !dropped.is_empty() {
            for (j, by_pos) in boards.iter().enumerate() {
                for board in by_pos {
                    for (r, &(k, _, special)) in board.iter().enumerate() {
                        if is_dropped[k] {
                            listed[k].push((j, special, r as f64 / board.len() as f64));
                        }
                    }
                }
            }
        }
        dropped.sort_by(|&a, &b| pool[b].film.total_cmp(&pool[a].film).then(a.cmp(&b)));
        for k in dropped {
            let who = pool[k].person;
            let p = &w.people[who];
            let wants = p.family.decider_wants;
            let pos = p.player.position;
            let i = pos.index();
            let mut best: Option<(f64, usize, bool, bool)> = None;
            for &(j, special, share) in &listed[k] {
                let plan = &plans[j];
                if special && special_taken[j] >= programs[j].special_admits {
                    continue;
                }
                let scholarship = if pos.is_specialist() {
                    taken[j] < plan.slots && taken_at[j][i][0] < plan.scholarship[i]
                } else {
                    taken[j] < plan.slots && taken_at[j][i][0] < plan.scholarship[i] + OVER_PLAN
                };
                let walk_on = !scholarship;
                if walk_on && (taken_walk_on[j] >= plan.walk_on_room || taken_at[j][i][1] >= plan.walk_on[i]) {
                    continue;
                }
                let u = utility(p, &wants, &programs[j], 0.0, 1.0 - share) + relationship(j, k) + feel(j, k)
                    - if walk_on { walk_on_cost(p, &programs[j]) } else { 0.0 };
                if best.is_none_or(|b| u > b.0) {
                    best = Some((u, j, walk_on, special));
                }
            }
            let Some((u, j, walk_on, special)) = best else { continue };
            held[k] = Some(Held { program: j, base: u, special, walk_on, pos: i });
            offers[k].push(j);
            scrambled[k] = true;
            view.join(j, pos, who, p.player.overall() + 4.0, 0);
            if walk_on {
                taken_walk_on[j] += 1;
            } else {
                taken[j] += 1;
            }
            taken_at[j][i][walk_on as usize] += 1;
            special_taken[j] += special as u32;
        }
    }

    dbg_lap("deferred acceptance");
    if std::env::var("AFM_DEBUG").is_ok() {
        let mut by_div = [0usize; 4];
        for h in held.iter().flatten() {
            by_div[programs[h.program].division as usize] += 1;
        }
        let want: Vec<usize> = (0..4)
            .map(|d| {
                programs.iter().enumerate().filter(|(_, p)| p.division as usize == d).map(|(j, _)| plans[j].slots).sum()
            })
            .collect();
        let listed: usize = boards.iter().flatten().map(|l| l.len()).sum();
        eprintln!(
            "signing {sign_year}: pool {} interest {} listed {listed} signed {:?} slots {:?}",
            pool.len(),
            interest.len(),
            by_div,
            want
        );
    }
    // --- Outcomes ---------------------------------------------------------------------
    // Only recruits whose time has come decide at this stage (T5.2): summer
    // committers commit; at the early period (or, before it, as January
    // enrollees) those who are ready sign with FBS and FCS programs; in
    // February everyone left decides.
    let esp_era = w.dates.early_signing.is_some() || stage == SignStage::Early;
    let mut decided: Vec<(usize, bool)> = Vec::new();
    for (k, r) in pool.iter().enumerate() {
        let p = &w.people[r.person];
        let timing = p.cal.commit_timing.unwrap_or(CommitTiming::Late);
        let division = held[k].map(|h| w.programs[h.program].division);
        let four_year = matches!(division, Some(Division::FBS | Division::FCS));
        let now = match stage {
            SignStage::Summer(wave) => {
                // Those who committed in an earlier wave hold to it until the early period.
                timing == CommitTiming::Summer(wave) && p.cal.commit.is_none() && !r.juco_mover && held[k].is_some()
            }
            SignStage::Early => timing <= CommitTiming::Early && four_year,
            SignStage::JanuaryEnrollees => {
                timing <= CommitTiming::Early
                    && four_year
                    && key_uniform(seed ^ 0xEE, p.player.id as u64)
                        < early_enrollee_chance(sign_year, recruiting_stars(p), r.gpa, division.unwrap_or(Division::D2))
            }
            SignStage::Final => true,
        };
        if !now {
            continue;
        }
        let early_enrollee = match stage {
            SignStage::JanuaryEnrollees => true,
            SignStage::Early => {
                r.juco_mover
                    || key_uniform(seed ^ 0xEE, p.player.id as u64)
                        < early_enrollee_chance(sign_year, recruiting_stars(p), r.gpa, division.unwrap_or(Division::D2))
            }
            _ => false,
        };
        decided.push((k, early_enrollee));
    }
    if std::env::var("AFM_DEBUG").is_ok() {
        let mut by_div = [0usize; 4];
        let mut held_div = [0usize; 4];
        let mut timing = [0usize; 3];
        for (k, _) in &decided {
            if let Some(h) = held[*k] {
                by_div[w.programs[h.program].division as usize] += 1;
            }
        }
        for (k, r) in pool.iter().enumerate() {
            if let Some(h) = held[k] {
                held_div[w.programs[h.program].division as usize] += 1;
            }
            match w.people[r.person].cal.commit_timing {
                Some(CommitTiming::Summer(_)) => timing[0] += 1,
                Some(CommitTiming::Early) => timing[1] += 1,
                _ => timing[2] += 1,
            }
        }
        eprintln!(
            "stage {stage:?} {sign_year}: decided by div {by_div:?}, held by div {held_div:?}, timing {timing:?}"
        );
    }
    // Who offered: the visits of the coming weeks go to these (T5).
    if stage != SignStage::Final {
        for (k, r) in pool.iter().enumerate() {
            let mut list = offers[k].clone();
            list.dedup();
            list.truncate(8);
            w.people[r.person].cal.offers = list;
        }
    }
    let month = today.month() as usize - 1;
    for (k, early_enrollee) in decided {
        let r = pool[k];
        let i = r.person;
        if let SignStage::Summer(wave) = stage {
            // A verbal commitment: news, not a signature (T5.1). The mind is
            // made up at the wave; the recruit announces on a day of his own
            // before the next wave (July 4 is a favorite after June's
            // official visits; proposed, not confirmed).
            let j = held[k].unwrap().program;
            let id = w.people[i].player.id;
            let span = if wave >= 3 { 30.0 } else { 58.0 };
            let u = key_uniform(seed ^ 0xA77, id as u64);
            let day = if wave == 1 && u < 0.08 {
                Date::ymd(today.year(), 7, 4)
            } else {
                today.add(1 + (key_uniform(seed ^ 0xA78, id as u64) * span) as i32)
            };
            w.announcements.push((day, i, id, j));
            continue;
        }
        let from_hs = w.school_name(w.people[i].hs).to_string();
        let committed = w.people[i].cal.commit.map(|c| c.0);
        match held[k] {
            Some(Held { program: j, special, walk_on, .. }) => {
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
                if w.recording && !r.juco_mover && !walk_on && division == Division::FBS {
                    w.fbs_signees_by_state[w.people[i].home_state] += 1;
                    if w.people[i].stars == 5 && !w.people[i].player.position.is_specialist() {
                        w.recruiting_log.five_stars.push((sign_year, j));
                    }
                }
                if stage == SignStage::Final {
                    w.log.signed_late += 1;
                } else {
                    w.log.signed_early += 1;
                }
                w.log.signed_by_division[division as usize][(stage == SignStage::Final) as usize] += 1;
                if committed.is_none() {
                    w.log.commits += 1;
                    w.log.commits_by_month[month] += 1;
                }
                if let Some(c) = committed
                    && c != j
                {
                    w.log.flips += 1;
                    let from = w.program_name(c).to_string();
                    let age = w.people[i].player.age;
                    let text = if scrambled[k] {
                        format!(
                            "His commitment to {from} fell through on signing day at {age}; {prog_name} had room for him."
                        )
                    } else {
                        format!("Flipped his commitment from {from} to {prog_name} at {age}.")
                    };
                    w.people[i].note(text);
                    if w.people[i].stars >= 4 {
                        let name = &w.people[i].name;
                        let text = if scrambled[k] {
                            format!("{name} signs with {prog_name} after his commitment to {from} fell through.")
                        } else {
                            format!("{name} flips from {from} to {prog_name}.")
                        };
                        w.calendar.post(
                            Event::new(today, CalLevel::Recruiting, Kind::Commitment, Loudness::Medium, text)
                                .program(j),
                        );
                    }
                }
                let p = &mut w.people[i];
                p.walk_on = walk_on;
                let step = match (r.juco_mover, division) {
                    (true, Division::JUCO) => Step::Sideways,
                    (true, _) => Step::Up,
                    (false, Division::FBS) => Step::HsToFbs,
                    (false, Division::FCS) => Step::HsToFcs,
                    (false, _) => Step::HsToSmallCollege,
                };
                p.pending_dip = Some(World::adjustment(p, step));
                let age = p.player.age;
                let pos = p.player.position;
                let services = match (p.stars, pos.is_specialist()) {
                    (0, false) => "an unranked recruit".to_string(),
                    (0, true) => format!("an unranked {}", pos.name()),
                    (s, false) => format!("a {s}-star"),
                    (s, true) => format!("a {s}-star {}", pos.name()),
                };
                // A specialist's camp stars are the kicking and snapping camps' own (fix list 1.5).
                let stars = match p.camp_stars {
                    c if c > 0 && pos.is_specialist() => format!("{services} (a {c}-star by the specialist camps)"),
                    _ => services,
                };
                let when = match stage {
                    SignStage::Early => " in the early signing period",
                    SignStage::JanuaryEnrollees => " as a January enrollee",
                    _ if esp_era => " on February signing day",
                    _ => "",
                };
                if r.juco_mover {
                    p.note(format!(
                        "At {age} his JUCO film earned a move up to {prog_name} ({}){when}.",
                        division.name()
                    ));
                } else {
                    let gpa = p.bg.gpa;
                    p.gpa_at_signing = Some(gpa);
                    if walk_on {
                        p.note(format!(
                            "Walked on at {prog_name} ({}) at {age} as {stars} out of {from_hs}: a roster spot without a scholarship (core GPA {gpa:.2}).",
                            division.name()
                        ));
                    } else {
                        p.note(format!(
                            "Signed with {prog_name} ({}) at {age} as {stars} out of {from_hs}{when} (core GPA {gpa:.2}).",
                            division.name()
                        ));
                    }
                    let nil = held_offer[k];
                    if nil >= 10_000.0 && super::ncaa::nil_era(year) {
                        if w.recording {
                            w.programs[j].era.offers.push((year, p.player.position, p.stars, nil));
                        }
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
                    match ncaa_tier(division, eligibility_gpa(gpa, p.cal.test_score, sign_year)) {
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
                p.cal.signed = Some(super::Signed { program: j, walk_on, date: today, early_enrollee });
                p.cal.commit = None;
                // His high school coach gets the credit for sending him up.
                let (hs, group) = (p.hs, super::coach::Group::of(p.player.position));
                let up = match w.programs[j].division {
                    Division::FBS => 1.0,
                    Division::FCS => 0.5,
                    _ => 0.25,
                };
                if w.recording
                    && !r.juco_mover
                    && let Some(c) = w.high_schools[hs].coach
                {
                    w.coaches[c].units[group as usize].moved_up += up;
                }
                let (stars_n, name) = (w.people[i].stars, w.people[i].name.clone());
                if stars_n >= 4 && !r.juco_mover {
                    let text = format!("{stars_n}-star {} {name} signs with {prog_name}.", w.people[i].player.position);
                    let loud = if stars_n == 5 { Loudness::Medium } else { Loudness::Small };
                    w.calendar.post(Event::new(today, CalLevel::Recruiting, Kind::Signing, loud, text).program(j));
                }
            }
            None => {
                if committed.is_some() {
                    w.log.decommits += 1;
                }
                let fell_through = committed.map(|c| w.program_name(c).to_string());
                let p = &mut w.people[i];
                p.cal.commit = None;
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
                    p.note(match &fell_through {
                        Some(school) => format!(
                            "His commitment to {school} fell through; at {age} he went to prep school for a year to fix his grades."
                        ),
                        None => format!("At {age} he went to prep school for a year to fix his grades."),
                    });
                } else {
                    p.stage = Stage::Done;
                    // A ranked recruit who never signs gets a reason (trial 1
                    // flag 5), and so does one whose commitment fell through.
                    if fell_through.is_some() || recruiting_stars(p) >= 3 {
                        let gpa = p.bg.gpa;
                        let why = if gpa < 2.0 {
                            format!(
                                "With a core GPA of {gpa:.2}, under the NCAA's 2.0 floor, no four-year school could take him, and no junior college place came through; he stopped playing."
                            )
                        } else if let Some(school) = &fell_through {
                            format!(
                                "His commitment to {school} fell through, and no other program had room for him; he stopped playing after high school."
                            )
                        } else if offers[k].is_empty() && p.cal.offers.is_empty() {
                            // (`cal.offers` holds the offers of the stage before.)
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
    if stage == SignStage::Final {
        // Seniors nobody signed simply finish high school.
        for p in w.people.iter_mut() {
            if p.stage == Stage::HighSchool && p.class_of <= sign_year && p.cal.signed.is_none() {
                p.stage = Stage::Done;
            }
        }
    }
    let _ = n;
}

/// Signees arrive on campus (T3): January enrollees for the spring semester
/// and spring practice, everyone else for summer school in June.
pub(crate) fn enroll(w: &mut World, early: bool) {
    let mut n = 0;
    for i in 0..w.people.len() {
        let Some(s) = w.people[i].cal.signed else { continue };
        if early && !s.early_enrollee {
            continue;
        }
        let name = w.program_name(s.program).to_string();
        let p = &mut w.people[i];
        if p.stage == Stage::College {
            // A junior college player moving up keeps two seasons at most behind him.
            p.college_seasons = p.college_seasons.min(2);
        }
        p.stage = Stage::College;
        p.college = Some(s.program);
        p.path.push(s.program);
        p.on_notice = false;
        p.cal.signed = None;
        if early {
            let age = p.player.age;
            p.note(format!("Enrolled early at {name} in January at {age}, in time for spring practice."));
        }
        n += 1;
    }
    if early {
        w.log.early_enrollees += n;
    }
    w.rebuild_rosters();
}

// --- The transfer portal (T3) ---------------------------------------------------------

/// Why a player leaves (T3).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Why {
    PlayingTime,
    CoachingChange,
    Steered,
    MoveUp,
    Dismissed,
    Money,
    SpringDepth,
}

impl Why {
    fn reason(self) -> &'static str {
        match self {
            Why::PlayingTime => "playing time",
            Why::CoachingChange => "coaching change",
            Why::Steered => "steered",
            Why::MoveUp => "move up",
            Why::Dismissed => "dismissed",
            Why::Money => "money",
            Why::SpringDepth => "spring depth chart",
        }
    }
    fn phrase(self) -> &'static str {
        match self {
            Why::PlayingTime => "short of snaps",
            Why::CoachingChange => "after a coaching change",
            Why::Steered => "free at last to choose for himself",
            Why::MoveUp => "after standing out a level below",
            Why::Dismissed => "after his dismissal",
            Why::Money => "for a bigger NIL deal",
            Why::SpringDepth => "after losing the spring depth chart battle",
        }
    }
    fn from_reason(r: &str) -> Why {
        match r {
            "coaching change" => Why::CoachingChange,
            "steered" => Why::Steered,
            "move up" => Why::MoveUp,
            "dismissed" => Why::Dismissed,
            "money" => Why::Money,
            "spring depth chart" => Why::SpringDepth,
            _ => Why::PlayingTime,
        }
    }
}

/// The market a window opens: every program's room without this window's
/// entrants, updated as each one lands (11.2, 11.16, fix list 1.10).
pub(crate) struct PortalMarket {
    /// The offseason it belongs to (the season just played) and the season the entrants play next.
    label: i32,
    season: i32,
    serious: std::collections::HashSet<usize>,
    medians: [[f64; POSITION_COUNT]; 4],
    open: Vec<[usize; POSITION_COUNT]>,
    backups_best: Vec<[f64; POSITION_COUNT]>,
    depth: Vec<Vec<Vec<f64>>>,
    adds: Vec<usize>,
    view: RosterView,
}

/// How much of the yearly transfer flow an era had (T11; proposed, not
/// confirmed): before the portal a transfer needed his school's release and
/// sat a year, so about half as many moved; the first portal years about
/// four in five; from 2020 on, today's flow.
fn era_factor(label: i32, rule: crate::calendar::rules::TransferRule) -> f64 {
    use crate::calendar::rules::TransferRule;
    match rule {
        TransferRule::Permission => 0.5,
        TransferRule::OpenPortal if label < 2020 => 0.8,
        _ => 1.0,
    }
}

/// A transfer window opens (T3): who enters, on which day, and when each
/// one decides. Stars and standouts enter on the first days; fringe players
/// trickle in. A head coach talks some of his rotation players into staying
/// first. The spring window takes players who lost the spring depth battle.
pub(crate) fn portal_open(w: &mut World, which: super::clock::PortalWindow, close: Date) {
    use super::clock::PortalWindow;
    let today = w.today;
    // The offseason it belongs to: the winter window follows this season; the
    // spring window comes in the next football year.
    let label = if which == PortalWindow::Spring { w.year - 1 } else { w.year };
    let rule = w.changes.rules(label).transfer;
    let era = era_factor(label, rule);
    let seed = w.options.seed as u64 ^ ((label as u64) << 24) ^ if which == PortalWindow::Spring { 0x5A } else { 0xA5 };
    let n = w.people.len();
    w.rebuild_rosters();
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
    let mut medians = [[99.0; POSITION_COUNT]; 4];
    for (d, list) in starters.iter_mut().enumerate() {
        for (k, v) in list.iter_mut().enumerate() {
            v.sort_by(|a, b| a.total_cmp(b));
            if !v.is_empty() {
                medians[d][k] = v[v.len() / 2];
            }
        }
    }
    let median = |d: Division, pos: Position| medians[d as usize][pos.index()];
    // A head coach meets the players he wants to keep before the window (T3).
    if which == PortalWindow::Winter {
        let sway: Vec<f64> = w
            .programs
            .iter()
            .map(|p| {
                p.head_coach
                    .map_or(1.0, |c| clamp(1.0 - 0.03 * (w.coaches[c].ability().relationships - 10.0), 0.75, 1.25))
            })
            .collect();
        for p in w.people.iter_mut() {
            if p.stage == Stage::College
                && p.cal.lean > 0.0
                && p.last_role == Some(PlayingTime::Rotation)
                && let Some(c) = p.college
            {
                let f = sway[c];
                let why = if f < 1.0 {
                    "his head coach asked him to stay"
                } else {
                    "his head coach didn't fight to keep him"
                };
                p.cal.lean *= f;
                p.cal.lean_events.push((today, why, f));
            }
        }
    }
    let mut entrants: Vec<(usize, Why)> = Vec::new();
    let mut steered_stays: Vec<usize> = Vec::new();
    // A coach-change window is only for the players whose head coach left.
    let new_coach: Vec<bool> = w
        .programs
        .iter()
        .map(|p| p.head_coach.is_some_and(|c| w.coaches[c].job.is_some_and(|j| j.since == w.year)))
        .collect();
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::College || p.cal.portal.is_some() {
            continue;
        }
        let Some(c) = p.college else { continue };
        if which == PortalWindow::CoachChange
            && (!new_coach[c] || !p.cal.lean_events.iter().any(|e| e.1 == "his head coach is gone"))
        {
            continue;
        }
        let div = w.programs[c].division;
        let u = 0.5 + 0.5 * (key_normal(seed, p.player.id as u64) / 3.0).clamp(-1.0, 1.0);
        if which == PortalWindow::Spring {
            // After spring ball: a backup who lost the battle (proposed, not
            // confirmed: a third of the winter rate).
            if div != Division::JUCO
                && p.college_seasons < 4
                && matches!(p.last_role, Some(PlayingTime::Backup | PlayingTime::Rotation))
                && u < 0.35 * p.cal.lean * era
            {
                entrants.push((i, Why::SpringDepth));
            }
            continue;
        }
        if p.player.history.last().is_some_and(|h| h.events.has(crate::player::DevelopmentEvent::Dismissed)) {
            entrants.push((i, Why::Dismissed));
            continue;
        }
        if p.college_seasons >= 4 || div == Division::JUCO || p.cal.declare_plan == Some(true) {
            continue;
        }
        let pos = p.player.position;
        let perf = p.last_performance.unwrap_or(0.0);
        let up = match div {
            Division::FCS => Some(Division::FBS),
            Division::D2 => Some(Division::FCS),
            _ => None,
        };
        // Standouts below FBS get noticed; richer programs quietly reach out first (never punished).
        if let Some(up) = up
            && perf >= median(up, pos) + 1.0
        {
            let tampered = w.programs[c].collective.budget < 2.0e6;
            let chance = PIPELINE.transfer_chance * if tampered { 1.6 } else { 1.0 } * era;
            if u < chance {
                entrants.push((i, Why::MoveUp));
                continue;
            }
        }
        // A kid steered to the wrong school decides once, after his first year, whether to leave.
        if p.steered && p.player.age >= 18 {
            if u < 0.45 * era {
                entrants.push((i, Why::Steered));
                continue;
            }
            steered_stays.push(i);
        }
        // The lean built over the season (T3).
        if u < p.cal.lean * era {
            let changed = p.cal.lean_events.iter().any(|e| e.1 == "his head coach is gone");
            entrants.push((i, if changed { Why::CoachingChange } else { Why::PlayingTime }));
            continue;
        }
        // A greedy starter at a poor program can be bought away.
        if p.last_role == Some(PlayingTime::Starter)
            && super::ncaa::nil_era(label)
            && w.programs[c].collective.budget < 1.0e6
            && div == Division::FBS
            && u < 0.06 * p.family.own_wants.money * 4.0
        {
            entrants.push((i, Why::Money));
        }
    }
    // Friends leaving (T3): a teammate from his high school going makes it easier to go.
    let mut gone_from: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for &(i, _) in &entrants {
        let p = &w.people[i];
        gone_from.insert((p.college.unwrap_or(0), p.hs));
    }
    let entered: std::collections::HashSet<usize> = entrants.iter().map(|e| e.0).collect();
    if which == PortalWindow::Winter {
        for i in 0..n {
            let p = &w.people[i];
            if p.stage != Stage::College
                || entered.contains(&i)
                || p.cal.lean <= 0.0
                || p.cal.declare_plan == Some(true)
            {
                continue;
            }
            let Some(c) = p.college else { continue };
            if !gone_from.contains(&(c, p.hs)) {
                continue;
            }
            let u = 0.5 + 0.5 * (key_normal(seed, p.player.id as u64) / 3.0).clamp(-1.0, 1.0);
            if u < p.cal.lean * era * 1.15 {
                entrants.push((i, Why::PlayingTime));
                w.people[i].cal.lean_events.push((today, "a high school teammate left", 1.15));
            }
        }
    }
    for i in steered_stays {
        w.people[i].steered = false;
    }
    if entrants.is_empty() {
        return;
    }
    // Dates: stars and standouts enter on the first days and decide fast.
    let len = today.days_to(close).max(0) as f64;
    let leaving: std::collections::HashSet<usize> = entrants.iter().map(|e| e.0).collect();
    let mut by_day: std::collections::BTreeMap<Date, u32> = std::collections::BTreeMap::new();
    let mut serious = std::collections::HashSet::new();
    let mut events = Vec::new();
    // Teams still in the playoff (T3): their players get five days after
    // the last game, and play it out first. Bowl teams' entrants sit out.
    let mut playoff_last: std::collections::HashMap<usize, Date> = std::collections::HashMap::new();
    if let Some(cs) = w.college.as_ref() {
        for g in cs.games.iter().filter(|g| g.played.is_none()) {
            if matches!(g.kind, super::schedule::GameKind::Playoff(_) | super::schedule::GameKind::Title)
                && g.division == Division::FBS
            {
                for t in [g.home, g.away] {
                    let e = playoff_last.entry(t).or_insert(g.date);
                    *e = (*e).max(g.date);
                }
            }
        }
        // A team that can still win plays to the title game's date.
        if let Some(title) = cs
            .games
            .iter()
            .filter(|g| g.kind == super::schedule::GameKind::Title && g.division == Division::FBS)
            .map(|g| g.date)
            .max()
        {
            for d in playoff_last.values_mut() {
                *d = (*d).max(title);
            }
        }
    }
    let mut cs_out: Vec<u32> = Vec::new();
    for &(i, why) in &entrants {
        let c = w.people[i].college.unwrap();
        let div = w.programs[c].division;
        let p = &w.people[i];
        let perf = p.last_performance.unwrap_or(p.player.overall());
        let q = clamp((perf - median(div, p.player.position)) / 10.0 + 0.4 + 0.1 * p.stars as f64, 0.0, 1.0);
        let id = p.player.id as u64;
        let enter_in =
            if why == Why::Dismissed { 0.0 } else { (len * (1.0 - q).powi(2) * key_uniform(seed ^ 0xE7, id)).floor() };
        let mut entered_on = today.add(enter_in as i32);
        match playoff_last.get(&c) {
            Some(&last) if why != Why::Dismissed && which != PortalWindow::Spring => {
                let extra = (5.0 * key_uniform(seed ^ 0xE8, id)).floor() as i32;
                entered_on = entered_on.max(last.add(1 + extra));
            }
            _ => cs_out.push(p.player.id),
        }
        let decide_in = 3.0 + (22.0 * (1.0 - q) * key_uniform(seed ^ 0xD7, id)).round();
        let decide_by = entered_on.add(decide_in as i32);
        if why == Why::Dismissed && w.people[i].rng.chance(PIPELINE.college_trouble.serious_share) {
            serious.insert(i);
        }
        let p = &mut w.people[i];
        p.cal.portal = Some(super::PortalEntry { entered: entered_on, decide_by, why: why.reason() });
        // His NIL deal ends when he enters (T3).
        if p.nil_income > 0.0 {
            p.nil_income = 0.0;
            let age = p.player.age;
            p.note(format!("Lost his NIL deal when he entered the portal at {age}."));
        }
        *by_day.entry(entered_on).or_default() += 1;
        if p.stars >= 4 || p.last_role == Some(PlayingTime::Starter) {
            let text =
                format!("{} ({}, {}) enters the transfer portal.", p.name, p.player.position, w.programs[c].real_name);
            let loud = if p.stars >= 4 && p.last_role == Some(PlayingTime::Starter) {
                Loudness::Medium
            } else {
                Loudness::Small
            };
            events.push(Event::new(entered_on, CalLevel::College, Kind::Transfer, loud, text).program(c));
        }
    }
    for e in events {
        w.calendar.post(e);
    }
    w.log.portal_entries += entrants.len() as u32;
    w.log.portal_by_day.extend(by_day);
    // Players still to play a bowl or playoff game sit it out.
    if let Some(cs) = w.college.as_mut() {
        for id in cs_out {
            cs.out.insert(id, "entered the transfer portal");
        }
    }
    let text = match which {
        PortalWindow::Winter => format!("The transfer window opens until {}.", close.pretty()),
        PortalWindow::Spring => format!("The spring transfer window opens until {}.", close.pretty()),
        PortalWindow::CoachChange => {
            format!("Players at programs with a new head coach may enter the portal until {}.", close.pretty())
        }
    };
    let loud = if which == PortalWindow::CoachChange { Loudness::Medium } else { Loudness::Big };
    w.calendar.post(Event::new(today, CalLevel::College, Kind::Window, loud, text).rule("NCAA: transfer window"));
    // The market every entrant shops in.
    let pn = w.programs.len();
    let perf_of = |p: &Person| p.last_performance.unwrap_or(p.player.overall());
    let mut open = vec![[0usize; POSITION_COUNT]; pn];
    let mut backups_best = vec![[f64::NEG_INFINITY; POSITION_COUNT]; pn];
    let mut depth: Vec<Vec<Vec<f64>>> = vec![vec![Vec::new(); POSITION_COUNT]; pn];
    for (j, prog) in w.programs.iter().enumerate() {
        let mut returning = [0usize; POSITION_COUNT];
        for &i in prog.roster.iter().filter(|i| !leaving.contains(i)) {
            let p = &w.people[i];
            let k = p.player.position.index();
            depth[j][k].push(perf_of(p));
            if p.last_role == Some(PlayingTime::Starter) {
                returning[k] += 1;
            } else {
                backups_best[j][k] = backups_best[j][k].max(perf_of(p));
            }
        }
        for pos in Position::ALL {
            open[j][pos.index()] = crate::league::depth_slots(pos).starters.saturating_sub(returning[pos.index()]);
            depth[j][pos.index()].sort_by(|a, b| b.total_cmp(a));
        }
    }
    let view = RosterView::new(&w.programs, &w.people, &leaving);
    let mut market =
        PortalMarket { label, season: label + 1, serious, medians, open, backups_best, depth, adds: vec![0; pn], view };
    // Entrants from an earlier window still deciding keep their place in line.
    if let Some(old) = w.portal_market.take() {
        market.adds = old.adds;
    }
    w.portal_market = Some(market);
    w.portal_pending.extend(entrants.iter().map(|e| e.0));
}

/// Entrants whose day has come decide (T3, T9): each shops among programs
/// that want him (11.16), best first; his first choice can fall through
/// while he waits. No school: his old one may take him back (portal regret),
/// or he leaves football. `force` decides everyone still waiting.
pub(crate) fn portal_day(w: &mut World, force: bool) {
    let today = w.today;
    let Some(mut market) = w.portal_market.take() else {
        w.portal_pending.clear();
        return;
    };
    let mut due: Vec<usize> = Vec::new();
    let mut waiting: Vec<usize> = Vec::new();
    for &i in &w.portal_pending {
        match w.people[i].cal.portal {
            Some(e) if force || e.decide_by <= today => due.push(i),
            Some(_) => waiting.push(i),
            None => {}
        }
    }
    if due.is_empty() {
        w.portal_market = Some(market);
        return;
    }
    let perf_of = |p: &Person| p.last_performance.unwrap_or(p.player.overall());
    due.sort_by(|&a, &b| perf_of(&w.people[b]).total_cmp(&perf_of(&w.people[a])).then(a.cmp(&b)));
    let pn = w.programs.len();
    let season = market.season;
    let one_time = w.changes.rules(season).one_time_transfer;
    let seed = w.options.seed as u64 ^ ((market.label as u64) << 24) ^ 0xFA11;
    // The day's landings (person, program), applied to the rosters in
    // place: a full rebuild every portal day costs more than the decisions.
    let mut landings: Vec<(usize, usize)> = Vec::new();
    for i in due {
        let Some(entry) = w.people[i].cal.portal else { continue };
        let why = Why::from_reason(entry.why);
        let c = w.people[i].college.unwrap();
        let from = w.programs[c].division;
        let pos = w.people[i].player.position;
        let role = w.people[i].last_role;
        let label = market.label;
        let record = |w: &mut World, to: Option<Division>| {
            if w.recording {
                w.recruiting_log.portal.push(super::PortalRecord { year: label, role, why: why.reason(), from, to });
            }
        };
        let perf = perf_of(&w.people[i]);
        let serious = market.serious.contains(&i);
        w.people[i].cal.portal = None;
        if serious && w.people[i].rng.chance(PIPELINE.college_trouble.serious_quit_share) {
            let p = &mut w.people[i];
            p.stage = Stage::Done;
            p.note("After a serious incident no program would take him, and he never played again.");
            record(w, None);
            continue;
        }
        let k = pos.index();
        let medians = market.medians;
        let median = |d: Division| medians[d as usize][k];
        // Who wants him (11.16): a program where he would make the two-deep
        // next season (proposed, not confirmed). A program with an open
        // starting spot reaches out to anyone who beats the backups it
        // has left (fix list 1.10). Nobody else calls.
        let slots = crate::league::depth_slots(pos).starters;
        let wr: &World = w;
        let mk: &PortalMarket = &market;
        let mut ranked: Vec<(usize, f64)> = (0..pn)
            .into_par_iter()
            .filter_map(|j| {
                let ahead = mk.depth[j][k].iter().take_while(|&&v| v >= perf).count();
                let wanted = ahead < 2 * slots || (mk.open[j][k] > 0 && perf > mk.backups_best[j][k]);
                if !wanted
                    || j == c
                    || mk.adds[j] >= 10
                    || wr.programs[j].roster.len() + mk.adds[j] >= super::ncaa::roster_limits(&wr.programs[j], season).0
                {
                    return None;
                }
                let d = wr.programs[j].division;
                // A dismissed player starts over a level down or at JUCO.
                let allowed = match why {
                    Why::Dismissed => d as usize > from as usize || (d == from && !serious && perf >= median(d)),
                    Why::MoveUp => (d as usize) < from as usize,
                    _ => d as usize >= from as usize || d == Division::FBS,
                };
                if !allowed || perf + 2.0 < median(d) - if d as usize > from as usize { 3.0 } else { 0.0 } {
                    return None;
                }
                let p = &wr.people[i];
                let u = utility(p, &p.family.own_wants, &wr.programs[j], wr.programs[j].collective.budget / 40.0, 0.6)
                    + mk.view.fit(j, i, p, &p.family.own_wants, perf, 3, 0.0, early_weight(p.stars));
                Some((j, u))
            })
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        // His first choice can fall through while he waits (T9): another
        // entrant takes the spot, or the staff moves on (about 1% a day).
        let waited = entry.entered.days_to(today).max(1) as f64;
        let fell = ranked.len() >= 2 && key_uniform(seed, w.people[i].player.id as u64) > 0.99f64.powf(waited);
        let pick = if fell { ranked.get(1).copied() } else { ranked.first().copied() };
        let age = w.people[i].player.age - 1;
        let from_name = w.program_name(c).to_string();
        let what = why.phrase();
        let Some((j, _)) = pick else {
            // Portal regret (T3): his old school may take him back.
            let back = why != Why::Dismissed
                && w.programs[c].roster.len() < super::ncaa::roster_limits(&w.programs[c], season).0
                && key_uniform(seed ^ 0xBAC, w.people[i].player.id as u64)
                    < if perf >= median(from) - 2.0 { 0.4 } else { 0.15 };
            let p = &mut w.people[i];
            if back {
                p.note(format!("Entered the portal at {age} {what}, found no new school and returned to {from_name}."));
                w.log.portal_returned += 1;
            } else {
                p.stage = Stage::Done;
                p.note(format!("Entered the portal at {age} {what}, found no school and left football."));
            }
            record(w, None);
            continue;
        };
        if fell {
            let first = w.program_name(ranked[0].0).to_string();
            w.people[i].note(format!("His first choice, {first}, fell through while he waited."));
        }
        let landed = w.programs[j].division;
        record(w, Some(landed));
        w.log.portal_landed += 1;
        market.adds[j] += 1;
        if market.open[j][k] > 0 && perf > market.backups_best[j][k] {
            market.open[j][k] -= 1;
        }
        // He is in that room now, for the entrants still shopping (11.2).
        let at = market.depth[j][k].partition_point(|&v| v >= perf);
        market.depth[j][k].insert(at, perf);
        let (overall, seasons) = (w.people[i].player.overall(), w.people[i].college_seasons);
        market.view.join(j, pos, i, overall, seasons);
        if w.recording {
            w.people[i].transfer_from = w.people[i].last_role;
        }
        let to = w.programs[j].division;
        let to_name = w.program_name(j).to_string();
        let step = if (to as usize) < from as usize {
            Step::Up
        } else if to == from {
            Step::Sideways
        } else {
            Step::Down
        };
        // Before the one-time exception (2021) a transfer into FBS sat a
        // year, unless he came from a junior college or, from 2006 (the
        // graduate transfer exception), had graduated.
        let grad_free = w.people[i].cal.graduated && w.year >= 2006;
        let sits = !one_time && to == Division::FBS && from != Division::JUCO && !grad_free;
        let (stars, starter, name) =
            (w.people[i].stars, w.people[i].last_role == Some(PlayingTime::Starter), w.people[i].name.clone());
        landings.push((i, j));
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
        if sits {
            p.cal.transfer_sit = true;
            p.note("Under the transfer rules of the day he had to sit out a season.");
            w.log.sit_out_transfers += 1;
        }
        if stars >= 4 || starter {
            let text = format!("{name} transfers from {from_name} to {to_name}.");
            w.calendar.post(Event::new(today, CalLevel::College, Kind::Transfer, Loudness::Small, text).program(j));
        }
    }
    w.portal_pending = waiting;
    if w.portal_pending.is_empty() {
        w.rebuild_rosters();
    } else {
        w.portal_market = Some(market);
        for (i, j) in landings {
            let roster = &mut w.programs[j].roster;
            let at = roster.partition_point(|&x| x < i);
            roster.insert(at, i);
        }
        // Anyone no longer on a college roster (gone, declared, moved) drops off.
        let people = &w.people;
        for (j, prog) in w.programs.iter_mut().enumerate() {
            prog.roster.retain(|&i| people[i].stage == Stage::College && people[i].college == Some(j));
        }
    }
}

/// July 1, the new money year (T3, 11.5): each collective (and from 2025
/// each school's revenue share) pays its roster; big money can backfire.
pub(crate) fn nil_payouts(w: &mut World) {
    let pn = w.programs.len();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The services' quotas count only the regular positions; a kicker,
    /// punter or long snapper ranked among them gets 3 stars at most.
    #[test]
    fn services_cap_specialists_at_three_stars() {
        let specialist = [false, true, false, true, false, false, false];
        let quotas = [(5, 1), (4, 2), (3, 1)];
        assert_eq!(service_stars(&specialist, &quotas), vec![5, 3, 4, 3, 4, 3, 0]);
        assert_eq!(service_stars(&[true, false], &[(5, 1)]), vec![3, 5]);
    }

    /// Official visits (T5): the class signing next, from September 1 of the
    /// senior year; from 2018 also April 1 to June 30 of the junior year.
    #[test]
    fn official_visits_follow_the_ncaa_calendar() {
        let ok = official_visit_allowed;
        // Before 2018: seniors only.
        assert!(!ok(2016, Date::ymd(2016, 5, 13), 2017, false), "junior spring");
        assert!(!ok(2016, Date::ymd(2016, 8, 19), 2017, false), "summer before the senior year");
        assert!(ok(2016, Date::ymd(2016, 9, 2), 2017, false), "senior fall");
        assert!(ok(2016, Date::ymd(2017, 1, 13), 2017, false), "senior January");
        assert!(!ok(2016, Date::ymd(2016, 10, 7), 2018, false), "a junior in the fall");
        // From 2018: the junior spring too.
        assert!(ok(2018, Date::ymd(2018, 5, 11), 2019, true), "junior spring");
        assert!(!ok(2018, Date::ymd(2018, 3, 23), 2019, true), "before April 1");
        assert!(!ok(2018, Date::ymd(2018, 8, 17), 2019, true), "summer before the senior year");
        assert!(ok(2018, Date::ymd(2018, 10, 5), 2019, true), "senior fall");
        assert!(!ok(2018, Date::ymd(2018, 10, 5), 2020, true), "a junior in the fall");
    }
}
