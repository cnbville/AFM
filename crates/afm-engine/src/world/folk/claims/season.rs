//! A player on a team between seasons (talk facts design, section 5, "Out
//! of season"): from the last game to the next first game there is no
//! Saturday to talk about, so the sheet reads what the world keeps in
//! those months. The season that ended, testing day, his week of practice,
//! report day, the spring step, the chart after spring, a new position,
//! the room changing, a new system, a teammate drafted, a failed term, his
//! other sport, the colleges after him, a pro's spring, and the next dated
//! thing on his calendar.
//!
//! A measured number can be said (a forty, a bench, a weight). The size of
//! a growth step never is: only that it came.

use super::alumni::Alumni;
use super::kinds::{self, Kind};
use super::player::{Chart, chart_changes};
use super::sheet::{self, Point, Ref};
use super::{Man, Reader, team, words};
use crate::calendar::Date;
use crate::player::DevelopmentEvent;
use crate::positions::Position;
use crate::training::rules::Block;
use crate::world::Stage;
use crate::world::folk::Team;
use crate::world::times::{At, Event as Timed};

/// A forty this much faster or slower is a change (the tests print
/// hundredths). Proposed, not confirmed.
pub const FORTY_STEP: i32 = 3;
/// A lift this many pounds up or down is a change (the bar moves in fives).
/// Proposed, not confirmed.
pub const LIFT_STEP: i32 = 10;
/// Progress at a new position: getting there from here, ready from here
/// (of 100). Proposed, not confirmed.
pub const MOVE_MIDWAY: u8 = 35;
/// Ready from this much of the move. Proposed, not confirmed.
pub const MOVE_READY: u8 = 75;
/// His own season was a good one from this average game grade, a bad one
/// under this (60 is average). Proposed, not confirmed.
pub const GOOD_SEASON: f64 = 68.0;
/// Under this his own year was a bad one. Proposed, not confirmed.
pub const BAD_SEASON: f64 = 54.0;
/// The next dated thing is on the sheet from this far out.
/// Proposed, not confirmed.
pub const LOOK_AHEAD: i32 = 120;

pub(super) fn points(r: &Reader, m: &Man, out: &mut Vec<Point>) {
    let mut add = |p: Option<Point>| out.extend(p);
    add(season_ended(r, m));
    add(testing(r, m));
    add(practice_week(r, m));
    add(report_day(r, m));
    add(spring_step(r, m));
    add(chart_move(r, m));
    add(new_position(r, m));
    add(room_changed(r, m));
    add(new_system(r, m));
    add(teammate_drafted(r, m));
    add(failed_term(r, m));
    add(other_sport(r, m));
    add(colleges(r, m));
    add(pro_spring(r, m));
    add(effort(r, m));
    add(ahead(r, m));
}

/// The season that ended in the last month, for the team and for him: the
/// record, the last game, and whether his own year was a good one (his
/// games and his grades in your team's games kept whole).
fn season_ended(r: &Reader, m: &Man) -> Option<Point> {
    let team = m.team?;
    let played = team::played(r, team);
    let mut pt = team::season_ended(r, team, &played, Kind::SeasonEnded)?;
    if m.mine {
        let from = played.iter().map(|g| g.day).min()?;
        let grades: Vec<f64> =
            r.w.seat_games
                .iter()
                .filter(|g| g.date >= from)
                .filter_map(|g| super::side_of(g, team).and_then(|s| g.stats.players[s].get(&m.id)))
                .filter(|l| l.graded >= super::player::GRADED_MIN)
                .map(|l| l.grade())
                .collect();
        pt = pt.count("his_games", grades.len() as i64);
        if grades.len() >= 3 {
            let avg = grades.iter().sum::<f64>() / grades.len() as f64;
            pt = pt.state(
                "his",
                if avg >= GOOD_SEASON {
                    "good"
                } else if avg < BAD_SEASON {
                    "bad"
                } else {
                    "even"
                },
            );
        }
    }
    Some(pt)
}

