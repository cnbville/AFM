//! Section 4, topic 2: the NFL year on its real dates. The offseason's dated
//! steps (the combine, the tag window, the new league year, the league
//! meeting, pro days, OTAs, camp, cutdowns), the regular season built by the
//! real opponent formula and played week by week, Black Monday, the playoffs
//! under each era's field and the Super Bowl. Around them: the training
//! blocks the CBA allows (topic 7), the contract and money dates other
//! sections will fill (A1-A8, A17), awards and Hall of Fame days (A15),
//! owner meetings (A44), betting releases (A45) and whisper windows (A46).
//!
//! NFL games are a stand-in until the league's rosters are full players
//! (section 5): each game is won from the two rosters' starters, the staffs
//! and home field, sized so a season's win shares spread like the real
//! league's (about 0.19, 0.12 of it luck).

use super::coach::TitleKind;
use super::news::{Channel, NewsItem, NewsKind};
use super::season::{key_normal, key_uniform};
use super::{World, recruiting, staff};
use crate::calendar::{Date, Event, Kind, Level, Loudness, Moment, Weekday, YearDates};

/// Win odds per standard deviation of roster strength (a logit), per point
/// of staff quality over 2.5, and home field (about 55% at home). Proposed,
/// not confirmed: they keep the old yearly stand-in's spread.
const PER_SD: f64 = 0.52;
const PER_STAFF: f64 = 0.24;
const HOME: f64 = 0.2;
/// About one game in 300 ends tied.
const TIE: f64 = 0.003;

#[derive(Clone, Debug, Default)]
pub(crate) struct NflSeason {
    pub year: i32,
    /// Conference (0 AFC, 1 NFC) and division (0-7, AFC East first) per team.
    pub conf: Vec<usize>,
    pub div: Vec<usize>,
    /// Each week's games, (home, away).
    pub weeks: Vec<Vec<(usize, usize)>>,
    pub wins: Vec<u32>,
    pub losses: Vec<u32>,
    pub ties: Vec<u32>,
    pub div_wins: Vec<u32>,
    /// Strength in standard deviations and staff quality, read at kickoff.
    pub z: Vec<f64>,
    pub staff: Vec<f64>,
    pub expect: Vec<f64>,
    pub started: bool,
    pub booked: bool,
    /// Playoff seeds per conference, best first, still alive.
    pub alive: [Vec<usize>; 2],
    /// Seed of each playoff team.
    pub seed: Vec<Option<usize>>,
    /// The round a playoff team went out in (1 wild card .. 4 Super Bowl), or
    /// 5 for the champion.
    pub out_round: Vec<u32>,
    pub champion: Option<usize>,
    pub runner_up: Option<usize>,
    /// This April's draft order (from last season), and the order this season sets.
    pub last_order: Option<Vec<usize>>,
    pub final_order: Option<Vec<usize>>,
    /// Each team's division finish (0 first) last season, for the formula.
    pub prev_place: Vec<usize>,
    /// Dated per-team days: pro days per program (A12), ticket renewals (A17).
    pub pro_days: Vec<(usize, Date)>,
    pub ticket_days: Vec<(usize, Date)>,
    /// Each team's bye Sunday.
    pub byes: Vec<Option<Date>>,
    /// Games judged huge by the stakes test, per team.
    pub huge: Vec<u32>,
}

/// Conference and division of each team, from the real alignment.
fn alignment(w: &World) -> (Vec<usize>, Vec<usize>) {
    let rows = &*super::data::NFL_TEAMS;
    let n = w.nfl.len();
    let mut conf = vec![0; n];
    let mut div = vec![0; n];
    for t in 0..n {
        let name = w.nfl[t].name;
        let row = rows.iter().find(|r| r.name == name).or_else(|| rows.get(t));
        let (c, d) = row.map_or((t / 16, (t / 4) % 4), |r| {
            let c = if r.conference == "AFC" { 0 } else { 1 };
            let d = match r.division.as_str() {
                "East" => 0,
                "North" => 1,
                "South" => 2,
                _ => 3,
            };
            (c, d)
        });
        conf[t] = c;
        div[t] = c * 4 + d;
    }
    (conf, div)
}

/// A new football year: last season's order and places carry over.
pub(crate) fn new_season(w: &World) -> NflSeason {
    let n = w.nfl.len();
    let (conf, div) = alignment(w);
    let (last_order, prev_place) = match w.nfl_season.as_ref() {
        Some(s) if s.booked => (s.final_order.clone(), places(s)),
        _ => (None, strength_places(w, &div)),
    };
    let year = w.year;
    let seed = w.options.seed as u64 ^ (year as u64) << 18 ^ 0x4E46;
    let y = &w.dates;
    // Pro days (A12): each FBS program and the FCS programs that produce
    // pros, on its own day in March and early April.
    let mut pro_days = Vec::new();
    let span = y.pro_days.0.days_to(y.pro_days.1).max(1);
    for p in w.programs.iter().filter(|p| p.division <= super::data::Division::FCS) {
        let k = (key_uniform(seed ^ 0x5044, p.id as u64) * (span + 1) as f64) as i32;
        let mut d = y.pro_days.0.add(k.min(span));
        if d.weekday() == Weekday::Sun {
            d = d.add(1);
        }
        pro_days.push((p.id, d));
    }
    let ticket_days = (0..n)
        .map(|t| {
            let k = (key_uniform(seed ^ 0x544B, t as u64) * 60.0) as i32;
            (t, Date::ymd(year, 3, 1).add(k))
        })
        .collect();
    NflSeason {
        year,
        conf,
        div,
        wins: vec![0; n],
        losses: vec![0; n],
        ties: vec![0; n],
        div_wins: vec![0; n],
        seed: vec![None; n],
        out_round: vec![0; n],
        last_order,
        prev_place,
        pro_days,
        ticket_days,
        byes: vec![None; n],
        huge: vec![0; n],
        ..Default::default()
    }
}

