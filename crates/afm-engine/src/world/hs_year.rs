//! Section 4, topic 4: each state's high school year on its own real
//! calendar. A state's season starts on its own Friday (shifted from the
//! 2026 dates with Labor Day), plays its regular season week by week, then
//! its playoffs and finals (shifted with Thanksgiving). Around it: grade
//! checks by each state's rule (A29), SAT and ACT dates (A30), homecoming and
//! senior night (A31), all-state teams (A32), weekly state polls (A33), heat
//! rules on practice days (A35, A36), forfeits and no contests (A39), spring
//! ball as a breakout stage (T4.1) and the January all-star games.
//!
//! High school games stay a stand-in (about 90,000 a year is too many for
//! the match engine): a game is won by comparing the two teams' starters as
//! they play this year, plus home field and luck.

use rayon::prelude::*;
use std::collections::HashMap;

use super::coach::TitleKind;
use super::high_school::HsSeason;
use super::season::{self, TeamResult, key_normal, key_uniform, win_chance};
use super::{Stage, World};
use crate::calendar::anchors::{labor_day, thanksgiving};
use crate::calendar::data::{ACT_DATES, GradeCheck, SAT_DATES, hs_calendar};
use crate::calendar::weather::{Severe, weather_at};
use crate::calendar::{Date, Event, Kind, Level, Loudness, Weekday};
use crate::player::PlayingTime;
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::clamp;

/// High school games are close to a coin flip between equal teams: the
/// stand-in's spread in rating points.
const HS_SPREAD: f64 = 3.5;

/// One state's season dates for a year.
#[derive(Clone, Debug)]
pub(crate) struct StateSeason {
    pub start: Date,
    pub finals: Date,
    /// Regular-season game days, by week (a bye week has no games).
    pub weeks: Vec<Date>,
    /// The playoff rounds: quarterfinals, semifinals, final.
    pub rounds: [Date; 3],
    pub developed: bool,
    pub done: bool,
}