/// Testing day in the last three weeks: his forty, his bench and his
/// weight, against his own last test.
fn testing(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let tests = &r.w.training.tests;
    let (k, day) = tests.iter().enumerate().rev().find(|(_, d)| !d.baseline && d.date <= r.today)?;
    if !sheet::in_window(Kind::TestingDay, day.date, r.today) {
        return None;
    }
    let now = day.rows.iter().find(|x| x.id == m.id)?;
    let before = tests[..k].iter().rev().find_map(|d| d.rows.iter().find(|x| x.id == m.id));
    let mut pt = Point::new(Kind::TestingDay)
        .on(day.date)
        .text("when", words::when_words(day.date, r.today))
        .measured("forty", now.forty as f64 / 100.0, 2)
        .count("bench", now.bench as i64)
        .count("squat", now.squat as i64)
        .count("weight", now.weight as i64);
    match before {
        Some(b) => {
            // Lower is better on the clock, higher under the bar.
            let step = |d: i32, by: i32| {
                if d >= by {
                    1
                } else if d <= -by {
                    -1
                } else {
                    0
                }
            };
            let forty = step(b.forty as i32 - now.forty as i32, FORTY_STEP);
            let bench = step(now.bench as i32 - b.bench as i32, LIFT_STEP);
            let squat = step(now.squat as i32 - b.squat as i32, LIFT_STEP);
            let word = |s: i32| match s {
                1 => "better",
                -1 => "worse",
                _ => "same",
            };
            pt = pt
                .measured("forty_before", b.forty as f64 / 100.0, 2)
                .count("bench_before", b.bench as i64)
                .count("weight_before", b.weight as i64)
                .state("forty", word(forty))
                .state("bench", word(bench))
                .state("test", word((forty + bench + squat).signum()));
        }
        None => pt = pt.state("test", "first"),
    }
    Some(pt)
}

/// His week of practice, in spring, summer or camp: among the week's best
/// or worst, and his one-on-ones won and lost.
fn practice_week(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let l = r.w.training.log.last()?;
    if matches!(l.block, Some(Block::Season | Block::Postseason) | None)
        || !sheet::in_window(Kind::PracticeWeek, l.monday, r.today)
    {
        return None;
    }
    let best = l.best.iter().any(|x| x.0 == m.id);
    let worst = l.worst.iter().any(|x| x.0 == m.id);
    // His one-on-ones: (winner, loser, wins, losses) a pair.
    let (mut won, mut lost) = (0u32, 0u32);
    let mut most: Option<(u32, u32)> = None;
    for &(a, b, wins, losses) in &l.battles {
        let (rival, w_, l_) = if a == m.id {
            (b, wins as u32, losses as u32)
        } else if b == m.id {
            (a, losses as u32, wins as u32)
        } else {
            continue;
        };
        won += w_;
        lost += l_;
        if most.is_none_or(|x| w_ + l_ > x.1) {
            most = Some((rival, w_ + l_));
        }
    }
    if !best && !worst && won + lost == 0 {
        return None;
    }
    let phase = match l.block {
        Some(Block::Spring) => "spring",
        Some(Block::Camp) => "camp",
        Some(Block::Summer) => "summer",
        Some(Block::Otas | Block::Phase1 | Block::Phase2) => "otas",
        _ => "winter",
    };
    let mut pt = Point::new(Kind::PracticeWeek).on(l.monday).state("phase", phase).state(
        "grade",
        if best {
            "best"
        } else if worst {
            "worst"
        } else {
            "middle"
        },
    );
    if won + lost > 0 {
        pt = pt.count("won", won as i64).count("lost", lost as i64).state(
            "battles",
            match won.cmp(&lost) {
                std::cmp::Ordering::Greater => "won",
                std::cmp::Ordering::Less => "lost",
                std::cmp::Ordering::Equal => "split",
            },
        );
        if let Some((rival, _)) = most
            && let Some(name) = r.name(rival)
        {
            pt = pt.text("rival", name).refer(Ref::Player(rival));
        }
    }
    Some(pt)
}