/// Division finishes from a finished season.
fn places(s: &NflSeason) -> Vec<usize> {
    let n = s.wins.len();
    let mut place = vec![0; n];
    for d in 0..8 {
        let mut members: Vec<usize> = (0..n).filter(|&t| s.div[t] == d).collect();
        members.sort_by(|&a, &b| record_key(s, b).total_cmp(&record_key(s, a)).then(a.cmp(&b)));
        for (k, &t) in members.iter().enumerate() {
            place[t] = k;
        }
    }
    place
}

/// Division finishes by roster strength, before the first season.
fn strength_places(w: &World, div: &[usize]) -> Vec<usize> {
    let n = w.nfl.len();
    let mut place = vec![0; n];
    for d in 0..8 {
        let mut members: Vec<usize> = (0..n).filter(|&t| div[t] == d).collect();
        members.sort_by(|&a, &b| w.nfl[b].strength().total_cmp(&w.nfl[a].strength()).then(a.cmp(&b)));
        for (k, &t) in members.iter().enumerate() {
            place[t] = k;
        }
    }
    place
}

fn record_key(s: &NflSeason, t: usize) -> f64 {
    let g = (s.wins[t] + s.losses[t] + s.ties[t]).max(1) as f64;
    (s.wins[t] as f64 + 0.5 * s.ties[t] as f64) / g + 0.001 * s.div_wins[t] as f64 + 0.0001 * s.z.get(t).copied().unwrap_or(0.0)
}

/// The season's games by the real opponent formula (2002 on): six division
/// games, four against a division of the same conference (a three-year
/// rotation), four against a division of the other conference (four
/// years), two against the same-place teams of the other two divisions in
/// the conference, and from 2021 a 17th game against a same-place team of
/// another division of the other conference. The 32-team alignment is used
/// for every year (proposed, not confirmed).
fn opponent_formula(s: &NflSeason, season: i32, games: u32) -> Vec<(usize, usize)> {
    let n = s.div.len();
    let members = |d: usize| -> Vec<usize> {
        let mut m: Vec<usize> = (0..n).filter(|&t| s.div[t] == d).collect();
        m.sort_by_key(|&t| (s.prev_place[t], t));
        m
    };
    let mut out = Vec::new();
    let par = season.rem_euclid(2) as usize;
    // Division: home and away.
    for d in 0..8 {
        let m = members(d);
        for i in 0..m.len() {
            for j in 0..m.len() {
                if i != j {
                    out.push((m[i], m[j]));
                }
            }
        }
    }
    // Four against four: home for half of them.
    let four = |a: usize, b: usize, out: &mut Vec<(usize, usize)>| {
        let (ma, mb) = (members(a), members(b));
        for (i, &x) in ma.iter().enumerate() {
            for (j, &y) in mb.iter().enumerate() {
                if (i + j + par) % 2 == 0 { out.push((x, y)) } else { out.push((y, x)) }
            }
        }
    };
    let rot3 = season.rem_euclid(3) as usize;
    let pairs3: [(usize, usize); 2] = match rot3 {
        0 => [(0, 1), (2, 3)],
        1 => [(0, 2), (1, 3)],
        _ => [(0, 3), (1, 2)],
    };
    for c in 0..2 {
        for &(a, b) in &pairs3 {
            four(c * 4 + a, c * 4 + b, &mut out);
        }
    }
    let rot4 = season.rem_euclid(4) as usize;
    for a in 0..4 {
        four(a, 4 + (a + rot4) % 4, &mut out);
    }
    // Same place, the two other divisions of the conference.
    for c in 0..2 {
        for a in 0..4 {
            let partner = pairs3.iter().find_map(|&(x, y)| if x == a { Some(y) } else if y == a { Some(x) } else { None });
            for b in 0..4 {
                if b == a || Some(b) == partner || b < a {
                    continue;
                }
                let (ma, mb) = (members(c * 4 + a), members(c * 4 + b));
                for p in 0..ma.len().min(mb.len()) {
                    if (p + par + a + b) % 2 == 0 { out.push((ma[p], mb[p])) } else { out.push((mb[p], ma[p])) }
                }
            }
        }
    }
    // The 17th game (2021 on): AFC hosts in odd years.
    if games >= 17 {
        for a in 0..4 {
            let b = 4 + (a + rot4 + 2) % 4;
            let (ma, mb) = (members(a), members(b));
            for p in 0..ma.len().min(mb.len()) {
                if par == 1 { out.push((ma[p], mb[p])) } else { out.push((mb[p], ma[p])) }
            }
        }
    }
    out
}