/// A classification inside a state: its schools play each other.
#[derive(Clone, Debug)]
struct Group {
    state: usize,
    class: u32,
    schools: Vec<usize>,
    /// Each week's games: (home, away).
    schedule: Vec<Vec<(usize, usize)>>,
    /// Still alive in the playoffs.
    alive: Vec<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct HsSeasonState {
    pub states: Vec<StateSeason>,
    pub results: Vec<TeamResult>,
    groups: Vec<Group>,
    /// Strength lost each week (regular weeks then playoff rounds) to
    /// starters out for grades, by school.
    out: HashMap<usize, Vec<f64>>,
    /// Each school's homecoming and senior night (A31).
    pub marks: Vec<Option<(Date, Date)>>,
    /// The new class of 14-year-olds arrives (before the earliest season).
    pub class_day: Date,
    /// Level ratings and teaching credit, once every state has developed.
    pub ratings_day: Date,
    rated: bool,
}

/// A 2026 date shifted to `year` by the anchor it follows (Labor Day for
/// season starts, Thanksgiving for playoffs and finals), so the weekday stays.
fn shifted(m: u32, d: u32, year: i32, anchor: fn(i32) -> Date) -> Date {
    anchor(year).add(anchor(2026).days_to(Date::ymd(2026, m, d)))
}

/// The year's dates for each state.
fn state_dates(w: &World, year: i32) -> Vec<StateSeason> {
    w.states
        .iter()
        .map(|st| {
            let c = hs_calendar(&st.code);
            let start = shifted(c.start.0, c.start.1, year, labor_day);
            let playoffs = shifted(c.playoffs.0, c.playoffs.1, year, thanksgiving);
            let finals = shifted(c.finals.0, c.finals.1, year, thanksgiving);
            let room = (start.days_to(playoffs) / 7).max(1) as usize;
            let games = (c.games as usize).min(room);
            // Bye weeks go in the middle of the season.
            let byes = room - games;
            let mut weeks = Vec::with_capacity(games);
            let mut k = 0;
            let mut skipped = 0;
            while weeks.len() < games {
                let mid = games / 2 + skipped;
                if skipped < byes && k == mid {
                    skipped += 1;
                } else {
                    weeks.push(start.add(7 * k as i32));
                }
                k += 1;
            }
            let span = (playoffs.days_to(finals) / 7).max(2);
            let semi = playoffs.add(7 * (span / 2));
            StateSeason { start, finals, weeks, rounds: [playoffs, semi, finals], developed: false, done: false }
        })
        .collect()
}

/// A new football year: every state's dates and its weekly schedules, built
/// in February from the rosters of the day.
pub(crate) fn new_season(w: &World) -> HsSeasonState {
    let year = w.year;
    let states = state_dates(w, year);
    let seed = w.options.seed as u64 ^ (year as u64) << 32 ^ 0x4853;
    let mut by: std::collections::BTreeMap<(usize, u32), Vec<usize>> = std::collections::BTreeMap::new();
    for s in &w.high_schools {
        by.entry((s.state, s.classification)).or_default().push(s.id);
    }
    // Groups come out by state, then classification.
    let mut groups: Vec<Group> = by
        .into_iter()
        .map(|((state, class), schools)| Group {
            state,
            class,
            schools,
            schedule: vec![Vec::new(); states[state].weeks.len()],
            alive: Vec::new(),
        })
        .collect();
    let mut marks = vec![None; w.high_schools.len()];
    let mut first = 0;
    while first < groups.len() {
        let state = groups[first].state;
        let end = first + groups[first..].iter().take_while(|g| g.state == state).count();
        let n_weeks = states[state].weeks.len();
        let classes: Vec<(u32, &[usize])> =
            groups[first..end].iter().map(|g| (g.class, g.schools.as_slice())).collect();
        let weeks = state_schedule(&classes, n_weeks, seed ^ (state as u64) << 8);
        // Each game is filed under the home team's classification.
        let mut home_days: Vec<(usize, Date)> = Vec::new();
        for (k, games) in weeks.into_iter().enumerate() {
            for (h, a) in games {
                let class = w.high_schools[h].classification;
                let g = first + groups[first..end].iter().position(|g| g.class == class).unwrap_or(0);
                groups[g].schedule[k].push((h, a));
                home_days.push((h, states[state].weeks[k]));
            }
        }
        // Homecoming: the home game nearest midseason; senior night: the last home game.
        home_days.sort_by_key(|&(h, d)| (h, d));
        let mid = states[state].weeks[n_weeks / 2];
        for days in home_days.chunk_by(|a, b| a.0 == b.0) {
            let hc = days.iter().map(|x| x.1).min_by_key(|d| d.days_to(mid).abs()).unwrap();
            let sn = days.last().unwrap().1;
            marks[days[0].0] = Some((hc, sn));
        }
        first = end;
    }
    let earliest = states.iter().map(|s| s.start).min().unwrap_or(w.dates.hs_school_start);
    let latest = states.iter().map(|s| s.start).max().unwrap_or(w.dates.hs_school_start);
    HsSeasonState {
        states,
        results: vec![TeamResult::default(); w.high_schools.len()],
        groups,
        out: HashMap::new(),
        marks,
        class_day: w.dates.hs_school_start.min(earliest.add(-10)),
        ratings_day: latest.add(1),
        rated: false,
    }
}

/// Shuffles a week's schedule may try before it settles for byes or extra
/// games outside the classification.
const SCHEDULE_TRIES: u64 = 8;

/// A state's regular season, week by week, as (home, away) games, from its
/// classifications (class number and schools). Two schools never meet twice
/// in a regular season; a playoff rematch is fine. Each week pairs schools
/// inside their classification first, in a keyed shuffle. A school left
/// over (an odd classification, or one too small to fill a season without a
/// rematch) plays a school it hasn't met from the nearest classification;
/// one still left has a bye. A week tries a few shuffles and keeps the one with the fewest
/// byes, then the fewest games outside the classification. The weeks are
/// built forward and played in reverse, so the games outside the
/// classification come early, as non-district games do.
fn state_schedule(classes: &[(u32, &[usize])], n_weeks: usize, seed: u64) -> Vec<Vec<(usize, usize)>> {
    const FREE: usize = usize::MAX;
    let ids: Vec<usize> = classes.iter().flat_map(|c| c.1.iter().copied()).collect();
    let class_of: Vec<u32> = classes.iter().flat_map(|c| std::iter::repeat_n(c.0, c.1.len())).collect();
    let n = ids.len();
    let odd = classes.iter().filter(|c| c.1.len() % 2 == 1).count();
    // Who each school has met, by index into `ids`.
    let mut met: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut weeks: Vec<Vec<(usize, usize)>> = Vec::with_capacity(n_weeks);
    for k in 0..n_weeks {
        // The best try so far: (byes, games outside the classification) and its games.
        let mut best = ((usize::MAX, usize::MAX), Vec::new());
        for t in 0..SCHEDULE_TRIES {
            let key = seed ^ (k as u64) << 20 ^ t << 48;
            let shuffle: Vec<f64> = ids.iter().map(|&id| key_uniform(key, id as u64)).collect();
            let mut order: Vec<usize> = (0..n).collect();
            order.sort_by(|&a, &b| class_of[a].cmp(&class_of[b]).then(shuffle[a].total_cmp(&shuffle[b])));
            let mut partner = vec![FREE; n];
            let mut pairs: Vec<(usize, usize)> = Vec::with_capacity(n / 2);
            // Inside the classification.
            for (x, &a) in order.iter().enumerate() {
                if partner[a] != FREE {
                    continue;
                }
                let b = order[x + 1..]
                    .iter()
                    .take_while(|&&b| class_of[b] == class_of[a])
                    .find(|&&b| partner[b] == FREE && !met[a].contains(&b));
                if let Some(&b) = b {
                    (partner[a], partner[b]) = (b, a);
                    pairs.push((a, b));
                }
            }
            let inside = pairs.len();
            // The rest: a school not yet met, from the nearest classification.
            let left: Vec<usize> = order.iter().copied().filter(|&a| partner[a] == FREE).collect();
            for (x, &a) in left.iter().enumerate() {
                if partner[a] != FREE {
                    continue;
                }
                let b = left[x + 1..]
                    .iter()
                    .filter(|&&b| partner[b] == FREE && !met[a].contains(&b))
                    .min_by_key(|&&b| class_of[b].abs_diff(class_of[a]));
                if let Some(&b) = b {
                    (partner[a], partner[b]) = (b, a);
                    pairs.push((a, b));
                }
            }
            let score = (n - 2 * pairs.len(), pairs.len() - inside);
            // Nothing better is possible: every classification paired off,
            // and only the odd ones out play across.
            let full = score.0 <= n % 2 && score.1 <= odd / 2;
            if score < best.0 {
                best = (score, pairs);
            }
            if full {
                break;
            }
        }
        let pairs = best.1;
        for &(a, b) in &pairs {
            met[a].push(b);
            met[b].push(a);
        }
        weeks.push(pairs);
    }
    weeks.reverse();
    weeks
        .into_iter()
        .enumerate()
        .map(|(k, pairs)| {
            pairs.into_iter().map(|(a, b)| if k % 2 == 0 { (ids[a], ids[b]) } else { (ids[b], ids[a]) }).collect()
        })
        .collect()
}

/// The high school day.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(hs) = w.hs_season.as_ref() else { return };
    if d == hs.class_day {
        w.add_class();
        w.rebuild_rosters();
    }
    let n_states = w.states.len();
    for s in 0..n_states {
        let st = w.hs_season.as_ref().unwrap().states[s].clone();
        if st.done {
            continue;
        }
        if d == st.start.add(-1) && !st.developed {
            heat_days(w, s, st.start);
            start_state(w, s);
            poll(w, s, d, "Preseason");
        }
        if let Some(k) = st.weeks.iter().position(|&g| g == d) {
            play_week(w, s, k, d);
        }
        if st.weeks.iter().any(|&g| g.add(3) == d) {
            poll(w, s, d, "Weekly");
        }
        if d == st.rounds[0] {
            seed_playoffs(w, s);
        }
        if let Some(r) = st.rounds.iter().position(|&x| x == d) {
            play_round(w, s, r, d);
        }
        if d == st.finals {
            finish_state(w, s, d);
        }
    }
    let hs = w.hs_season.as_ref().unwrap();
    if d == hs.ratings_day && !hs.rated {
        season::attach_level_ratings(w);
        super::staff::credit_teaching(w);
        w.hs_season.as_mut().unwrap().rated = true;
    }
    // SAT and ACT Saturdays (A30).
    let (y, m, _) = d.parts();
    if d.weekday() == Weekday::Sat
        && SAT_DATES
            .iter()
            .chain(ACT_DATES.iter())
            .any(|&(mm, n)| mm == m && Date::nth_weekday(y, mm, Weekday::Sat, n) == d)
    {
        test_day(w, d);
    }
    // High school spring ball in the states that have it (T4.1): notices
    // early in May, the step by the end of it.
    if (m, d.day()) == (5, 1) {
        spring_notices_hs(w, d);
    }
    if d == Date::last_weekday(y, 5, Weekday::Fri) {
        spring_jumps_hs(w, d);
    }
    // State all-star games for the graduating seniors, late May.
    if d == Date::last_weekday(y, 5, Weekday::Sat) {
        state_all_stars(w, d);
    }
}

