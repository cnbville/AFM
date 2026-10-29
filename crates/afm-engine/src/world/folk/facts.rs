//! The world's facts about a team, read the way a new coach hears them
//! (takeover items 12, 18, 20 and 21): its last seasons, the man you
//! replaced and why he left, the rival and your record against him, the
//! money and the buildings in words, the schedule already set and the
//! dates that come due. Nothing here is stored: each fact is read from the
//! world's own history when a scene asks.

use super::Team;
use crate::calendar::{Date, Level};
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;

/// One season of a team, in its own terms.
#[derive(Clone, Debug)]
pub struct Season {
    pub year: i32,
    pub wins: u32,
    pub losses: u32,
    pub coach: Option<usize>,
    /// "won the conference", "lost in the wild card round", "state champions".
    pub note: String,
}

/// The team's last `n` finished seasons, newest last.
pub fn seasons(w: &World, team: Team, n: usize) -> Vec<Season> {
    let mut out: Vec<Season> = match team {
        Team::College(p) => w.programs[p]
            .seasons
            .iter()
            .map(|s| Season {
                year: s.year,
                wins: s.wins,
                losses: s.losses,
                coach: s.coach,
                note: if s.national_title {
                    "national champions".into()
                } else if s.conference_title {
                    "conference champions".into()
                } else {
                    String::new()
                },
            })
            .collect(),
        Team::High(h) => w.high_schools[h]
            .seasons
            .iter()
            .map(|s| Season {
                year: s.year,
                wins: s.wins,
                losses: s.losses,
                coach: s.coach,
                note: if s.state_title { "state champions".into() } else { String::new() },
            })
            .collect(),
        Team::Nfl(t) => w
            .nfl_seasons
            .iter()
            .filter(|s| s.team == t)
            .map(|s| Season {
                year: s.year,
                wins: s.wins,
                losses: s.losses,
                coach: s.coach,
                note: match s.round {
                    5 => "won the Super Bowl".into(),
                    4 => "lost the Super Bowl".into(),
                    3 => "lost the conference title game".into(),
                    2 => "lost in the divisional round".into(),
                    1 => "lost in the wild card round".into(),
                    _ => String::new(),
                },
            })
            .collect(),
        Team::Other(f) => crate::world::pro::other::team_seasons(w, f)
            .into_iter()
            .map(|(year, wins, losses, note)| Season { year, wins, losses, coach: None, note })
            .collect(),
    };
    out.sort_by_key(|s| s.year);
    let cut = out.len().saturating_sub(n);
    out.drain(..cut);
    out
}

/// "8-5 in 2025, conference champions"
pub fn season_words(s: &Season) -> String {
    if s.note.is_empty() {
        format!("{}-{} in {}", s.wins, s.losses, s.year)
    } else {
        format!("{}-{} in {}, {}", s.wins, s.losses, s.year, s.note)
    }
}

/// The man you replaced: the last head coach whose stint here ended, and
/// why, in his record's own words.
pub fn predecessor(w: &World, team: Team, you: usize) -> Option<(usize, String, i32, i32)> {
    let emp = team.employer();
    let mut best: Option<(usize, String, i32, i32)> = None;
    for c in &w.coaches {
        if c.id == you {
            continue;
        }
        for s in &c.history {
            if s.employer == emp && s.role == Role::HeadCoach && best.as_ref().is_none_or(|b| s.to > b.3) {
                best = Some((c.id, s.ended.to_string(), s.from, s.to));
            }
        }
    }
    best
}

/// Why he left, the way people in the building put it.
pub fn why_left(why: &str) -> &'static str {
    match why {
        "fired" => "he was fired",
        "retired" => "he retired",
        w if w.contains("burned out") => "he stepped away, worn out",
        w if w.contains("health") => "he stepped away for his health",
        w if w.contains("resigned") => "he resigned",
        w if w.contains("not renewed") => "the school didn't renew his stipend",
        w if w.contains("left") || w.contains("took") || w.contains("hired") => "he left for another job",
        _ => "he moved on",
    }
}