/// Spreads the games over the weeks, one game a team a week, with byes
/// between weeks 4 and 14 (a keyed search; any byes are allowed if it fails).
fn assign_weeks(games: &[(usize, usize)], n: usize, n_weeks: usize, seed: u64) -> Vec<Vec<(usize, usize)>> {
    for attempt in 0..400u64 {
        let strict = attempt < 300;
        let mut left: Vec<(usize, usize)> = games.to_vec();
        let mut weeks: Vec<Vec<(usize, usize)>> = Vec::with_capacity(n_weeks);
        let mut ok = true;
        for k in 0..n_weeks {
            let weeks_left = n_weeks - k;
            let mut remaining = vec![0usize; n];
            for &(a, b) in &left {
                remaining[a] += 1;
                remaining[b] += 1;
            }
            let bye_ok = !strict || (3..=13).contains(&k);
            let mut best: Option<Vec<(usize, usize)>> = None;
            for tries in 0..60u64 {
                let key = seed ^ attempt << 32 ^ (k as u64) << 16 ^ tries;
                let mut order: Vec<usize> = (0..left.len()).collect();
                // Teams that must play every remaining week go first.
                order.sort_by(|&x, &y| {
                    let ux = remaining[left[x].0].max(remaining[left[x].1]) as f64 + key_uniform(key, x as u64);
                    let uy = remaining[left[y].0].max(remaining[left[y].1]) as f64 + key_uniform(key, y as u64);
                    uy.total_cmp(&ux)
                });
                let mut used = vec![false; n];
                let mut pick = Vec::new();
                for &g in &order {
                    let (a, b) = left[g];
                    if !used[a] && !used[b] {
                        used[a] = true;
                        used[b] = true;
                        pick.push(g);
                    }
                }
                // Everyone with as many games as weeks left must play; outside the
                // bye weeks, everyone with games left plays.
                let fine = (0..n).all(|t| used[t] || remaining[t] == 0 || (remaining[t] < weeks_left && bye_ok));
                if fine {
                    best = Some(pick.iter().map(|&g| left[g]).collect());
                    break;
                }
            }
            let Some(week) = best else {
                ok = false;
                break;
            };
            left.retain(|g| !week.contains(g));
            weeks.push(week);
        }
        if ok && left.is_empty() {
            return weeks;
        }
    }
    // Fallback: greedy, any leftover games played in the last week.
    let mut left: Vec<(usize, usize)> = games.to_vec();
    let mut weeks = Vec::new();
    for _ in 0..n_weeks {
        let mut used = vec![false; n];
        let mut week = Vec::new();
        left.retain(|&(a, b)| {
            if !used[a] && !used[b] {
                used[a] = true;
                used[b] = true;
                week.push((a, b));
                false
            } else {
                true
            }
        });
        weeks.push(week);
    }
    if let Some(last) = weeks.last_mut() {
        last.extend(left);
    }
    weeks
}

/// The NFL day.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.nfl.is_empty() || w.nfl_season.is_none() {
        return;
    }
    let y = w.dates.clone();
    dated_events(w, d, &y);
    if d == y.combine_invites {
        recruiting::combine_invites(w);
    }
    if d == y.combine_end {
        recruiting::combine(w);
    }
    if d == y.annual_meeting.0 {
        league_meeting(w, d);
    }
    if d == y.schedule_release {
        build_schedule(w, d);
    }
    if d == y.nfl_kickoff.add(-1) {
        kickoff(w, d);
    }
    if let Some(k) = y.nfl_weeks.iter().position(|&s| s == d) {
        play_week(w, k, d);
    }
    if d == y.black_monday {
        black_monday(w, d);
    }
    if d == y.wild_card {
        play_round(w, 1, d);
    }
    if d == y.divisional {
        play_round(w, 2, d);
    }
    if d == y.conference_title_games {
        play_round(w, 3, d);
    }
    // NFL hires after the conference title games: weeks of interviews, and
    // a playoff team's coaches can't take a job until it is out (T9).
    if d == y.conference_title_games.add(1) {
        staff::carousel(w, staff::Scope::NflHires);
    }
    if d == y.super_bowl {
        play_round(w, 4, d);
    }
}

fn post(w: &mut World, d: Date, kind: Kind, loud: Loudness, rule: &'static str, text: impl Into<String>) {
    w.calendar.post(Event::new(d, Level::Nfl, kind, loud, text).rule(rule));
}

fn post_at(w: &mut World, m: Moment, kind: Kind, loud: Loudness, rule: &'static str, text: impl Into<String>) {
    w.calendar.post(Event::new(m.date, Level::Nfl, kind, loud, text).at(m).rule(rule));
}