/// Heat on August practice days (A35, A36): in the states with a heat rule,
/// a practice day with a heat index of 104 F or more at 3pm is cut or moved
/// indoors. Logged; the step on players waits for training (section 6b).
fn heat_days(w: &mut World, s: usize, start: Date) {
    let c = hs_calendar(&w.states[s].code);
    if !c.heat_rule {
        return;
    }
    let code = w.states[s].code.clone();
    let mut d = Date::ymd(start.year(), 8, 1).min(start.add(-21));
    let mut days = 0;
    while d < start {
        if d.weekday() != Weekday::Sun && weather_at(w.options.seed as u64, s, &code, d, 15).heat_index() >= 104.0 {
            days += 1;
        }
        d = d.add(1);
    }
    w.log.heat_limited_days += days;
}

/// A state's season starts: staffs set positions and depth charts, the
/// year's development lands, team strength is read, and grade checks are
/// scheduled for those who will miss games (A29).
fn start_state(w: &mut World, s: usize) {
    let schools: Vec<usize> = w.states[s].schools.clone();
    // The state's players, read from its schools' rosters (rebuilt on the
    // class day, before any state starts).
    let mut who = vec![false; w.people.len()];
    for &h in &schools {
        for &i in &w.high_schools[h].roster {
            let p = &w.people[i];
            who[i] = p.stage == Stage::HighSchool && p.player.age > 14;
        }
    }
    season::reconsider_positions(w, &who);
    let roles = season::hs_roles(w, &schools);
    season::develop(w, &roles);
    let people = &w.people;
    let mut res: Vec<(usize, TeamResult)> = Vec::with_capacity(schools.len());
    for &h in &schools {
        let (strength, preseason, incidents) = season::team_strength(&w.high_schools[h].roster, people);
        res.push((
            h,
            TeamResult {
                strength,
                preseason,
                incidents,
                roster: w.high_schools[h].roster.len() as u32,
                ..Default::default()
            },
        ));
    }
    let st = w.hs_season.as_ref().unwrap().states[s].clone();
    let n_weeks = st.weeks.len() + 3;
    let check = hs_calendar(&w.states[s].code).grades;
    let seed = w.options.seed as u64 ^ (w.year as u64) << 28 ^ 0x6772;
    let mut out: HashMap<usize, Vec<f64>> = HashMap::new();
    let mut missed_players = 0;
    for &h in &schools {
        let starters = w.high_schools[h]
            .roster
            .iter()
            .filter(|&&i| {
                let p = &w.people[i];
                p.last_role == Some(PlayingTime::Starter) && !p.player.position.is_specialist()
            })
            .count()
            .max(1);
        for &i in &w.high_schools[h].roster.clone() {
            let p = &w.people[i];
            let miss = grade_outs(p, check, n_weeks, st.weeks.len(), seed);
            if miss == 0 {
                continue;
            }
            missed_players += 1;
            let games = miss.count_ones();
            let starter = p.last_role == Some(PlayingTime::Starter) && !p.player.position.is_specialist();
            if starter {
                // A starter out: his backup plays, about 8 points worse.
                let v = out.entry(h).or_insert_with(|| vec![0.0; n_weeks]);
                for (k, x) in v.iter_mut().enumerate() {
                    if miss >> k & 1 == 1 {
                        *x += 8.0 / starters as f64;
                    }
                }
            }
            let p = &mut w.people[i];
            p.cal.hs_grade_out = games;
            if starter {
                let age = p.player.age - 1;
                let rule = match check {
                    GradeCheck::SixWeek => "the state's no pass, no play rule",
                    GradeCheck::Weekly => "weekly grade checks",
                    GradeCheck::NineWeek => "a failing grading period",
                    GradeCheck::Semester => "last semester's grades",
                };
                let g = if games == 1 { "1 game".to_string() } else { format!("{games} games") };
                p.note(format!("At {age} he was ruled ineligible for {g} under {rule}."));
            }
        }
    }
    w.log.hs_grade_outs += missed_players;
    let hs = w.hs_season.as_mut().unwrap();
    for (h, r) in res {
        hs.results[h] = r;
    }
    hs.out.extend(out);
    hs.states[s].developed = true;
}