/// The team's main rival: (rival, the game's name, our wins, their wins).
pub fn rival(w: &World, team: Team) -> Option<(Team, &'static str, u32, u32)> {
    match team {
        Team::College(p) => {
            let cs = w.college.as_ref().or(w.college_prev.as_ref())?;
            let mut found: Option<(usize, usize, usize)> = None;
            for (&(a, b), &(r, first)) in &cs.rivalry_of {
                if a == p || b == p {
                    let other = if a == p { b } else { a };
                    // A trophy or a rivalry-week slot outranks the rest.
                    let rank = crate::calendar::data::RIVALRIES[r].trophy.is_some() as usize;
                    if found.is_none_or(|f| rank > f.2 || (rank == f.2 && r < f.0)) {
                        found = Some((r, other, rank));
                    }
                    let _ = first;
                }
            }
            let (r, other, _) = found?;
            let first = cs.rivalry_of.iter().find(|x| x.1.0 == r).map(|x| x.1.1)?;
            let s = cs.series.get(r)?;
            let (ours, theirs) = if first == p { (s.wins[0], s.wins[1]) } else { (s.wins[1], s.wins[0]) };
            Some((Team::College(other), crate::calendar::data::RIVALRIES[r].name_in(w.year), ours, theirs))
        }
        Team::Nfl(t) => {
            // The division's oldest rivalry stands in: the team in its
            // division it has played longest (the first listed).
            let mine = crate::world::franchise::era_in(w, t, w.year)?;
            let other = (0..w.nfl.len())
                .filter(|&o| o != t)
                .find(|&o| crate::world::franchise::era_in(w, o, w.year).is_some_and(|e| e.div == mine.div))?;
            Some((Team::Nfl(other), "the division rivalry", 0, 0))
        }
        Team::High(h) => {
            // The nearest school in the same classification and town or area.
            let s = &w.high_schools[h];
            let other = w.states[s.state]
                .schools
                .iter()
                .copied()
                .filter(|&o| o != h)
                .filter(|&o| w.high_schools[o].area == s.area)
                .max_by(|&a, &b| {
                    let fa = w.high_schools[a].prestige
                        - 0.2 * (w.high_schools[a].classification as f64 - s.classification as f64).abs();
                    let fb = w.high_schools[b].prestige
                        - 0.2 * (w.high_schools[b].classification as f64 - s.classification as f64).abs();
                    fa.total_cmp(&fb)
                })?;
            Some((Team::High(other), "the crosstown game", 0, 0))
        }
        Team::Other(_) => None,
    }
}

/// How big the job is, 0-1: what a coach's reputation is read against.
pub fn job_size(w: &World, team: Team) -> f64 {
    match team {
        Team::College(p) => {
            let prog = &w.programs[p];
            let lvl = match prog.division {
                Division::FBS if prog.power_four() => 0.55,
                Division::FBS => 0.35,
                Division::FCS => 0.2,
                _ => 0.08,
            };
            (lvl + 0.45 * prog.prestige).min(1.0)
        }
        Team::Nfl(_) => 0.9,
        Team::Other(f) => 0.6 * crate::world::pro::other::prestige(f),
        Team::High(h) => 0.05 + 0.2 * w.high_schools[h].prestige,
    }
}

/// Your standing (1-20, C10) when you have a career: what owners, general
/// managers, big programs' athletic directors and other coaches think of
/// your coaching.
pub fn your_rep(w: &World) -> f64 {
    w.career.as_ref().and_then(|c| w.coaches.get(c.coach)).map_or(10.0, |c| c.reputation)
}

/// Your name (1-20, C10): what fans, boosters, recruits and small schools
/// know of you.
pub fn your_name(w: &World) -> f64 {
    w.career.as_ref().and_then(|c| w.coaches.get(c.coach)).map_or(8.0, |c| c.name_value())
}

/// How a job's people read you (C10): a small school's by your name, a big
/// one's and the NFL's by your standing (`staff::name_weight`).
pub fn read_by(w: &World, team: Team) -> f64 {
    let k = crate::world::staff::name_weight(w, team.employer(), Role::HeadCoach);
    (1.0 - k) * your_rep(w) + k * your_name(w)
}

/// How you read against the job, about -1 (a reach) to 1 (a coup).
pub fn fit(w: &World, team: Team) -> f64 {
    let rep = read_by(w, team) / 20.0;
    let need = 0.2 + 0.75 * job_size(w, team);
    ((rep - need) * 2.5).clamp(-1.0, 1.0)
}