/// The year's dated steps that are a date, a window or a slot other sections
/// fill (T2, T7, A1-A8, A11-A17, A44-A46).
fn dated_events(w: &mut World, d: Date, y: &YearDates) {
    let season = w.year;
    if d == y.start {
        post(w, d, Kind::Window, Loudness::Small, "CBA: offseason", "Players are on their own until the offseason program; rehab can happen at the facility.");
        post(w, d, Kind::Window, Loudness::Small, "CBA: reserve/future", "Reserve/future contracts can be signed.");
        let cap = w.rules.salary_cap;
        let text = match cap {
            Some(c) => format!("The {season} salary cap is set at ${:.1} million.", c / 1.0e6),
            None => format!("The {season} league year has no salary cap."),
        };
        post(w, d.add(10), Kind::Date, Loudness::Medium, "CBA: salary cap", text);
    }
    if d == y.tag_window_open {
        post(w, d, Kind::Window, Loudness::Medium, "CBA: tag window", "The franchise and transition tag window opens.");
    }
    if d == y.tag_deadline.date {
        post_at(w, y.tag_deadline, Kind::Deadline, Loudness::Big, "CBA: tag deadline", "Franchise tag deadline.");
    }
    if d == y.combine_start {
        post(w, d, Kind::Date, Loudness::Medium, "NFL: combine", "The scouting combine opens in Indianapolis.");
        // A11, A46: agents and teams meet; free agency whispers.
        post(w, d, Kind::Whisper, Loudness::Small, "", "Combine week: agents and teams meet in Indianapolis, and free agency whispers start.");
    }
    if d == y.tampering_open.date {
        post_at(w, y.tampering_open, Kind::Window, Loudness::Big, "CBA: negotiating window", "The two-day negotiating window opens: teams may talk to other teams' free agents.");
    }
    if d == y.league_year.date {
        let lock = w.rules.lockout_days;
        match lock {
            Some(days) => post(w, d, Kind::Date, Loudness::Huge, "CBA: lockout", format!("The league year is frozen by a lockout for {days} days.")),
            None => post_at(w, y.league_year, Kind::Date, Loudness::Big, "CBA: new league year", "The new league year opens at 4pm ET: free agency and trades begin."),
        }
    }
    if d == y.roster_bonus_window.0 {
        post(w, d, Kind::Slot, Loudness::Small, "contracts: roster bonuses", "Roster bonuses come due over the next days: cuts cluster before them (A1).");
    }
    if d == y.comp_picks {
        post(w, d, Kind::Date, Loudness::Small, "CBA: compensatory picks", "Compensatory picks are awarded.");
    }
    if d == y.pro_days.0 {
        post(w, d, Kind::Window, Loudness::Small, "NFL: pro days", "Pro day season opens.");
        post(w, d, Kind::Window, Loudness::Small, "NFL: top-30 visits", "Teams may host up to 30 prospects on official visits, through the deadline (A13).");
    }
    let todays: Vec<usize> = w.nfl_season.as_ref().unwrap().pro_days.iter().filter(|x| x.1 == d).map(|x| x.0).collect();
    for p in todays {
        let text = format!("{} pro day.", w.program_name(p));
        w.calendar.post(Event::new(d, Level::College, Kind::Date, Loudness::Small, text).program(p).rule("NFL: pro day"));
    }
    let tickets: Vec<usize> = w.nfl_season.as_ref().unwrap().ticket_days.iter().filter(|x| x.1 == d).map(|x| x.0).collect();
    for t in tickets {
        let text = format!("{} season ticket renewal deadline.", w.nfl[t].name);
        w.calendar.post(Event::new(d, Level::Nfl, Kind::Slot, Loudness::Small, text).team(t).rule("team: tickets (A17)"));
    }
    if d == Date::ymd(season, 4, 1) {
        post(w, d, Kind::Whisper, Loudness::Small, "", "Draft smokescreen season: teams leak false interest until the draft (A46).");
    }
    if d == y.top30_deadline {
        post(w, d, Kind::Deadline, Loudness::Small, "NFL: top-30 visits", "Deadline for top-30 visits.");
    }
    if d == y.offseason_new_coach {
        post(w, d, Kind::Window, Loudness::Small, "CBA: phase 1", "Teams with a new head coach may start their offseason program (phase 1, strength and conditioning) and add a voluntary minicamp.");
    }
    if d == y.offseason_returning {
        post(w, d, Kind::Window, Loudness::Small, "CBA: phase 1", "Offseason programs open: two weeks of strength and conditioning only (phase 1).");
    }
    if d == y.offseason_returning.add(14) {
        post(w, d, Kind::Window, Loudness::Small, "CBA: phase 2", "Phase 2: three weeks of individual drills, no offense against defense.");
    }
    if d == y.rfa_deadline {
        post(w, d, Kind::Deadline, Loudness::Small, "CBA: RFA offer sheets", "Restricted free agents' offer sheet deadline (A6).");
    }
    if d == y.draft[0].date.add(1) {
        // A placeholder for the UDFA rush's own hour.
    }
    if d == y.fifth_year_option {
        post(w, d, Kind::Deadline, Loudness::Small, "CBA: fifth-year option", "Fifth-year option deadline for first-round picks.");
    }
    if d == y.rookie_minicamp {
        post(w, d, Kind::Practice, Loudness::Small, "CBA: rookie minicamp", "Rookie minicamps.");
    }
    if d == y.schedule_release.add(1) {
        win_totals(w, d);
    }
    if d == y.otas.0 {
        post(w, d, Kind::Window, Loudness::Small, "CBA: phase 3 (OTAs)", "OTAs open (phase 3): 7-on-7 and 11-on-11 without live contact; workout bonuses count attendance (A3).");
    }
    if d == y.rookie_program {
        post(w, d, Kind::Window, Loudness::Small, "NFL: rookie program", "The league's rookie program takes rookies away from their teams for its days (A14).");
    }
    if d == y.june_1 {
        post(w, d, Kind::Date, Loudness::Small, "CBA: June 1", "June 1: a release now spreads dead money over two years (A2).");
    }
    if d == y.minicamp.0 {
        post(w, d, Kind::Practice, Loudness::Small, "CBA: mandatory minicamp", "Mandatory minicamps (three days).");
    }
    if d == y.ring_ceremony {
        let champ = w.nfl_season.as_ref().and_then(|_| w.prev_champion());
        if let Some(t) = champ {
            let text = format!("The {} receive their Super Bowl rings.", w.nfl[t].name);
            w.calendar.post(Event::new(d, Level::Nfl, Kind::Ceremony, Loudness::Small, text).team(t));
        }
    }
    if d == y.summer_break.0 {
        post(w, d, Kind::Window, Loudness::Small, "CBA: summer break", "Summer break: players train on their own.");
        post(w, d, Kind::Slot, Loudness::Small, "", "Offseason weddings, youth camps, charity events and quarterback-led throwing sessions fill the summer (A43).");
    }
    if d == y.tag_long_term_deadline.date {
        post_at(w, y.tag_long_term_deadline, Kind::Deadline, Loudness::Big, "CBA: tagged players", "Deadline for long-term deals with tagged players.");
    }
    if d == y.veterans_report.add(-3) {
        post(w, d, Kind::Slot, Loudness::Small, "owners (A44)", "Preseason owner meetings: each team's goals are set.");
    }
    if d == y.rookies_report {
        post(w, d, Kind::Practice, Loudness::Small, "CBA: training camp", "Rookies report to training camp.");
    }
    if d == y.veterans_report {
        post(w, d, Kind::Practice, Loudness::Medium, "CBA: training camp", "Veterans report: a player who stays away is holding out (fines of about $50,000 a day); one who reports but won't practice is holding in (A4, A5).");
    }
    if d == y.veterans_report.add(5) {
        post(w, d, Kind::Practice, Loudness::Small, "CBA: acclimation", "Five days without pads are done: pads go on.");
    }
    if d == y.hof_game {
        post(w, d, Kind::Game, Loudness::Medium, "", "The Hall of Fame Game opens the preseason.");
    }
    if d == y.hof_enshrinement {
        post(w, d, Kind::Ceremony, Loudness::Medium, "", "Hall of Fame enshrinement in Canton.");
    }
    for (k, p) in y.preseason.iter().enumerate() {
        if d == *p {
            post(w, d, Kind::Game, Loudness::Small, "", format!("Preseason week {}.", k + 1));
        }
    }
    if d == y.cutdown.date {
        post_at(w, y.cutdown, Kind::Deadline, Loudness::Huge, "CBA: roster cutdown", format!("Cutdown day: rosters go from 90 to {}.", w.rules.nfl_roster));
    }
    if d == y.waiver_claims.date {
        post_at(w, y.waiver_claims, Kind::Deadline, Loudness::Big, "CBA: waivers", "Waiver claims on cut players are awarded.");
    }
    if d == y.practice_squads {
        post(w, d, Kind::Date, Loudness::Medium, "CBA: practice squads", format!("Practice squads are signed ({} spots).", w.rules.nfl_practice_squad));
    }
    if d == y.trade_deadline.date.add(-14) {
        post(w, d, Kind::Whisper, Loudness::Small, "", "Trade rumors pick up ahead of the deadline (A46).");
    }
    if d == y.trade_deadline.date {
        post_at(w, y.trade_deadline, Kind::Deadline, Loudness::Huge, "CBA: trade deadline", "The trade deadline, 4pm ET.");
    }
    if d == y.pro_bowl_voting.0 {
        post(w, d, Kind::Window, Loudness::Small, "", "Pro Bowl voting opens.");
    }
    if let Some(&last) = y.nfl_weeks.last()
        && d == last
    {
        post(w, d, Kind::Deadline, Loudness::Small, "contracts: incentives", "The last regular-season games: every incentive is judged on the season's final numbers (A8).");
        post(w, d, Kind::Slot, Loudness::Small, "owners (A44)", "End-of-season reviews with the owners, the day before Black Monday.");
    }
    if d == y.all_pro {
        post(w, d, Kind::Award, Loudness::Medium, "", "The All-Pro teams are named.");
    }
    if d == y.hof_finalists {
        post(w, d, Kind::Award, Loudness::Small, "", "Hall of Fame finalists are named.");
    }
    if d == y.pro_bowl_games {
        post(w, d, Kind::Game, Loudness::Small, "", "The Pro Bowl Games.");
    }
    if d == y.nfl_honors {
        post(w, d, Kind::Award, Loudness::Big, "", "NFL Honors: MVP and the season's awards; the Hall of Fame class is announced.");
    }
    let byes: Vec<usize> = (0..w.nfl.len())
        .filter(|&t| w.nfl_season.as_ref().unwrap().byes[t].is_some_and(|b| b.add(-5) == d))
        .collect();
    if !byes.is_empty() {
        let names: Vec<&str> = byes.iter().map(|&t| w.nfl[t].name).collect();
        let text = format!("Bye week (four days off by the CBA, self-scouting, owner check-ins): {}.", names.join(", "));
        post(w, d, Kind::Slot, Loudness::Small, "CBA: bye week", text);
    }
}