/// Report day in the last two weeks: in shape or out of shape, and the
/// pounds over his target, from your strength staff's notes.
fn report_day(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let notes = &r.w.training.notes;
    let fresh = |d: Date| sheet::in_window(Kind::ReportDay, d, r.today);
    let own = notes
        .iter()
        .rev()
        .find(|n| n.id == m.id && n.dept == "strength" && n.text.contains("reported ") && fresh(n.date));
    // The team's line names the men over their weight: "(12 lb over)".
    let tag = format!("[[p:{}|", m.id);
    let over =
        notes.iter().rev().filter(|n| n.id == 0 && n.text.starts_with("Report day:") && fresh(n.date)).find_map(|n| {
            let rest = &n.text[n.text.find(&tag)?..];
            let rest = &rest[rest.find("]] (")? + 4..];
            let lb: i64 = rest.split_whitespace().next()?.parse().ok()?;
            rest.split_whitespace().nth(1).is_some_and(|x| x == "lb").then_some((n.date, lb))
        });
    let day = own.map(|n| n.date).or(over.map(|x| x.0))?;
    let shape = match own {
        Some(n) if n.text.contains("out of shape") => "out",
        Some(n) if n.text.contains("great shape") => "great",
        _ => "heavy",
    };
    let mut pt = Point::new(Kind::ReportDay).on(day).state("shape", shape);
    if let Some((_, lb)) = over {
        pt = pt.count("pounds", lb);
    }
    Some(pt)
}

/// The spring step: his coaches tipped him for one (from the day spring
/// practice opens to the spring game), and for a month after the spring
/// game whether it came. Never its size.
fn spring_step(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    if p.cal.spring_notice {
        return Some(Point::new(Kind::SpringStep).due(w.dates.spring_game).state("step", "tipped"));
    }
    let game = w.dates.spring_game;
    let age = game.days_to(r.today);
    if !(0..=kinds::NEWS_DAYS).contains(&age) {
        return None;
    }
    // hidden: the event is on his row and a step is on its way; the
    // step's size is never read.
    let came = p.player.history.last().is_some_and(|x| x.events.has(DevelopmentEvent::SpringBreakout))
        || p.cal.spring_gain_ahead.is_some();
    let at = format!("At {} the spring step he was tipped for", p.row_age());
    let missed = p.notes.iter().rev().take(4).any(|n| n.starts_with(&at) && n.contains("didn't come"));
    let step = if came {
        "came"
    } else if missed {
        "missed"
    } else {
        return None;
    };
    Some(
        Point::new(Kind::SpringStep)
            .on(game)
            .state("step", step)
            .weigh(1.0 - 0.5 * age as f32 / kinds::NEWS_DAYS as f32),
    )
}

/// You moved him on the chart in the last four weeks, between seasons: up
/// or down, and who he passed or who passed him. In season the same order
/// is a job won or lost (`player::job`).
fn chart_move(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine || m.team.is_some_and(|t| r.in_season(t)) {
        return None;
    }
    let (day, other, up) = match chart_changes(r, m).into_iter().last()? {
        Chart::Up(day, o) => (day, o, true),
        Chart::Down(day, o) => (day, o, false),
    };
    let mut pt = Point::new(Kind::ChartMove).on(day).state("move", if up { "up" } else { "down" });
    if let Some((id, name)) = other {
        pt = pt.text("rival", name).refer(Ref::Player(id));
    }
    Some(pt)
}

/// A position he is learning on your plan, and how far along he is.
fn new_position(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let w = r.w;
    if let Some((to, prog)) = w.training.plan_of(m.id).moving {
        let pos = Position::ALL[to as usize % Position::ALL.len()];
        let stage = if prog >= MOVE_READY {
            "ready"
        } else if prog >= MOVE_MIDWAY {
            "getting_there"
        } else {
            "early"
        };
        return Some(Point::new(Kind::NewPosition).text("position", words::spot(pos)).state("stage", stage));
    }
    // He just finished the move: your position coach's note, for two weeks.
    let n = w
        .training
        .notes
        .iter()
        .rev()
        .find(|n| n.id == m.id && n.dept == "position" && n.text.contains("is ready to play "))?;
    if !(0..=kinds::FORTNIGHT_DAYS).contains(&n.date.days_to(r.today)) {
        return None;
    }
    Some(
        Point::new(Kind::NewPosition)
            .on(n.date)
            .text("position", words::spot(w.people[m.i].player.position))
            .state("stage", "ready"),
    )
}