/// A hire grade the way the national writers hand them out.
pub fn hire_grade(f: f64) -> &'static str {
    match f {
        f if f > 0.6 => "A",
        f if f > 0.35 => "A-",
        f if f > 0.15 => "B+",
        f if f > -0.05 => "B",
        f if f > -0.25 => "B-",
        f if f > -0.45 => "C+",
        f if f > -0.65 => "C",
        _ => "C-",
    }
}

/// The money in words, by level.
pub fn money_words(w: &World, team: Team) -> Vec<(String, String)> {
    let m = |v: f64| {
        if v >= 1e6 { format!("${:.1} million", v / 1e6) } else { format!("${}k", (v / 1e3).round()) }
    };
    match team {
        Team::College(p) => {
            let prog = &w.programs[p];
            let mut v = Vec::new();
            let budget = crate::world::staff::program_budget(prog);
            v.push(("Athletic budget".into(), format!("about {} a year for every sport", m(budget))));
            if w.year >= 2021 {
                v.push((
                    "NIL collective".into(),
                    if prog.collective.budget > 0.0 {
                        format!("{} to spend on players this offseason", m(prog.collective.budget))
                    } else {
                        "next to nothing this offseason".into()
                    },
                ));
            }
            if w.year >= 2025 && prog.division == Division::FBS && prog.power_four() {
                v.push((
                    "Revenue sharing".into(),
                    "about $20.5 million a year paid to players directly (House settlement)".into(),
                ));
            }
            let b = prog.collective.boosters();
            v.push((
                "Boosters".into(),
                match prog.collective.big_booster {
                    Some(big) => format!("{} a year from the booster base, {} of it from one man", m(b), m(big)),
                    None => format!("{} a year from the booster base, spread wide", m(b)),
                },
            ));
            v
        }
        Team::Nfl(t) => {
            let mut v = Vec::new();
            if let Some(c) = w.clubs.get(t) {
                let o = &c.office.owner;
                v.push((
                    "Owner's money".into(),
                    match o.cash_will {
                        x if x > 0.66 => "spends to the cap and pays guarantees without blinking".into(),
                        x if x > 0.33 => "spends when the football people make the case".into(),
                        _ => "watches cash closely: guarantees are a fight".into(),
                    },
                ));
                let room = crate::world::pro::space(w, t, w.year);
                v.push(("Cap room".into(), format!("about {} for the new league year", m(room))));
            }
            v
        }
        Team::Other(f) => {
            let k = crate::world::pro::other::league_of(f);
            vec![(
                "Payroll".into(),
                match k {
                    crate::world::pro::other::CFL => {
                        "a hard salary cap set by the league each year, and a ratio of Canadians to fill".into()
                    }
                    crate::world::pro::other::AFL1 | crate::world::pro::other::AFL2 => {
                        "a small league-wide pay scale: most players earn a few hundred dollars a game".into()
                    }
                    _ => "league-set pay: the league owns the teams and signs the checks".into(),
                },
            )]
        }
        Team::High(h) => {
            let s = &w.high_schools[h];
            vec![
                (
                    "Budget".into(),
                    match s.resources {
                        r if r > 0.7 => {
                            "a district that funds football well, plus a booster club that raises real money".into()
                        }
                        r if r > 0.4 => {
                            "the district's line item and what the booster club raises at the concession stand".into()
                        }
                        _ => "thin: the booster club pays for meals and travel, and some years for helmets".into(),
                    },
                ),
                (
                    "Coaching stipends".into(),
                    "set by the district's schedule, on top of each coach's teaching salary".into(),
                ),
            ]
        }
    }
}