/// The league meeting in late March (topic 11): from 2027 the agreed NFL
/// changes are proposed and voted on; a relocation request is heard.
fn league_meeting(w: &mut World, d: Date) {
    let season = w.year;
    post(w, d, Kind::Date, Loudness::Medium, "NFL: annual meeting", "The league's annual meeting: rules proposals are voted on.");
    let mut rng = w.rng.fork();
    for text in w.changes.league_meeting(season, &mut rng) {
        w.log.rule_changes.push(text.clone());
        post(w, d, Kind::RuleChange, Loudness::Big, "owners' vote", text);
    }
    if season >= 2027 && rng.chance(crate::calendar::rules::CHANGE_ODDS.relocation_request) {
        relocate(w, d, &mut rng);
    }
}

/// A team asks to move (topic 11, change 5): the smallest market with the
/// least patient owner asks; 24 of 32 owners must agree. Proposed, not
/// confirmed: the cities without a team that real bids have named.
fn relocate(w: &mut World, d: Date, rng: &mut crate::rng::Rng) {
    const CITIES: [&str; 6] = ["San Antonio", "St. Louis", "San Diego", "Portland", "Salt Lake City", "Oakland"];
    let rows = &*super::data::NFL_TEAMS;
    let t = (0..w.nfl.len())
        .max_by(|&a, &b| {
            let ka = rows.iter().find(|r| r.name == w.nfl[a].name).map_or(16, |r| r.market_rank) as f64 - w.nfl[a].owner;
            let kb = rows.iter().find(|r| r.name == w.nfl[b].name).map_or(16, |r| r.market_rank) as f64 - w.nfl[b].owner;
            ka.total_cmp(&kb)
        })
        .unwrap_or(0);
    let yes = (0..32).filter(|_| rng.chance(0.7)).count() as u32;
    let old = w.nfl[t].name;
    let city = CITIES[rng.int(0, CITIES.len() as i32 - 1) as usize];
    if yes >= 24 {
        let nickname = old.rsplit(' ').next().unwrap_or("Team");
        let name: &'static str = Box::leak(format!("{city} {nickname}").into_boxed_str());
        w.nfl[t].name = name;
        let text = format!("Owners approve the {old}' move to {city}, {yes}-{}: they play next season as the {name}.", 32 - yes);
        w.changes.applied.push(crate::calendar::rules::AppliedChange {
            change: crate::calendar::Change::Relocation,
            season: w.year,
            detail: format!("{old} to {city}"),
        });
        w.log.rule_changes.push(text.clone());
        post(w, d, Kind::RuleChange, Loudness::Huge, "owners' vote", text);
    } else {
        let text = format!("The {old}' request to move to {city} fails, {yes} votes of the 24 needed.");
        post(w, d, Kind::RuleChange, Loudness::Medium, "owners' vote", text);
    }
}