/// Which weeks a player misses for grades (A29), as a bit per week (regular
/// weeks then playoff rounds). Failing a check grows below a 2.6 GPA and with
/// a poor work ethic (proposed, not confirmed: 30% a check at a 2.0 GPA).
fn grade_outs(p: &super::Person, check: GradeCheck, n_weeks: usize, regular: usize, seed: u64) -> u32 {
    let gpa = p.bg.gpa;
    let q = clamp(0.5 * (2.6 - gpa), 0.0, 0.6) * clamp(1.0 - 0.04 * (p.player.personality.work_ethic - 10.0), 0.6, 1.4);
    if q <= 0.0 {
        return 0;
    }
    let id = p.player.id as u64;
    let all = if n_weeks >= 32 { u32::MAX } else { (1u32 << n_weeks) - 1 };
    match check {
        // Last semester's grades: out for the season, unless summer school fixed them.
        GradeCheck::Semester => {
            if key_uniform(seed ^ 1, id) < 0.4 * q {
                all
            } else {
                0
            }
        }
        // Texas: report cards about every six weeks; a failing grade sits him three games.
        GradeCheck::SixWeek => {
            let mut m = 0;
            for (k, at) in [(1u64, 4usize), (2, 10)] {
                if at < n_weeks && key_uniform(seed ^ (10 + k), id) < q {
                    for j in at..(at + 3).min(n_weeks) {
                        m |= 1 << j;
                    }
                }
            }
            m
        }
        // A weekly check: a failing week sits the next game.
        GradeCheck::Weekly => {
            let mut m = 0;
            for j in 1..n_weeks {
                if key_uniform(seed ^ (100 + j as u64), id) < q / 5.0 {
                    m |= 1 << j;
                }
            }
            m
        }
        // A nine-week grading period: out for the rest of the fall.
        GradeCheck::NineWeek => {
            let at = (regular * 7 / 10).max(1);
            if key_uniform(seed ^ 3, id) < q { all & !((1u32 << at) - 1) } else { 0 }
        }
    }
}

fn adj(hs: &HsSeasonState, school: usize, week: usize) -> f64 {
    hs.out.get(&school).and_then(|v| v.get(week)).copied().unwrap_or(0.0)
}

/// One Friday of a state's regular season: every classification plays.
fn play_week(w: &mut World, s: usize, k: usize, d: Date) {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (k as u64) << 8 ^ 0x6761;
    // Severe weather can cancel a state's Friday (A38): most are made up the
    // next day; some become no contests.
    let code = w.states[s].code.clone();
    let wx = weather_at(w.options.seed as u64, s, &code, d, 19);
    let storm = wx.severe == Some(Severe::Hurricane);
    let hs = w.hs_season.as_mut().unwrap();
    let mut games = 0;
    let mut forfeits = 0;
    let mut no_contests = 0;
    let (mut homecomings, mut senior_nights) = (0, 0);
    for g in hs.groups.iter().filter(|g| g.state == s) {
        let Some(week) = g.schedule.get(k) else { continue };
        for &(h, a) in week {
            let key = (h as u64) << 20 ^ a as u64;
            if hs.results[h].roster == 0 || hs.results[a].roster == 0 {
                continue;
            }
            if storm && key_uniform(seed ^ 0x51, key) < 0.3 || key_uniform(seed ^ 0x52, key) < 0.002 {
                no_contests += 1;
                continue;
            }
            games += 1;
            if let Some((hc, sn)) = hs.marks[h] {
                homecomings += (hc == d) as u32;
                senior_nights += (sn == d) as u32;
            }
            let pe = win_chance(hs.results[h].preseason, hs.results[a].preseason, HS_SPREAD, 0.8);
            hs.results[h].expected += pe;
            hs.results[a].expected += 1.0 - pe;
            // A forfeit (A39): a short roster can't field a team, or an
            // ineligible player is found; it counts as a loss.
            let short = |r: u32| if r < 22 { 0.02 } else { 0.0015 };
            let fh = key_uniform(seed ^ 0x53, key) < short(hs.results[h].roster);
            let fa = !fh && key_uniform(seed ^ 0x54, key) < short(hs.results[a].roster);
            let home_wins = if fh || fa {
                forfeits += 1;
                fa
            } else {
                let p = win_chance(
                    hs.results[h].strength - adj(hs, h, k),
                    hs.results[a].strength - adj(hs, a, k),
                    HS_SPREAD,
                    0.8,
                );
                key_uniform(seed ^ 0x55, key) < p
            };
            let (wn, l) = if home_wins { (h, a) } else { (a, h) };
            hs.results[wn].wins += 1;
            hs.results[l].losses += 1;
        }
    }
    w.log.hs_games += games;
    w.log.hs_forfeits += forfeits;
    w.log.hs_no_contests += no_contests;
    w.log.hs_homecomings += homecomings;
    w.log.hs_senior_nights += senior_nights;
}

/// A state's poll (A33): its top five, by record and how they look.
fn poll(w: &mut World, s: usize, d: Date, which: &str) {
    if !w.recording {
        return;
    }
    let hs = w.hs_season.as_ref().unwrap();
    let mut teams: Vec<(f64, usize)> = w.states[s]
        .schools
        .iter()
        .filter(|&&h| hs.results[h].roster > 0)
        .map(|&h| {
            let r = &hs.results[h];
            (r.wins as f64 - r.losses as f64 + 0.15 * r.strength, h)
        })
        .collect();
    teams.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top: Vec<String> = teams
        .iter()
        .take(5)
        .enumerate()
        .map(|(k, &(_, h))| {
            let r = &hs.results[h];
            if r.games() == 0 {
                format!("{}. {}", k + 1, w.school_name(h))
            } else {
                format!("{}. {} ({}-{})", k + 1, w.school_name(h), r.wins, r.losses)
            }
        })
        .collect();
    if top.is_empty() {
        return;
    }
    let text = format!("{which} {} high school poll: {}.", w.states[s].code, top.join(", "));
    w.calendar.post(Event::new(d, Level::HighSchool, Kind::Poll, Loudness::Small, text));
}