/// The buildings, in words.
pub fn facilities_words(w: &World, team: Team) -> String {
    match team {
        Team::College(p) => {
            let prog = &w.programs[p];
            let f = prog.facilities;
            let stadium = if prog.stadium > 0 { format!(" The stadium holds {}.", thousands(prog.stadium)) } else { String::new() };
            let words = match f {
                f if f > 0.85 => "Among the best in the country: an indoor field, a football building that recruits for you, a nutrition station and a players' lounge.",
                f if f > 0.65 => "Good: an indoor practice facility, a modern weight room and a football operations building from the last decade.",
                f if f > 0.45 => "Middle of the pack: the weight room is fine, the indoor facility is shared with other sports, and the locker room is due for a renovation.",
                f if f > 0.25 => "Behind: no indoor field of its own, an old weight room, and recruits notice.",
                _ => "Bare bones: a shared weight room, no indoor field, and a locker room older than your players.",
            };
            format!("{words}{stadium}")
        }
        Team::Nfl(_) => "A modern facility with an indoor field, a full sports science department and a cafeteria run by a team dietitian.".into(),
        Team::Other(f) => match crate::world::pro::other::league_of(f) {
            crate::world::pro::other::CFL => "A stadium the club shares with the city, a practice field beside it and a weight room in the bowels of the stands.".into(),
            crate::world::pro::other::AFL1 | crate::world::pro::other::AFL2 => "An indoor arena booked around hockey and concerts, and a practice field rented by the week.".into(),
            _ => "A rented stadium for the spring and a practice site the league arranged, shared some weeks.".into(),
        },
        Team::High(h) => match w.high_schools[h].resources {
            r if r > 0.75 => "A stadium the district built for football, turf, a field house and a weight room a small college would take.".into(),
            r if r > 0.5 => "Turf on the game field, a weight room shared with the other sports, and a field house that floods.".into(),
            r if r > 0.3 => "Grass that's worn by October, a small weight room and a press box the booster club rebuilt.".into(),
            _ => "One field for practice and games, borrowed weights and a press box held together with paint.".into(),
        },
    }
}

/// Numbers with commas.
pub fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// What's due in the first months, by level (takeover item 21).
pub fn due(w: &World, team: Team) -> Vec<(Date, String)> {
    let y = &w.dates;
    let today = w.today;
    let mut v: Vec<(Date, String)> = Vec::new();
    match team.level() {
        Level::College => {
            if let Some((open, close)) = w.folk.window.map(|x| (x.1, x.2)) {
                v.push((open, "The coaching-change transfer window opens for your players (15 days)".into()));
                v.push((close, "The coaching-change window closes".into()));
            }
            v.push((y.signing_day, "National signing day: the rest of the class signs".into()));
            v.push((y.spring_semester, "Spring semester classes start: grades and study hall".into()));
            v.push((y.spring_practice.0, "Spring practice opens (15 sessions)".into()));
            v.push((y.spring_game, "The spring game".into()));
            v.push((y.junior_days.0, "Junior days: juniors on campus".into()));
            v.push((y.portal_open, "The spring transfer window".into()));
            v.push((y.spring_term_end, "Spring term ends: eligibility is checked".into()));
            v.push((y.official_visits_from, "Official visit season opens".into()));
            v.push((y.summer_workouts.0, "Summer workouts".into()));
            v.push((y.media_days.0, "Conference media days".into()));
            v.push((y.fall_camp, "Fall camp".into()));
        }
        Level::Nfl => {
            v.push((y.tag_window_open, "The franchise tag window opens".into()));
            v.push((y.combine_start, "The combine in Indianapolis".into()));
            v.push((y.tag_deadline.date, "The tag deadline".into()));
            if let Some(t) = y.tampering_open {
                v.push((t.date, "The negotiating window opens".into()));
            }
            v.push((y.league_year.date, "The new league year: free agency opens".into()));
            v.push((y.annual_meeting.0, "The league's annual meeting".into()));
            v.push((y.offseason_new_coach, "Your offseason program can start (a new head coach starts early)".into()));
            v.push((y.pro_days.0, "Pro days".into()));
            v.push((y.top30_deadline, "Top-30 visits close".into()));
            v.push((y.draft[0].date, "The draft".into()));
            v.push((y.rookie_minicamp, "Rookie minicamp".into()));
            v.push((y.otas.0, "Organized team activities".into()));
            v.push((y.minicamp.0, "Mandatory minicamp".into()));
            v.push((y.veterans_report, "Veterans report to camp".into()));
        }
        _ => {
            v.push((y.spring_practice.0, "Spring ball, where the state allows it".into()));
            v.push((y.spring_game, "The spring game".into()));
            v.push((y.summer_workouts.0, "Summer strength program and 7-on-7".into()));
            v.push((y.hs_school_start, "School starts: athletics period and fall practice".into()));
        }
    }
    v.retain(|x| x.0 >= today);
    v.sort_by_key(|x| x.0);
    v.dedup_by_key(|x| x.0.0 * 1000 + x.1.len() as i32);
    v
}