/// Schedule release in May: the season's games by the formula.
fn build_schedule(w: &mut World, d: Date) {
    let season = w.year;
    let games = w.rules.nfl_games;
    let n_weeks = w.dates.nfl_weeks.len();
    let n = w.nfl.len();
    let seed = w.options.seed as u64 ^ (season as u64) << 22 ^ 0x5343;
    let s = w.nfl_season.as_mut().unwrap();
    s.year = season;
    let list = opponent_formula(s, season, games);
    s.weeks = assign_weeks(&list, n, n_weeks, seed);
    let sundays = w.dates.nfl_weeks.clone();
    for (k, week) in s.weeks.iter().enumerate() {
        let mut playing = vec![false; n];
        for &(a, b) in week {
            playing[a] = true;
            playing[b] = true;
        }
        for t in 0..n {
            if !playing[t] && s.byes[t].is_none() {
                s.byes[t] = sundays.get(k).copied();
            }
        }
    }
    post(w, d, Kind::Date, Loudness::Medium, "NFL: schedule release", format!("The {season} schedule is released: {games} games over {n_weeks} weeks."));
}

/// Win totals after the schedule release (A45): the market's read of each
/// roster.
fn win_totals(w: &mut World, d: Date) {
    let games = w.rules.nfl_games as f64;
    let s: Vec<f64> = w.nfl.iter().map(|t| t.strength()).collect();
    let mean = s.iter().sum::<f64>() / s.len().max(1) as f64;
    let sd = (s.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / s.len().max(1) as f64).sqrt().max(0.5);
    // A line sits on the half win nearest what the roster should win.
    let mut v: Vec<(f64, usize)> =
        s.iter().enumerate().map(|(t, x)| (((0.5 + 0.13 * (x - mean) / sd) * games).floor() + 0.5, t)).collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    let line = |x: &(f64, usize)| format!("{} {:.1}", w.nfl[x.1].name, x.0);
    let top: Vec<String> = v.iter().take(3).map(line).collect();
    let bottom: Vec<String> = v.iter().rev().take(3).map(line).collect();
    let text = format!("Preseason win totals are posted. Highest: {}. Lowest: {}.", top.join(", "), bottom.join(", "));
    post(w, d, Kind::News, Loudness::Small, "betting (A45)", text);
}

/// The eve of the season: each roster and staff is read.
fn kickoff(w: &mut World, d: Date) {
    let n = w.nfl.len();
    if w.nfl_season.as_ref().unwrap().weeks.is_empty() {
        build_schedule(w, d);
    }
    let strength: Vec<f64> = w.nfl.iter().map(|t| t.strength()).collect();
    let mean = strength.iter().sum::<f64>() / n as f64;
    let sd = (strength.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n as f64).sqrt().max(0.5);
    let z: Vec<f64> = strength.iter().map(|s| (s - mean) / sd).collect();
    let staff: Vec<f64> = w.nfl.iter().map(|t| (t.coaching - 13.0) / 2.5).collect();
    let s = w.nfl_season.as_mut().unwrap();
    s.z = z;
    s.staff = staff;
    s.expect = vec![0.0; n];
    s.started = true;
    post(w, d, Kind::Date, Loudness::Big, "", "The NFL season kicks off.");
}

fn win_prob(s: &NflSeason, home: usize, away: usize, field: f64) -> f64 {
    let x = PER_SD * (s.z[home] - s.z[away]) + PER_STAFF * (s.staff[home] - s.staff[away]) + field;
    1.0 / (1.0 + (-x).exp())
}

/// One Sunday: every game of the week.
fn play_week(w: &mut World, k: usize, d: Date) {
    let Some(s) = w.nfl_season.as_ref() else { return };
    if !s.started || k >= s.weeks.len() {
        return;
    }
    stakes(w, k, d);
    let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (k as u64) << 10 ^ 0x4E47;
    let s = w.nfl_season.as_mut().unwrap();
    let games = s.weeks[k].clone();
    for &(h, a) in &games {
        let key = (h as u64) << 8 ^ a as u64;
        let p = win_prob(s, h, a, HOME);
        let pe = 1.0 / (1.0 + (-(PER_SD * (s.z[h] - s.z[a]) + HOME)).exp());
        s.expect[h] += pe;
        s.expect[a] += 1.0 - pe;
        let u = key_uniform(seed, key);
        let same_div = s.div[h] == s.div[a];
        if key_uniform(seed ^ 0x7E, key) < TIE {
            s.ties[h] += 1;
            s.ties[a] += 1;
        } else {
            let (wn, l) = if u < p { (h, a) } else { (a, h) };
            s.wins[wn] += 1;
            s.losses[l] += 1;
            if same_div {
                s.div_wins[wn] += 1;
            }
        }
    }
    w.log.nfl_games += games.len() as u32;
    // A45: next week's lines open after the games.
    if k + 1 < w.dates.nfl_weeks.len() {
        post(w, d, Kind::News, Loudness::Small, "betting (A45)", format!("Week {} lines open.", k + 2));
    }
}