/// The room at his spot changed in the last month: someone arrived (a
/// signee, an early enrollee, a transfer), or a man at his spot went into
/// the portal.
fn room_changed(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let w = r.w;
    let pos = w.people[m.i].player.position;
    let fresh = |d: Date| sheet::in_window(Kind::RoomChanged, d, r.today);
    // Arrivals on your roster carry their day.
    let mut came: Vec<(Date, usize)> = w
        .training
        .arrived
        .iter()
        .filter(|a| a.0 != m.id && fresh(a.1))
        .filter_map(|a| crate::world::folk::person(w, a.0).map(|i| (a.1, i)))
        .filter(|&(_, i)| w.people[i].player.position == pos && super::team_of(w, i) == m.team)
        .collect();
    came.sort();
    if let Some(&(day, _)) = came.last() {
        let names: Vec<String> = came.iter().rev().take(3).map(|x| w.people[x.1].name.clone()).collect();
        let how = {
            let q = &w.people[came.last()?.1];
            if q.path.len() > 1 || q.transfers > 0 {
                "transfer"
            } else if q.college_seasons == 0 && q.stage == Stage::College {
                "freshman"
            } else {
                "new"
            }
        };
        let mut pt = Point::new(Kind::RoomChanged)
            .on(day)
            .text("names", words::list(&names))
            .count("arrived", came.len() as i64)
            .state("change", "push")
            .state("how", how);
        for x in came.iter().rev().take(3) {
            pt = pt.refer(Ref::Player(w.people[x.1].player.id));
        }
        return Some(pt);
    }
    // A man at his spot in the portal: the road is open.
    let Some(Team::College(prog)) = m.team else { return None };
    let (i, e) = w
        .portal_pending
        .iter()
        .filter_map(|&i| w.people.get(i).and_then(|q| q.cal.portal.map(|e| (i, e))))
        .filter(|&(i, e)| {
            let q = &w.people[i];
            i != m.i && q.player.position == pos && q.path.last() == Some(&prog) && fresh(e.entered)
        })
        .max_by_key(|x| x.1.entered)?;
    Some(
        Point::new(Kind::RoomChanged)
            .on(e.entered)
            .text("names", w.people[i].name.clone())
            .state("change", "open")
            .state("how", "portal")
            .refer(Ref::Player(w.people[i].player.id)),
    )
}

/// A new system installed this spring on his side of the ball, from the
/// day spring practice opens to the opener: the system, and whether your
/// staff reads him as having it.
fn new_system(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let Some(Team::College(prog)) = m.team else { return None };
    let p = &w.people[m.i];
    let pos = p.player.position;
    let inst = &w.programs[prog].installed;
    let known = if words::on_offense(pos) { inst.off_known } else { inst.def_known };
    let opener = w.dates.cfb_weeks.first().copied()?;
    if known >= 1.0 || pos.is_specialist() || r.today < w.dates.spring_practice.0 || r.today > opener {
        return None;
    }
    let system = crate::world::systems::system_words(pos, inst)?;
    let mut pt = Point::new(Kind::NewSystem).text("system", system);
    if m.mine {
        // hidden: your staff's word for his preparation (the training
        // week shows it), never the growth state it is packed beside.
        pt = pt.state(
            "grasp",
            match crate::training::prep::latest(&p.player.train) {
                4 | 5 => "has_it",
                1 | 2 => "behind",
                _ => "learning",
            },
        );
    }
    Some(pt)
}

/// A teammate drafted in the last month: his friend first, then the man
/// at his own spot, then your highest pick.
fn teammate_drafted(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let Some(Team::College(prog)) = m.team else { return None };
    let first_night = super::alumni::draft_day(w, 1);
    if !sheet::in_window(Kind::TeammateDrafted, first_night, r.today) {
        return None;
    }
    let ix = Alumni::get(w);
    let friends = crate::world::folk::ties::friends(w, m.id);
    let pos = w.people[m.i].player.position;
    let day_of = |round: u8| super::alumni::draft_day(w, round);
    let class: Vec<_> = ix.class(prog, w.year).filter(|a| a.id != Some(m.id) && day_of(a.round) <= r.today).collect();
    let (a, tie) = class
        .iter()
        .find(|a| a.id.is_some_and(|id| friends.contains(&id)))
        .map(|a| (*a, "friend"))
        .or_else(|| class.iter().find(|a| a.position == pos).map(|a| (*a, "ahead")))
        .or_else(|| class.first().map(|a| (*a, "teammate")))?;
    let mut pt = Point::new(Kind::TeammateDrafted)
        .on(day_of(a.round))
        .text("name", a.name.clone())
        .text("round", words::round_word(a.round as u32))
        .text("club", a.club.clone())
        .count("picks", class.len() as i64)
        .state("tie", tie);
    if let Some(id) = a.id {
        pt = pt.refer(Ref::Player(id));
    }
    Some(pt)
}