/// A player's role last season in words.
pub fn role_words(r: Option<crate::world::PlayingTime>) -> &'static str {
    use crate::world::PlayingTime as P;
    match r {
        Some(P::Starter) => "a starter",
        Some(P::Rotation) => "in the rotation",
        Some(P::Backup) => "a backup",
        Some(P::Redshirt) => "a redshirt",
        _ => "new",
    }
}

/// His class in college ("junior", "redshirt sophomore") or his years in
/// the league, plain.
pub fn class_words(w: &World, i: usize) -> String {
    let p = &w.people[i];
    match p.stage {
        crate::world::Stage::Pro => {
            let yrs = p.pro.as_ref().map_or(0, |c| c.seasons.len());
            match yrs {
                0 => "a rookie".into(),
                1 => "in his second year".into(),
                n => format!("a {}-year veteran", n + 1),
            }
        }
        crate::world::Stage::College => {
            let k = p.college_seasons;
            let rs = if p.redshirt_used { "redshirt " } else { "" };
            let c = match k {
                0 => "freshman",
                1 => "sophomore",
                2 => "junior",
                _ => "senior",
            };
            format!("{rs}{c}")
        }
        _ => {
            let g = (w.year - p.class_of + 12).clamp(9, 12);
            match g {
                9 => "freshman".into(),
                10 => "sophomore".into(),
                11 => "junior".into(),
                _ => "senior".into(),
            }
        }
    }
}

/// Whether he leaves after this season: a senior's last year in college,
/// a high school senior, or a pro whose contract ends.
pub fn leaving(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    match p.stage {
        crate::world::Stage::College => p.college_seasons >= 3 && (p.redshirt_used || p.college_seasons >= 4),
        crate::world::Stage::Pro => p
            .pro
            .as_ref()
            .is_some_and(|c| c.contract.as_ref().is_none_or(|k| k.years.last().is_none_or(|y| y.year <= w.year))),
        _ => p.class_of <= w.year,
    }
}

/// The employer a coach works for, as a team.
pub fn team_of_coach(w: &World, c: usize) -> Option<Team> {
    w.coaches.get(c)?.job.map(|j| Team::from_employer(j.employer))
}

/// A coach's role at his team, plain ("defensive coordinator").
pub fn coach_role(w: &World, c: usize) -> String {
    w.coaches.get(c).and_then(|x| x.job).map_or_else(|| "out of coaching".to_string(), |j| j.role.name().to_string())
}

/// Every assistant on a team's staff now (college and NFL coaches the
/// world keeps), with his role.
pub fn assistants(w: &World, team: Team) -> Vec<(usize, Role)> {
    let mut v = Vec::new();
    match team {
        Team::College(p) => {
            let prog = &w.programs[p];
            if let Some(c) = prog.oc {
                v.push((c, Role::OffensiveCoordinator));
            }
            if let Some(c) = prog.dc {
                v.push((c, Role::DefensiveCoordinator));
            }
            for (g, c) in prog.position_coaches.iter().enumerate() {
                if let Some(c) = c {
                    v.push((*c, Role::PositionCoach(crate::world::coach::Group::ALL[g])));
                }
            }
        }
        Team::Nfl(t) => {
            let tm = &w.nfl[t];
            if let Some(c) = tm.oc {
                v.push((c, Role::OffensiveCoordinator));
            }
            if let Some(c) = tm.dc {
                v.push((c, Role::DefensiveCoordinator));
            }
        }
        Team::Other(f) => {
            for role in [Role::OffensiveCoordinator, Role::DefensiveCoordinator] {
                if let Some(c) = crate::world::pro::other::slot(w, f, role) {
                    v.push((c, role));
                }
            }
        }
        Team::High(_) => {}
    }
    let _ = Employer::College(0);
    v
}

/// Whether he is gone before next season starts: leaving after this one,
/// or already out of football.
pub fn leaving_now(w: &World, i: usize) -> bool {
    w.people[i].stage == crate::world::Stage::Done || (leaving(w, i) && w.people[i].stage != crate::world::Stage::Pro)
}