/// The stakes test for NFL games (topic 8): late in the year a game between
/// two teams in the playoff race, or for a division, is huge; a head coach
/// on the hot seat in such a game adds to it.
fn stakes(w: &mut World, k: usize, d: Date) {
    let s = w.nfl_season.as_ref().unwrap();
    let n_weeks = s.weeks.len();
    if k + 4 < n_weeks {
        return;
    }
    let per_conf = (w.rules.nfl_playoff_teams / 2) as usize;
    let mut cut = [0.0f64; 2];
    for (c, cut_c) in cut.iter_mut().enumerate() {
        let mut v: Vec<f64> = (0..s.wins.len()).filter(|&t| s.conf[t] == c).map(|t| s.wins[t] as f64 + 0.5 * s.ties[t] as f64).collect();
        v.sort_by(|a, b| b.total_cmp(a));
        *cut_c = v.get(per_conf.saturating_sub(1)).copied().unwrap_or(0.0);
    }
    let wins = |t: usize| s.wins[t] as f64 + 0.5 * s.ties[t] as f64;
    let in_race = |t: usize| (wins(t) - cut[s.conf[t]]).abs() <= 1.0;
    let lead = |t: usize| {
        (0..s.wins.len()).filter(|&o| s.div[o] == s.div[t]).map(|o| wins(o) as i32).max().unwrap_or(0) as f64
    };
    let mut huge: Vec<(usize, usize)> = Vec::new();
    for &(h, a) in &s.weeks[k] {
        let race = in_race(h) && in_race(a);
        let division = s.div[h] == s.div[a] && wins(h) >= lead(h) - 1.0 && wins(a) >= lead(a) - 1.0 && wins(h) >= cut[s.conf[h]] - 1.0;
        if race || division {
            huge.push((h, a));
        }
    }
    for (h, a) in huge {
        let s = w.nfl_season.as_mut().unwrap();
        s.huge[h] += 1;
        s.huge[a] += 1;
        let text = format!("A game with the season on it: the {} host the {}.", w.nfl[h].name, w.nfl[a].name);
        w.calendar.post(Event::new(d, Level::Nfl, Kind::Game, Loudness::Huge, text).team(h).at(Moment::at(d, 13, 0)));
    }
}

/// Black Monday: the regular season's book, the playoff field, and the
/// firings (topic 2). Hires wait until after the conference title games.
fn black_monday(w: &mut World, d: Date) {
    let year = w.year;
    let n = w.nfl.len();
    let Some(s) = w.nfl_season.as_ref() else { return };
    if !s.started || s.booked {
        return;
    }
    let s = s.clone();
    for t in 0..n {
        let games = (s.wins[t] + s.losses[t] + s.ties[t]).max(1);
        let share = (s.wins[t] as f64 + 0.5 * s.ties[t] as f64) / games as f64;
        let vs = share - s.expect[t] / games as f64;
        let team = &mut w.nfl[t];
        team.win_share.push(share);
        if team.win_share.len() > 3 {
            team.win_share.remove(0);
        }
        let (hc, oc, dc) = (team.head_coach, team.oc, team.dc);
        if let Some(h) = hc {
            let coach = &mut w.coaches[h];
            coach.wins += s.wins[t];
            coach.losses += s.losses[t];
            coach.season_reputation(vs, 0.0, true);
        }
        // A coordinator is judged on his unit: the team's year and his side's own form.
        for (c, off) in [(oc, true), (dc, false)] {
            let Some(c) = c else { continue };
            let unit = 0.03 * (staff::side_quality(&w.coaches[c], off) - 13.5) / 2.0
                + 0.04 * key_normal(w.options.seed as u64 ^ year as u64 ^ 0x554E, c as u64);
            w.coaches[c].season_reputation(0.5 * vs + unit, 0.0, false);
        }
    }
    // The playoff field: division winners first, then wild cards, by record.
    let per_conf = (w.rules.nfl_playoff_teams / 2) as usize;
    let mut alive: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    let mut seed = vec![None; n];
    for c in 0..2 {
        let mut winners: Vec<usize> = (0..4)
            .filter_map(|dv| {
                (0..n).filter(|&t| s.div[t] == c * 4 + dv).max_by(|&a, &b| record_key(&s, a).total_cmp(&record_key(&s, b)))
            })
            .collect();
        winners.sort_by(|&a, &b| record_key(&s, b).total_cmp(&record_key(&s, a)));
        let mut rest: Vec<usize> = (0..n).filter(|&t| s.conf[t] == c && !winners.contains(&t)).collect();
        rest.sort_by(|&a, &b| record_key(&s, b).total_cmp(&record_key(&s, a)));
        let field: Vec<usize> = winners.into_iter().chain(rest).take(per_conf).collect();
        for (k, &t) in field.iter().enumerate() {
            seed[t] = Some(k + 1);
        }
        alive[c] = field;
    }
    let s = w.nfl_season.as_mut().unwrap();
    s.alive = alive;
    s.seed = seed;
    s.booked = true;
    w.log.nfl_huge_by_team = s.huge.clone();
    post(w, d, Kind::Coaching, Loudness::Big, "", "Black Monday: the day after the regular season, when head coaches are fired.");
    staff::carousel(w, staff::Scope::NflFirings);
}

/// A playoff round: 1 wild card, 2 divisional, 3 conference title games, 4
/// the Super Bowl. The higher seed hosts; the Super Bowl is neutral.
fn play_round(w: &mut World, round: u32, d: Date) {
    let Some(s) = w.nfl_season.as_ref() else { return };
    if !s.booked {
        return;
    }
    let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (round as u64) << 4 ^ 0x504F;
    let mut s = s.clone();
    if round == 4 {
        let (a, b) = (s.alive[0].first().copied(), s.alive[1].first().copied());
        let (Some(a), Some(b)) = (a, b) else { return };
        let p = win_prob(&s, a, b, 0.0);
        let a_wins = key_uniform(seed, (a as u64) << 8 ^ b as u64) < p;
        let (champ, loser) = if a_wins { (a, b) } else { (b, a) };
        s.out_round[champ] = 5;
        s.out_round[loser] = 4;
        s.champion = Some(champ);
        s.runner_up = Some(loser);
        s.final_order = Some(order_from(&s));
        *w.nfl_season.as_mut().unwrap() = s;
        super_bowl_book(w, champ, loser, d);
        return;
    }
    let mut games = 0;
    for c in 0..2 {
        let field = s.alive[c].clone();
        let n = field.len();
        // Byes for the top seeds when the field isn't a power of two (wild card round only).
        let byes = if round == 1 { n.next_power_of_two() - n } else { 0 };
        let byes = if round == 1 && n == 8 { 0 } else { byes.min(n) };
        let (stay, play) = field.split_at(byes);
        let mut next: Vec<usize> = stay.to_vec();
        let m = play.len();
        for k in 0..m / 2 {
            let (hi, lo) = (play[k], play[m - 1 - k]);
            let p = win_prob(&s, hi, lo, HOME);
            let hi_wins = key_uniform(seed ^ c as u64, (hi as u64) << 8 ^ lo as u64) < p;
            let (wn, l) = if hi_wins { (hi, lo) } else { (lo, hi) };
            s.out_round[l] = round;
            next.push(wn);
            games += 1;
        }
        // Reseed: the best remaining seed plays the lowest.
        next.sort_by_key(|&t| s.seed[t].unwrap_or(99));
        s.alive[c] = next;
    }
    w.log.nfl_games += games;
    let names = ["", "Wild card weekend", "The divisional round", "The conference title games"];
    let text = names[round as usize].to_string() + ".";
    *w.nfl_season.as_mut().unwrap() = s;
    post(w, d, Kind::Game, Loudness::Huge, "", text);
}