/// The playoff field: each classification's best eight.
fn seed_playoffs(w: &mut World, s: usize) {
    let hs = w.hs_season.as_mut().unwrap();
    let results = &hs.results;
    for g in hs.groups.iter_mut().filter(|g| g.state == s) {
        let mut seeds: Vec<usize> = g.schools.iter().copied().filter(|&h| results[h].roster > 0).collect();
        seeds.sort_by(|&a, &b| {
            let ka = results[a].win_pct() + 0.01 * results[a].strength;
            let kb = results[b].win_pct() + 0.01 * results[b].strength;
            kb.total_cmp(&ka)
        });
        seeds.truncate(8);
        g.alive = seeds;
    }
}

/// A playoff round: the better seed hosts until the final.
fn play_round(w: &mut World, s: usize, r: usize, d: Date) {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (r as u64) << 12 ^ 0x706F;
    let hs = w.hs_season.as_mut().unwrap();
    let week = hs.states[s].weeks.len() + r;
    let mut games = 0;
    let mut alive_all: Vec<(usize, Vec<usize>)> = Vec::new();
    for (gi, g) in hs.groups.iter().enumerate() {
        if g.state != s || g.alive.len() < 2 {
            continue;
        }
        let n = g.alive.len();
        let pow = n.next_power_of_two();
        let byes = pow - n;
        let mut next: Vec<usize> = g.alive[..byes].to_vec();
        let rest = &g.alive[byes..];
        let m = rest.len();
        for k in 0..m / 2 {
            let (a, b) = (rest[k], rest[m - 1 - k]);
            let home = if r == 2 { 0.0 } else { 0.4 };
            let p = win_chance(
                hs.results[a].strength - adj(hs, a, week),
                hs.results[b].strength - adj(hs, b, week),
                HS_SPREAD,
                home,
            );
            let a_wins = key_uniform(seed, (a as u64) << 20 ^ b as u64) < p;
            let (wn, l) = if a_wins { (a, b) } else { (b, a) };
            hs.results[wn].wins += 1;
            hs.results[l].losses += 1;
            next.push(wn);
            games += 1;
        }
        // Keep the seeding order for the next round.
        next.sort_by_key(|x| g.alive.iter().position(|y| y == x));
        alive_all.push((gi, next));
    }
    for (gi, next) in alive_all {
        if (r == 2 || next.len() == 1)
            && let Some(&c) = next.first()
        {
            hs.results[c].state_title = true;
        }
        hs.groups[gi].alive = next;
    }
    w.log.hs_games += games;
    let _ = d;
}

/// A state's season ends with its finals: the season's book, coaches'
/// records and reputations, all-state teams (A32) and the final poll.
fn finish_state(w: &mut World, s: usize, d: Date) {
    let year = w.year;
    {
        let hs = w.hs_season.as_mut().unwrap();
        hs.states[s].done = true;
        for g in hs.groups.iter_mut().filter(|g| g.state == s) {
            // A bracket that ended early (a short field) crowns its last team standing.
            if g.alive.len() == 1 && !hs.results[g.alive[0]].state_title {
                hs.results[g.alive[0]].state_title = true;
            }
        }
    }
    poll(w, s, d, "Final");
    all_state(w, s, d);
    if !w.recording {
        return;
    }
    let schools = w.states[s].schools.clone();
    for h in schools {
        let t = w.hs_season.as_ref().unwrap().results[h].clone();
        if t.games() == 0 {
            continue;
        }
        let school = &mut w.high_schools[h];
        school.seasons.push(HsSeason {
            year,
            wins: t.wins,
            losses: t.losses,
            state_title: t.state_title,
            coach: school.coach,
        });
        if t.state_title {
            school.state_titles += 1;
        }
        if let Some(c) = school.coach {
            let stage = w.high_schools[h].prestige;
            let coach = &mut w.coaches[c];
            coach.wins += t.wins;
            coach.losses += t.losses;
            if t.state_title {
                coach.titles += 1;
                coach.career_titles += 1;
            }
            let locker = clamp(t.incidents as f64 / t.roster.max(1) as f64 * 8.0, 0.0, 1.0);
            coach.season_reputation(t.vs_expectation(), locker, true);
            let title = if t.state_title { TitleKind::State } else { TitleKind::None };
            coach.title_reputation(title, true, stage);
        }
    }
}

/// All-state teams (A32): in each classification, the best starter at each
/// spot on the first team, the next on the second.
fn all_state(w: &mut World, s: usize, d: Date) {
    let groups: Vec<(u32, Vec<usize>)> = w
        .hs_season
        .as_ref()
        .unwrap()
        .groups
        .iter()
        .filter(|g| g.state == s)
        .map(|g| (g.class, g.schools.clone()))
        .collect();
    let code = w.states[s].code.clone();
    let mut named = 0;
    for (class, schools) in groups {
        let mut by_pos: [Vec<(f64, usize)>; POSITION_COUNT] = Default::default();
        for &h in &schools {
            for &i in &w.high_schools[h].roster {
                let p = &w.people[i];
                if p.stage != Stage::HighSchool || p.last_role != Some(PlayingTime::Starter) {
                    continue;
                }
                // Games missed count against him: voters reward a full season.
                let perf = p.last_performance.unwrap_or(p.player.overall()) - 0.6 * p.cal.hs_grade_out as f64;
                by_pos[p.player.position.index()].push((perf, i));
            }
        }
        for pos in Position::ALL {
            let list = &mut by_pos[pos.index()];
            list.sort_by(|a, b| b.0.total_cmp(&a.0));
            // A first and a second team per classification, as many at each
            // spot as start; a smaller world names fewer, rounded by a keyed
            // draw so the count is right on average.
            let per_team = crate::league::depth_slots(pos).starters as f64 * w.options.scale;
            let draw = |t: u64| {
                let u = key_uniform(
                    w.options.seed as u64 ^ 0xA115 ^ (s as u64) << 24 ^ (class as u64) << 16 ^ t,
                    pos.index() as u64,
                );
                (per_team + u).floor() as usize
            };
            let (first, second) = (draw(1), draw(2));
            for (rank, &(_, i)) in list.iter().take(first + second).enumerate() {
                let team = if rank < first { 2 } else { 1 };
                let p = &mut w.people[i];
                p.cal.all_state = p.cal.all_state.max(team);
                let age = p.player.age - 1;
                let which = if team == 2 { "first-team" } else { "second-team" };
                p.note(format!("Named {which} all-state ({code} class {}) at {age}.", class + 1));
                named += 1;
            }
        }
    }
    w.log.hs_all_state += named;
    if w.recording {
        let text = format!("{code} all-state teams are named.");
        w.calendar.post(Event::new(d, Level::HighSchool, Kind::Award, Loudness::Small, text));
    }
}