/// He failed a term at the last check, in the last month: out of spring
/// practice (the fall term's check, after December's finals) or out of
/// the fall (the spring term's).
fn failed_term(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    if p.stage != Stage::College {
        return None;
    }
    let y = &w.dates;
    let fresh = |d: Date| sheet::in_window(Kind::FailedTerm, d, r.today);
    if p.cal.spring_ineligible && fresh(y.spring_term_end) {
        return Some(Point::new(Kind::FailedTerm).on(y.spring_term_end).state("out", "fall"));
    }
    if p.cal.fall_ineligible {
        // December's finals: this football year's, or the one before's
        // while the year is young.
        let day = [y.finals_week.1, y.finals_week.1.add(-364)].into_iter().find(|&d| fresh(d))?;
        return Some(Point::new(Kind::FailedTerm).on(day).state("out", "spring"));
    }
    None
}

/// A high school player's other sport, in its season, and his time at the
/// last meet when he runs track.
fn other_sport(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    if p.stage != Stage::HighSchool {
        return None;
    }
    let sport = crate::training::offseason::other_sport(&p.player).filter(|s| s.in_season(r.today))?;
    let mut pt = Point::new(Kind::OtherSport).text("sport", sport.name());
    if let Some(t) = w
        .times
        .of(m.id)
        .iter()
        .rev()
        .find(|t| matches!(t.at, At::Meet | At::StateMeet) && matches!(t.event, Timed::M100 | Timed::M200))
        .filter(|t| (0..=kinds::NEWS_DAYS).contains(&t.date.days_to(r.today)))
    {
        pt = pt
            .on(t.date)
            .measured("time", t.secs as f64, 2)
            .text("event", if t.event == Timed::M100 { "100 meters" } else { "200 meters" })
            .state("meet", if t.at == At::StateMeet { "state" } else { "regular" });
    }
    Some(pt)
}