/// The draft order the season sets: teams out of the playoffs by record,
/// worst first, then playoff teams by the round they went out in, the Super
/// Bowl loser 31st and the champion 32nd.
fn order_from(s: &NflSeason) -> Vec<usize> {
    let n = s.wins.len();
    let mut v: Vec<usize> = (0..n).collect();
    v.sort_by(|&a, &b| {
        let ra = (s.out_round[a], record_key(s, a));
        let rb = (s.out_round[b], record_key(s, b));
        ra.0.cmp(&rb.0).then(ra.1.total_cmp(&rb.1))
    });
    v
}

/// The Super Bowl's book: titles, reputations and the news.
fn super_bowl_book(w: &mut World, champ: usize, loser: usize, d: Date) {
    let year = w.year;
    for (t, title) in [(champ, TitleKind::National), (loser, TitleKind::Conference)] {
        let (hc, oc, dc) = (w.nfl[t].head_coach, w.nfl[t].oc, w.nfl[t].dc);
        if let Some(h) = hc {
            let coach = &mut w.coaches[h];
            coach.titles += 1;
            coach.career_titles += 1;
            coach.title_reputation(title, true, 1.0);
        }
        for c in [oc, dc].into_iter().flatten() {
            w.coaches[c].title_reputation(title, false, 1.0);
        }
    }
    w.nfl_champions.push((year, champ));
    let name = w.nfl[champ].name;
    let text = format!("The {name} win the Super Bowl over the {}.", w.nfl[loser].name);
    w.calendar.post(Event::new(d, Level::Nfl, Kind::Game, Loudness::Huge, text).team(champ));
    if w.recording {
        let coach = w.nfl[champ].head_coach.map_or(String::new(), |h| format!(" under head coach {}", w.coaches[h].name));
        w.news.push(NewsItem {
            date: Some(d),
            year,
            kind: NewsKind::NationalTitle,
            channel: Channel::News,
            program: None,
            text: format!("The {name} win the Super Bowl{coach}."),
        });
    }
}

/// The draft order for this April, from last season (T2); None before the
/// first season has been played on the clock.
pub(crate) fn draft_order(w: &World) -> Option<Vec<usize>> {
    w.nfl_season.as_ref().and_then(|s| s.last_order.clone())
}

impl World {
    /// Last season's Super Bowl champion, if the clock has played one.
    pub(crate) fn prev_champion(&self) -> Option<usize> {
        self.nfl_champions.last().filter(|c| c.0 == self.year - 1).map(|c| c.1)
    }
}

/// The training blocks of the year at every level (topic 7): when each kind
/// of training can happen. How training works is section 6b.
pub(crate) fn training_blocks(w: &mut World, d: Date, y: &YearDates) {
    let blocks: [(Date, Level, &str, &str); 9] = [
        (y.spring_semester, Level::College, "NCAA: winter conditioning", "College winter conditioning: strength and speed work, no football."),
        (y.spring_practice.0, Level::College, "NCAA: 15 spring practices", "College spring practice: 15 sessions over about five weeks."),
        (y.summer_workouts.0, Level::College, "NCAA: 8 hours a week", "College summer: 8 hours a week of required workouts, with film."),
        (y.fall_camp, Level::College, "NCAA: 29 camp practices", "College fall camp: 29 practices before the opener."),
        (Date::ymd(y.season, 1, 15).max(y.start), Level::HighSchool, "", "High school offseason lifting and track."),
        (Date::ymd(y.season, 5, 1), Level::HighSchool, "", "High school spring ball, in the states that allow it."),
        (Date::ymd(y.season, 6, 15), Level::HighSchool, "", "High school summer: 7-on-7 tournaments, camps, and each state's dead period."),
        (Date::ymd(y.season, 7, 27), Level::HighSchool, "", "High school practice opens in the earliest states."),
        (y.cfb_weeks[0].add(-5), Level::College, "NCAA: 20-hour rule", "College in-season: 20 countable hours a week, one day off."),
    ];
    for (date, level, rule, text) in blocks {
        if d == date {
            w.calendar.post(Event::new(d, level, Kind::Window, Loudness::Small, text).rule(rule));
        }
    }
    if d == y.nfl_kickoff {
        post(w, d, Kind::Window, Loudness::Small, "CBA: in-season practice", "NFL season: practice Wednesday to Friday, 14 padded practices in all.");
    }
}

/// A one-line playoff summary for trials: (seed, team) per conference.
pub(crate) fn playoff_field(s: &NflSeason) -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = s.seed.iter().enumerate().filter_map(|(t, k)| k.map(|k| (k, t))).collect();
    v.sort();
    v
}