/// A team's games this season: (date, opponent, at home).
pub fn team_games(w: &World, team: Team) -> Vec<(Date, Team, bool)> {
    let mut v: Vec<(Date, Team, bool)> = match team {
        Team::College(p) => w
            .college_fixtures(Some(p))
            .into_iter()
            .filter(|f| !f.cancelled)
            .map(|f| (f.date, Team::College(if f.home == p { f.away } else { f.home }), f.home == p))
            .collect(),
        Team::Nfl(t) => w
            .nfl_fixtures(Some(t))
            .into_iter()
            .filter_map(|f| Some((f.when?, Team::Nfl(if f.home == t { f.away } else { f.home }), f.home == t)))
            .collect(),
        Team::High(h) => w
            .hs_fixtures(h)
            .into_iter()
            .map(|f| (f.date, Team::High(if f.home == h { f.away } else { f.home }), f.home == h))
            .collect(),
        Team::Other(f) => {
            crate::world::pro::other::fixtures(w, f).into_iter().map(|(d, o, home)| (d, Team::Other(o), home)).collect()
        }
    };
    v.sort_by_key(|x| x.0);
    v
}

/// Your team's last game kept whole: (won, our points, theirs, opponent).
pub fn last_game(w: &World, team: Team) -> Option<(bool, i32, i32, Team)> {
    let (k, i) = team.parts();
    let g = w.seat_games.iter().rev().find(|g| g.teams.contains(&i) && (k == "nfl") == (g.level == Level::Nfl))?;
    let side = if g.teams[0] == i { 0 } else { 1 };
    let (us, them) = (g.stats.score[side], g.stats.score[1 - side]);
    let opp = g.teams[1 - side];
    let o = match team {
        Team::College(_) => Team::College(opp),
        Team::Nfl(_) => Team::Nfl(opp),
        Team::High(_) => Team::High(opp),
        // Other leagues' games aren't kept whole.
        Team::Other(_) => return None,
    };
    Some((us > them, us, them, o))
}

/// Your team's record this season from the games kept whole.
pub fn record_now(w: &World, team: Team) -> (u32, u32) {
    let (k, i) = team.parts();
    let start = match team.level() {
        Level::Nfl => w.dates.preseason[2].add(3),
        _ => w.dates.fall_camp,
    };
    let mut r = (0, 0);
    for g in w
        .seat_games
        .iter()
        .filter(|g| g.date >= start && g.teams.contains(&i) && (k == "nfl") == (g.level == Level::Nfl))
    {
        let side = if g.teams[0] == i { 0 } else { 1 };
        if g.stats.score[side] > g.stats.score[1 - side] {
            r.0 += 1;
        } else {
            r.1 += 1;
        }
    }
    r
}

/// A state's name from its postal code.
pub fn state_name(code: &str) -> &'static str {
    const NAMES: [(&str, &str); 51] = [
        ("AL", "Alabama"),
        ("AK", "Alaska"),
        ("AZ", "Arizona"),
        ("AR", "Arkansas"),
        ("CA", "California"),
        ("CO", "Colorado"),
        ("CT", "Connecticut"),
        ("DE", "Delaware"),
        ("DC", "the District"),
        ("FL", "Florida"),
        ("GA", "Georgia"),
        ("HI", "Hawaii"),
        ("ID", "Idaho"),
        ("IL", "Illinois"),
        ("IN", "Indiana"),
        ("IA", "Iowa"),
        ("KS", "Kansas"),
        ("KY", "Kentucky"),
        ("LA", "Louisiana"),
        ("ME", "Maine"),
        ("MD", "Maryland"),
        ("MA", "Massachusetts"),
        ("MI", "Michigan"),
        ("MN", "Minnesota"),
        ("MS", "Mississippi"),
        ("MO", "Missouri"),
        ("MT", "Montana"),
        ("NE", "Nebraska"),
        ("NV", "Nevada"),
        ("NH", "New Hampshire"),
        ("NJ", "New Jersey"),
        ("NM", "New Mexico"),
        ("NY", "New York"),
        ("NC", "North Carolina"),
        ("ND", "North Dakota"),
        ("OH", "Ohio"),
        ("OK", "Oklahoma"),
        ("OR", "Oregon"),
        ("PA", "Pennsylvania"),
        ("RI", "Rhode Island"),
        ("SC", "South Carolina"),
        ("SD", "South Dakota"),
        ("TN", "Tennessee"),
        ("TX", "Texas"),
        ("UT", "Utah"),
        ("VT", "Vermont"),
        ("VA", "Virginia"),
        ("WA", "Washington"),
        ("WV", "West Virginia"),
        ("WI", "Wisconsin"),
        ("WY", "Wyoming"),
    ];
    NAMES.iter().find(|x| x.0 == code).map_or("this", |x| x.1)
}