/// A test Saturday (A30): juniors and seniors take the SAT or ACT (on the
/// SAT scale), best score kept. Most take it in the spring of junior year
/// and retake in the fall; scores follow grades and work habits (proposed,
/// not confirmed: mean 1050 at a 2.9 GPA, 300 points per GPA point, 90 a
/// sitting).
fn test_day(w: &mut World, d: Date) {
    let (y, m, _) = d.parts();
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 16 ^ 0x5341;
    let mut taken = 0;
    for p in w.people.iter_mut() {
        if p.stage != Stage::HighSchool {
            continue;
        }
        // Juniors in the spring, seniors in the fall; seniors still short retake in the spring.
        let junior_spring = m <= 6 && p.class_of == y + 1;
        let senior_fall = m >= 8 && p.class_of == y + 1;
        let senior_retake = m <= 6 && p.class_of == y && p.cal.test_score.is_some_and(|s| s < 1000);
        if !(junior_spring || senior_fall || senior_retake) {
            continue;
        }
        let id = p.player.id as u64;
        let chance = match p.cal.test_score {
            None => 0.3,
            Some(s) if s < 1000 => 0.25,
            Some(_) => 0.05,
        };
        if key_uniform(seed, id) >= chance {
            continue;
        }
        let raw = 1050.0
            + 300.0 * (p.bg.gpa - 2.9)
            + 20.0 * (p.player.personality.work_ethic - 10.0)
            + 90.0 * key_normal(seed ^ 1, id);
        let score = (clamp(raw, 400.0, 1600.0) / 10.0).round() as u32 * 10;
        p.cal.test_score = Some(p.cal.test_score.map_or(score, |s| s.max(score)));
        taken += 1;
    }
    if w.recording && taken > 0 {
        let which = if SAT_DATES.iter().any(|&(mm, n)| mm == m && Date::nth_weekday(y, mm, Weekday::Sat, n) == d) {
            "SAT"
        } else {
            "ACT"
        };
        let text = format!("{which} test day: {taken} high school players sit the test.");
        w.calendar.post(Event::new(d, Level::Recruiting, Kind::Academic, Loudness::Small, text));
    }
}

// --- Spring ball as a breakout stage (T4.1) ---------------------------------------------

/// How right the mix is for a spring step (T4.1): work ethic,
/// coachability, a good position coach, open reps on the depth chart, a body
/// still filling out and room under his hidden ceiling. About zero is average.
fn spring_mix(w: &World, i: usize, teaching: f64, open: bool) -> f64 {
    let p = &w.people[i];
    let pers = &p.player.personality;
    let pos = p.player.position;
    let mut room = 0.0;
    let mut n = 0.0;
    for key in crate::attributes::Attr::ALL {
        if crate::positions::weight(pos, key) == 0.0 {
            continue;
        }
        room += (p.player.ceilings[key] - p.player.attributes[key]).max(0.0);
        n += 1.0;
    }
    let room = room / f64::max(n, 1.0);
    let body = if p.player.age <= 19 { 0.3 } else { 0.0 };
    0.3 * (pers.work_ethic - 10.0) / 3.0
        + 0.2 * (pers.coachability - 10.0) / 3.0
        + 0.25 * (teaching - 10.0) / 3.0
        + 0.35 * (room - 8.0) / 4.0
        + if open { 0.35 } else { -0.2 }
        + body
}

/// The share of eligible players who make a real spring step: about the top
/// 3% (proposed, not confirmed).
const SPRING_SHARE: f64 = 0.03;
/// The share of notices that land (T4.1: some don't pan out): about half
/// (the build's pick, proposed, not confirmed). Only a noticed player can
/// step, so a staff tips about twice as many players as step.
const NOTICES_LAND: f64 = 0.5;

/// The age on his latest season row. A row covers the season and the
/// offseason after it, so every note about either gives this age and a page
/// reads one age per season.
fn row_age(p: &super::Person) -> u32 {
    p.player.history.last().map_or(p.player.age, |h| h.age)
}

/// The value the top `share` of `values` clear.
fn top_bar(values: &[f64], share: f64) -> f64 {
    if values.is_empty() {
        return f64::INFINITY;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| b.total_cmp(a));
    let k = ((share * v.len() as f64).round() as usize).clamp(1, v.len());
    v[k - 1]
}

/// Whether a player's spot is open: no more than one healthy player ahead of
/// him at his position on the roster.
fn open_reps(w: &World, roster: &[usize], i: usize) -> bool {
    let p = &w.people[i];
    let mine = p.player.overall();
    let ahead = roster
        .iter()
        .filter(|&&j| {
            j != i
                && w.people[j].player.position == p.player.position
                && matches!(w.people[j].stage, Stage::College | Stage::HighSchool)
                && w.people[j].player.overall() > mine
        })
        .count();
    ahead <= crate::league::depth_slots(p.player.position).starters
}

/// The spring candidates: (person, his position coach's teaching, open reps).
fn college_spring_candidates(w: &World) -> Vec<(usize, f64, bool)> {
    let year = w.year;
    // Read only, so worked out on all cores; the list keeps the people order.
    w.people
        .par_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            // True freshmen (early enrollees), redshirt freshmen and sophomores.
            if p.stage != Stage::College || p.college_seasons > 1 || p.class_of < year - 1 {
                return None;
            }
            // Short of the grades after the fall term: no spring practice (A23).
            if p.cal.fall_ineligible {
                return None;
            }
            let c = p.college?;
            let prog = &w.programs[c];
            let group = super::coach::Group::of(p.player.position);
            let teaching =
                prog.position_coaches[group as usize].or(prog.head_coach).map_or(9.0, |c| w.coaches[c].teaching(group));
            Some((i, teaching, open_reps(w, &prog.roster, i)))
        })
        .collect()
}