/// A college moved on your high school player in the last three weeks: an
/// offer, a visit or a camp.
fn colleges(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    if !m.mine || p.stage != Stage::HighSchool {
        return None;
    }
    let fresh = |d: Date| sheet::in_window(Kind::Colleges, d, r.today);
    let mut moves: Vec<(Date, usize, &'static str)> = Vec::new();
    moves.extend(p.cal.early_offers.iter().filter(|o| fresh(o.1)).map(|o| (o.1, o.0, "offer")));
    moves.extend(p.cal.visits.iter().filter(|v| fresh(v.1)).map(|v| (v.1, v.0, "visit")));
    moves.extend(w.times.of(m.id).iter().filter(|t| fresh(t.date)).filter_map(|t| match t.at {
        At::Camp(j) => Some((t.date, j as usize, "camp")),
        _ => None,
    }));
    let &(day, prog, what) = moves.iter().max_by_key(|x| (x.0, x.2 == "offer"))?;
    Some(
        Point::new(Kind::Colleges)
            .on(day)
            .text("school", w.program_name(prog))
            .text("when", words::when_words(day, r.today))
            .count("offers", (p.cal.offers.len() + p.cal.early_offers.len()) as i64)
            .state("move", what)
            .refer(Ref::Team(Team::College(prog))),
    )
}

/// A pro's spring: the tag, extension talks with you, a camp battle.
fn pro_spring(r: &Reader, m: &Man) -> Option<Point> {
    use crate::world::front_office::nfl::{TalkKind, TalkState};
    use crate::world::pro::money::Kind as Deal;
    let w = r.w;
    let c = w.people[m.i].pro.as_ref()?;
    if m.team.is_some_and(|t| r.in_season(t)) {
        return None;
    }
    if let Some(k) = c.contract.as_ref().filter(|k| k.signed >= w.year)
        && matches!(k.kind, Deal::Franchise | Deal::Transition)
    {
        let tag = if k.kind == Deal::Franchise { "franchise" } else { "transition" };
        return Some(Point::new(Kind::ProSpring).text("tag", tag).state("what", "tag"));
    }
    if m.mine
        && let Some(t) = w.front_office.nfl.talk(m.id)
        && t.kind == TalkKind::Extension
        && matches!(t.state, TalkState::Open | TalkState::Standing)
    {
        return Some(Point::new(Kind::ProSpring).on(t.opened).state("what", "talks").weigh(1.0));
    }
    // A camp battle: the man he has faced most in camp's one-on-ones.
    if m.mine
        && let Some(l) = w.training.log.last().filter(|l| l.block == Some(Block::Camp))
        && (0..=kinds::FORTNIGHT_DAYS).contains(&l.monday.days_to(r.today))
    {
        let pos = w.people[m.i].player.position;
        let rival = l
            .battles
            .iter()
            .filter_map(|&(a, b, wins, losses)| {
                let other = if a == m.id {
                    b
                } else if b == m.id {
                    a
                } else {
                    return None;
                };
                Some((other, wins as u32 + losses as u32))
            })
            .filter(|&(o, _)| crate::world::folk::person(w, o).is_some_and(|i| w.people[i].player.position == pos))
            .max_by_key(|x| x.1)?;
        let name = r.name(rival.0)?;
        return Some(
            Point::new(Kind::ProSpring).text("rival", name).state("what", "battle").refer(Ref::Player(rival.0)),
        );
    }
    None
}

/// Your staff flagged his effort in the last two weeks: unprepared two
/// weeks running, by your position coach's note.
fn effort(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let n = r.w.training.notes.iter().rev().find(|n| {
        n.id == m.id && n.dept == "position" && n.text.contains("prepared two weeks running") && n.date <= r.today
    })?;
    let word = if n.text.contains("underprepared") { "underprepared" } else { "unprepared" };
    Some(Point::new(Kind::Effort).on(n.date).state("prep", word))
}

/// The next dated thing on his calendar, for a quiet day (rule 9): the
/// testing day, spring practice, the spring game, report day, the opener.
fn ahead(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let team = m.team?;
    let y = &w.dates;
    let mut next: Vec<(Date, &'static str, &'static str)> = Vec::new();
    match team {
        Team::College(_) => {
            next.push((y.spring_practice.0, "spring", "spring practice"));
            next.push((y.spring_game, "spring_game", "the spring game"));
            next.push((y.summer_workouts.0, "summer", "summer workouts"));
            next.push((y.fall_camp, "camp", "fall camp"));
        }
        Team::Nfl(_) => {
            next.push((y.otas.0, "otas", "OTAs"));
            next.push((y.minicamp.0, "minicamp", "minicamp"));
            next.push((y.veterans_report, "camp", "training camp"));
        }
        Team::High(_) => {
            next.push((y.summer_workouts.0, "summer", "the summer program"));
            next.push((y.hs_school_start, "camp", "fall practice"));
        }
        Team::Other(_) => {}
    }
    if let Some(g) = r.games(team).into_iter().find(|g| g.0 >= r.today) {
        // In season the next game is the team's point, not his.
        if r.in_season(team) {
            return None;
        }
        next.push((g.0, "opener", "the opener"));
    }
    if m.mine
        && let Some(f) = crate::training::seat::seat_frame(w, y)
    {
        let monday = r.today.on_or_before(crate::calendar::Weekday::Mon);
        if let Some(d) = (0..=(LOOK_AHEAD / 7))
            .filter_map(|k| {
                let mon = monday.add(7 * k);
                crate::training::plan::testing_day(&f, mon).map(|d| mon.add(d as i32))
            })
            .find(|&d| d >= r.today)
        {
            next.push((d, "testing", "testing day"));
        }
    }
    let &(day, key, what) =
        next.iter().filter(|x| x.0 >= r.today && r.today.days_to(x.0) <= LOOK_AHEAD).min_by_key(|x| x.0)?;
    Some(
        Point::new(Kind::Ahead)
            .due(day)
            .text("what", what)
            .text("when", words::when_words(day, r.today))
            .count("days", r.today.days_to(day) as i64)
            .state("what", key),
    )
}