fn hs_spring_candidates(w: &World) -> Vec<(usize, f64, bool)> {
    w.people
        .par_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            if p.stage != Stage::HighSchool || !(15..=16).contains(&p.player.age) {
                return None;
            }
            let school = &w.high_schools[p.hs];
            if !hs_calendar(&w.states[school.state].code).spring_ball {
                return None;
            }
            let group = super::coach::Group::of(p.player.position);
            let teaching = school.coach.map_or(7.0, |c| w.coaches[c].teaching(group));
            Some((i, teaching, open_reps(w, &school.roster, i)))
        })
        .collect()
}

/// Notices early in spring (T4.1): the staff reads the mix, sharper with a
/// better staff, so some notices don't pan out. A step never comes without
/// one.
fn notices(w: &mut World, cands: &[(usize, f64, bool)], d: Date, college: bool) {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 20 ^ if college { 0x5350 } else { 0x5348 };
    let mut n = 0;
    let reads: Vec<f64> = cands
        .iter()
        .map(|&(i, teaching, open)| {
            let mix = spring_mix(w, i, teaching, open);
            let sd = clamp(0.45 - 0.03 * (teaching - 10.0), 0.2, 0.8);
            mix + sd * key_normal(seed, w.people[i].player.id as u64)
        })
        .collect();
    let bar = top_bar(&reads, SPRING_SHARE / NOTICES_LAND);
    for (&(i, _, _), &read) in cands.iter().zip(&reads) {
        if read < bar {
            continue;
        }
        n += 1;
        let p = &mut w.people[i];
        p.cal.spring_notice = true;
        let first = p.name.split_whitespace().next().unwrap_or("He").to_string();
        let what = match super::coach::Group::of(p.player.position) {
            super::coach::Group::OL | super::coach::Group::DL => {
                "keeps his weight up and keeps winning his one-on-ones"
            }
            super::coach::Group::QB => "keeps reading it this fast and keeps his feet clean",
            _ => "keeps stacking good days and keeps his footwork sharp",
        };
        let age = row_age(p);
        p.note(format!("Spring notice at {age}: if {first} {what}, he'll likely make a big step."));
        if college && p.stars >= 4 {
            let text = format!("Spring practice: {} is turning heads.", p.name);
            let prog = p.college;
            let mut e = Event::new(d, Level::College, Kind::Breakout, Loudness::Small, text);
            if let Some(c) = prog {
                e = e.program(c);
            }
            w.calendar.post(e);
        }
    }
    w.log.spring_notices += n;
}

/// By the spring game the step lands, or the notice fizzles (T4.1). Only a
/// noticed player can step: the better half of the notices by the mix land.
/// The step closes 12-30% of the gap to his ceilings (proposed, not
/// confirmed) and goes on his latest season row, so the row's end runs
/// through it.
fn jumps(w: &mut World, cands: &[(usize, f64, bool)], d: Date, college: bool) {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 20 ^ if college { 0x5347 } else { 0x5346 };
    let (mut breakouts, mut fizzles) = (0, 0);
    let noticed: Vec<(usize, f64, bool)> =
        cands.iter().copied().filter(|&(i, _, _)| w.people[i].cal.spring_notice).collect();
    // The mix is the cause; a little luck in how it lands.
    let lands: Vec<f64> = noticed
        .iter()
        .map(|&(i, teaching, open)| {
            spring_mix(w, i, teaching, open) + 0.25 * key_normal(seed, w.people[i].player.id as u64)
        })
        .collect();
    let bar = top_bar(&lands, NOTICES_LAND);
    for (&(i, _, _), &land) in noticed.iter().zip(&lands) {
        let id = w.people[i].player.id as u64;
        let p = &mut w.people[i];
        p.cal.spring_notice = false;
        let mut gain = 0.0;
        if land >= bar {
            let share = 0.12 + 0.18 * key_uniform(seed ^ 1, id);
            let before = p.player.attributes;
            gain = crate::development::spring_jump(&mut p.player, share, &mut p.rng);
            // Under half a point is no step: he had no room left to make one.
            if gain < 0.5 {
                p.player.attributes = before;
            }
        }
        let age = row_age(p);
        if gain < 0.5 {
            fizzles += 1;
            p.note(format!("At {age} the spring step he was tipped for didn't come."));
            continue;
        }
        crate::development::record_spring_step(&mut p.player, gain);
        breakouts += 1;
        let r#where = if college { "spring practice" } else { "spring ball" };
        p.note(format!(
            "Broke out in {where} at {age}, as his coaches had hinted: +{gain:.1} overall by the spring game."
        ));
        if college && (p.stars >= 4 || gain >= 4.0) {
            let text = format!("{} breaks out in spring practice (+{gain:.1}).", p.name);
            let prog = p.college;
            let mut e = Event::new(d, Level::College, Kind::Breakout, Loudness::Medium, text);
            if let Some(c) = prog {
                e = e.program(c);
            }
            w.calendar.post(e);
        }
    }
    w.log.spring_breakouts += breakouts;
    w.log.spring_fizzles += fizzles;
}

/// College spring practice opens: notices (T4.1).
pub(crate) fn spring_notices(w: &mut World) {
    let d = w.today;
    let cands = college_spring_candidates(w);
    notices(w, &cands, d, true);
}

/// The spring game: the college steps land.
pub(crate) fn spring_game(w: &mut World) {
    let d = w.today;
    let cands = college_spring_candidates(w);
    jumps(w, &cands, d, true);
    if w.recording {
        w.calendar.post(Event::new(
            d,
            Level::College,
            Kind::Practice,
            Loudness::Small,
            "Spring games around the country.",
        ));
    }
}

fn spring_notices_hs(w: &mut World, d: Date) {
    let cands = hs_spring_candidates(w);
    notices(w, &cands, d, false);
}

fn spring_jumps_hs(w: &mut World, d: Date) {
    let cands = hs_spring_candidates(w);
    jumps(w, &cands, d, false);
}

// --- All-star games --------------------------------------------------------------------

/// The January high school all-star games: the All-American Bowl (San
/// Antonio, about 100 of the class's best) and the Polynesian Bowl
/// (Honolulu, about 100, Polynesian players first). A final stage before
/// signing day: the week's practices are scouted. Each game is played only
/// from its first real season (see `all_star_first_season`).
pub(crate) fn all_star_game(w: &mut World, name: &'static str) {
    if w.year < all_star_first_season(name) {
        return;
    }
    let d = w.today;
    let class = w.year + 1;
    let seed = w.options.seed as u64 ^ (class as u64) << 16 ^ name.len() as u64;
    let poly = name.starts_with("Polynesian");
    let mut pool: Vec<(f64, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.stage == Stage::HighSchool && p.class_of == class && p.stars >= 3 && p.cal.hs_all_star.is_none()
        })
        .map(|(i, p)| {
            let mut score = p.stars as f64 + 0.05 * p.player.overall() + 0.3 * key_normal(seed, p.player.id as u64);
            if poly && p.ethnicity == super::ethnicity::Ethnicity::Polynesian {
                score += 1.5;
            }
            (score, i)
        })
        .collect();
    pool.sort_by(|a, b| b.0.total_cmp(&a.0));
    let size = ((100.0 * w.options.scale).round() as usize).max(2);
    for &(_, i) in pool.iter().take(size) {
        let p = &mut w.people[i];
        p.cal.hs_all_star = Some(name);
        let age = p.player.age;
        // The age on his senior season's row (the game is in its offseason).
        p.note(format!("Played in the {name} at {}.", age - 1));
    }
    w.log.all_star_invites += pool.len().min(size) as u32;
    if w.recording {
        let text = format!("The {name} is played.");
        w.calendar.post(Event::new(d, Level::HighSchool, Kind::Game, Loudness::Medium, text));
    }
}

/// The first football season of a January high school all-star game (from
/// memory, to verify): the All-American Bowl's first game was Dec 30, 2000
/// (the 2000 season), the Polynesian Bowl's in January 2017 (the 2016
/// season).
pub(crate) fn all_star_first_season(name: &str) -> i32 {
    if name.starts_with("Polynesian") { 2016 } else { 2000 }
}

/// Each state's all-star game for its graduating seniors, late May.
fn state_all_stars(w: &mut World, d: Date) {
    let year = w.year;
    let mut states_played = std::collections::BTreeSet::new();
    for p in w.people.iter_mut() {
        if p.stage == Stage::HighSchool && p.class_of == year && p.cal.all_state >= 2 {
            states_played.insert(p.home_state);
            let age = row_age(p);
            p.note(format!("Played in his state's high school all-star game at {age}."));
        }
    }
    if w.recording && !states_played.is_empty() {
        let text = format!("{} states hold their high school all-star games.", states_played.len());
        w.calendar.post(Event::new(d, Level::HighSchool, Kind::Game, Loudness::Small, text));
    }
}

/// The year's high school dates in a state, for tests and the screens.
pub(crate) fn state_season_dates(code: &str, year: i32) -> (Date, Date, Date) {
    let c = hs_calendar(code);
    (
        shifted(c.start.0, c.start.1, year, labor_day),
        shifted(c.playoffs.0, c.playoffs.1, year, thanksgiving),
        shifted(c.finals.0, c.finals.1, year, thanksgiving),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldOptions;
    use std::collections::BTreeSet;

    #[test]
    fn high_school_regular_seasons_have_no_rematches() {
        let w = World::empty(WorldOptions { scale: 1.0, ..Default::default() });
        let hs = new_season(&w);
        let mut games = vec![0u32; w.high_schools.len()];
        for (s, st) in hs.states.iter().enumerate() {
            let mut met = BTreeSet::new();
            for k in 0..st.weeks.len() {
                let mut busy = BTreeSet::new();
                for g in hs.groups.iter().filter(|g| g.state == s) {
                    for &(h, a) in &g.schedule[k] {
                        assert!(met.insert((h.min(a), h.max(a))), "{} schools meet twice", w.states[s].code);
                        assert!(busy.insert(h) && busy.insert(a), "a school plays twice in one week");
                        games[h] += 1;
                        games[a] += 1;
                    }
                }
            }
        }
        // Byes come only from odd counts and the tiny states: under 1% of
        // school-weeks, and under 10% even in DC (25 schools in 4 classes).
        let byes = |state: Option<usize>| {
            let (mut slots, mut played) = (0, 0);
            for s in w.high_schools.iter().filter(|s| state.is_none_or(|x| s.state == x)) {
                slots += hs.states[s.state].weeks.len() as u32;
                played += games[s.id];
            }
            1.0 - played as f64 / slots as f64
        };
        assert!(byes(None) < 0.01, "bye share {:.3}", byes(None));
        let dc = w.states.iter().position(|s| s.code == "DC").unwrap();
        assert!(byes(Some(dc)) < 0.1, "DC bye share {:.3}", byes(Some(dc)));
        // The same seed gives the same schedule.
        let again = new_season(&w);
        assert!(hs.groups.iter().zip(&again.groups).all(|(a, b)| a.schedule == b.schedule));
        // Florida plays in its seven classifications.
        let fl = w.states.iter().position(|s| s.code == "FL").unwrap();
        assert_eq!(hs.groups.iter().filter(|g| g.state == fl).count(), 7);
    }

    #[test]
    fn january_all_star_games_start_in_their_real_seasons() {
        assert_eq!(all_star_first_season("All-American Bowl"), 2000);
        assert_eq!(all_star_first_season("Polynesian Bowl"), 2016);
    }
}
