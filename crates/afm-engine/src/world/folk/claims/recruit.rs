//! What changed for a recruit or a transfer since you last spoke, and what
//! is coming for him (talk facts design, section 6, "How a call flows" and
//! "Out of season"; rule 9). A call opens on what is freshest: your game on
//! Saturday, his own, a new offer, a visit, a commitment at his spot, a
//! coach who left. When nothing fresh has happened it looks ahead: visits
//! opening, the end of a dead period, his visit, your camp, signing day.
//!
//! Everything here is public or lived by both sides: scores, offers,
//! visits, the services' order, meet results, signings. Nothing reads his
//! plans (the day he means to decide is his own, `commit_timing`).
//!
//! Kinds with no reader yet, because the world keeps no dated record: a
//! decommitment (only a yearly count), his test score (no day), a building
//! announced or opened (years, not days; the `Buildings` pitch names
//! them), your pro day as an event (no day; the `ProDay` pitch names it).

use super::alumni;
use super::kinds::{self, Kind};
use super::player::last_name;
use super::program::{self, Ctx};
use super::sheet::{self, Point, Ref};
use super::{team, words};
use crate::calendar::{Date, Link, Period, Weekday, recruiting_period};
use crate::world::coach::{Employer, Group as Room, Role};
use crate::world::college::PollKind;
use crate::world::folk::Team;
use crate::world::front_office::college::{VisitKind, VisitState};
use crate::world::news::NewsKind;
use crate::world::scouting::services;
use crate::world::times::{At, Event as Timed, Timed as Clock};
use crate::world::{Stage, Visit};

/// A game this close to his home is one his family can drive to, in
/// miles. Proposed, not confirmed.
pub const NEAR_GAME: f64 = 120.0;
/// How far ahead a dated thing is named when nothing fresh has happened.
/// Proposed, not confirmed.
pub const AHEAD: i32 = kinds::AHEAD_DAYS;
/// A move of this many places in the national order is a big one.
/// Proposed, not confirmed.
pub const BIG_MOVE: i64 = 50;
/// A final or August ranking this high is its own word.
/// Proposed, not confirmed.
pub const TOP_TEN: usize = 10;

pub(super) fn points(r: &super::Reader, i: usize, prog: usize, out: &mut Vec<Point>) {
    if prog >= r.w.programs.len() {
        return;
    }
    let c = Ctx::new(r, i);
    your_games(&c, prog, out);
    out.extend(his_game(&c));
    out.extend(new_offer(&c, prog));
    visits(&c, prog, out);
    at_spot(&c, prog, out);
    out.extend(transfer_in(&c, prog));
    staff_news(&c, prog, out);
    out.extend(your_draft(&c, prog));
    times(&c, prog, out);
    out.extend(rank_update(&c));
    out.extend(top_schools(&c, prog));
    out.extend(insider_pick(&c, prog));
    honors(&c, out);
    polls(&c, prog, out);
    out.extend(your_spring(&c, prog));
    out.extend(schedule(&c, prog));
    out.extend(your_job(&c, prog));
    out.extend(entered_portal(&c));
    ahead(&c, out);
}

/// Your last game for a week, and how your season ended for a month.
fn your_games(c: &Ctx, prog: usize, out: &mut Vec<Point>) {
    let team = Team::College(prog);
    let played = team::played(c.r, team);
    if let Some(g) = played.first()
        && g.us != g.them
        && sheet::in_window(Kind::YourWin, g.day, c.r.today)
    {
        let kind = if g.us > g.them { Kind::YourWin } else { Kind::YourLoss };
        out.push(team::game_point(c.r, kind, g));
    }
    out.extend(team::season_ended(c.r, team, &played, Kind::YourBowl));
}

/// His own school's last game, in the week after it: the opponent and the
/// day. Other schools' games keep no score, so the line asks how it went.
fn his_game(c: &Ctx) -> Option<Point> {
    if c.p.stage != Stage::HighSchool {
        return None;
    }
    let (w, h, today) = (c.w, c.p.hs, c.r.today);
    let f = w
        .hs_fixtures(h)
        .into_iter()
        .filter(|f| f.date < today && sheet::in_window(Kind::HisGame, f.date, today))
        .max_by_key(|f| f.date)?;
    let opp = Team::High(if f.home == h { f.away } else { f.home });
    Some(
        Point::new(Kind::HisGame)
            .on(f.date)
            .text("opp", opp.name(w))
            .text("when", words::when_words(f.date, today))
            .state("place", if f.home == h { "home" } else { "away" })
            .state("round", if f.playoff { "playoff" } else { "season" })
            .refer(Ref::Team(opp)),
    )
}

/// A new offer from another school in the last three weeks: the newest
/// by name, and how many came.
fn new_offer(c: &Ctx, prog: usize) -> Option<Point> {
    let today = c.r.today;
    let new: Vec<(usize, Date)> =
        c.p.cal
            .early_offers
            .iter()
            .copied()
            .filter(|o| o.0 != prog && o.0 < c.w.programs.len() && sheet::in_window(Kind::NewOffer, o.1, today))
            .collect();
    let &(j, day) = new.iter().max_by_key(|o| (o.1, o.0))?;
    Some(
        Point::new(Kind::NewOffer)
            .on(day)
            .text("school", c.w.program_name(j))
            .text("when", words::when_words(day, today))
            .count("offers", new.len() as i64)
            .refer(Ref::Team(Team::College(j))),
    )
}

fn visit_word(v: Visit) -> &'static str {
    if v == Visit::Official { "official" } else { "unofficial" }
}

/// The visits he took: one elsewhere and one to you in the last three
/// weeks, and his visit to you that is set.
fn visits(c: &Ctx, prog: usize, out: &mut Vec<Point>) {
    let (w, today) = (c.w, c.r.today);
    let took = || c.p.cal.visits.iter().filter(|v| matches!(v.2, Visit::Official | Visit::Unofficial));
    if let Some(v) = took()
        .filter(|v| v.0 != prog && v.0 < w.programs.len() && sheet::in_window(Kind::VisitElsewhere, v.1, today))
        .max_by_key(|v| (v.1, v.0))
    {
        out.push(
            Point::new(Kind::VisitElsewhere)
                .on(v.1)
                .text("school", w.program_name(v.0))
                .text("when", words::when_words(v.1, today))
                .state("visit", visit_word(v.2))
                .refer(Ref::Team(Team::College(v.0))),
        );
    }
    if let Some(v) = took().filter(|v| v.0 == prog && sheet::in_window(Kind::YourVisit, v.1, today)).max_by_key(|v| v.1)
    {
        out.push(
            Point::new(Kind::YourVisit)
                .on(v.1)
                .text("when", words::when_words(v.1, today))
                .state("visit", visit_word(v.2)),
        );
    }
    // The visit ahead: one on his own calendar, or one you set that he
    // accepted.
    let mut next: Option<(Date, &'static str)> =
        took().filter(|v| v.0 == prog && v.1 > today).map(|v| (v.1, visit_word(v.2))).min();
    if c.mine(prog) {
        let id = c.p.player.id;
        let set = w
            .front_office
            .college
            .visits
            .iter()
            .filter(|pl| pl.id == id && pl.state == VisitState::Set && pl.date >= today)
            .filter_map(|pl| match pl.kind {
                VisitKind::Official => Some((pl.date, "official")),
                VisitKind::Unofficial => Some((pl.date, "unofficial")),
                _ => None,
            })
            .min();
        next = match (next, set) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }
    if let Some((day, kind)) = next.filter(|x| today.days_to(x.0) <= AHEAD) {
        out.push(
            Point::new(Kind::HisVisit)
                .due(day)
                .text("when", words::when_words(day, today))
                .count("days", today.days_to(day) as i64)
                .state("visit", kind),
        );
    }
}

/// A commitment you took at his spot, and who signed with you there, in
/// the last month: the newest of each, by name.
fn at_spot(c: &Ctx, prog: usize, out: &mut Vec<Point>) {
    let (w, today) = (c.w, c.r.today);
    let room = Room::of(c.pos);
    let mut newest: [Option<(Date, usize)>; 2] = [None, None];
    for (i, day, signed) in c.r.joined(prog) {
        if i == c.i || Room::of(w.people[i].player.position) != room {
            continue;
        }
        let slot = &mut newest[signed as usize];
        if slot.is_none_or(|b| (day, i) > b) {
            *slot = Some((day, i));
        }
    }
    for (kind, slot) in [(Kind::CommitAtSpot, newest[0]), (Kind::SigneeAtSpot, newest[1])] {
        let Some((day, i)) = slot else { continue };
        let q = &w.people[i];
        let mut pt = Point::new(kind)
            .on(day)
            .text("name", q.name.clone())
            .text("spot", words::spot(q.player.position))
            .text("when", words::when_words(day, today))
            .refer(Ref::Player(q.player.id));
        // The stars the services gave him are public.
        if q.stars > 0 {
            pt = pt.count("stars", q.stars as i64);
        }
        out.push(pt);
    }
}

/// A transfer who landed with you at his spot in the last month.
fn transfer_in(c: &Ctx, prog: usize) -> Option<Point> {
    let (w, today) = (c.w, c.r.today);
    let room = Room::of(c.pos);
    let season = w.dates.season;
    let (rec, i) = w
        .recruiting_log
        .portal
        .iter()
        .rev()
        .take_while(|x| x.year + 1 >= season)
        .filter(|x| {
            x.to_program == Some(prog) && x.id != c.p.player.id && sheet::in_window(Kind::TransferIn, x.decided, today)
        })
        .filter_map(|x| {
            let i = crate::world::folk::person(w, x.id)?;
            (Room::of(w.people[i].player.position) == room).then_some((x, i))
        })
        .max_by_key(|x| (x.0.decided, x.0.id))?;
    let q = &w.people[i];
    let mut pt = Point::new(Kind::TransferIn)
        .on(rec.decided)
        .text("name", q.name.clone())
        .text("spot", words::spot(q.player.position))
        .text("when", words::when_words(rec.decided, today))
        .refer(Ref::Player(q.player.id));
    if rec.from_program < w.programs.len() {
        pt = pt.text("from", w.program_name(rec.from_program));
    }
    Some(pt)
}

/// Your staff in the news in the last two months: a head coach who left
/// or was fired, one who was hired, and the man who would coach his
/// position when he is new in the job this season.
fn staff_news(c: &Ctx, prog: usize, out: &mut Vec<Point>) {
    let (w, today) = (c.w, c.r.today);
    let coach_of = |refs: &[crate::calendar::Ref]| {
        refs.iter().find_map(|r| match r.to {
            Link::Coach(k) if (k as usize) < w.coaches.len() => Some(k as usize),
            _ => None,
        })
    };
    let (mut left, mut hired) = (None, None);
    for n in w.news.iter().rev().take_while(|n| n.year + 1 >= w.year) {
        let Some(day) = n.date else { continue };
        if n.program != Some(prog) {
            continue;
        }
        match n.kind {
            NewsKind::CoachLeft | NewsKind::CoachFired
                if left.is_none() && sheet::in_window(Kind::CoachLeft, day, today) =>
            {
                left = Some((day, n));
            }
            NewsKind::CoachHired if hired.is_none() && sheet::in_window(Kind::CoachHired, day, today) => {
                hired = Some((day, n));
            }
            _ => {}
        }
    }
    if let Some((day, n)) = left {
        let mut pt = Point::new(Kind::CoachLeft)
            .on(day)
            .text("when", words::when_words(day, today))
            .state("how", if matches!(n.kind, NewsKind::CoachFired) { "fired" } else { "left" });
        if let Some(k) = coach_of(&n.refs) {
            pt = pt.text("coach", format!("Coach {}", last_name(&w.coaches[k].name))).refer(Ref::Coach(k));
        }
        out.push(pt);
    }
    let mut named = None;
    if let Some((day, n)) = hired {
        let mut pt = Point::new(Kind::CoachHired).on(day).text("when", words::when_words(day, today));
        if let Some(k) = coach_of(&n.refs) {
            named = Some(k);
            pt = pt.text("coach", format!("Coach {}", last_name(&w.coaches[k].name))).refer(Ref::Coach(k));
            if let Some(job) = w.coaches[k].job.filter(|j| j.employer == Employer::College(prog)) {
                pt = pt
                    .text("title", job.role.name())
                    .state("role", if job.role == Role::HeadCoach { "head" } else { "assistant" });
            }
        }
        out.push(pt);
    }
    // A hiring below the head coach has a season, not a day: the new man
    // at his position is news until that first season's opener.
    let g = Room::of(c.pos);
    if let Some(k) = w.programs[prog].position_coaches[g as usize]
        && named != Some(k)
        && let Some(job) = w.coaches[k].job
        && job.first == w.dates.season
        && w.dates.cfb_weeks.first().is_some_and(|&d| today <= d)
    {
        out.push(
            Point::new(Kind::CoachHired)
                .text("coach", format!("Coach {}", last_name(&w.coaches[k].name)))
                .text("title", g.coach_title())
                .state("role", "position")
                .refer(Ref::Coach(k)),
        );
    }
}

/// Your draft, in the month after it: how many went, your first pick by
/// name, and whether one of them plays his spot.
fn your_draft(c: &Ctx, prog: usize) -> Option<Point> {
    let (w, today) = (c.w, c.r.today);
    let first_night = alumni::draft_day(w, 1);
    if !sheet::in_window(Kind::YourDraft, first_night, today) {
        return None;
    }
    let class: Vec<_> = c.ix.class(prog, w.year).filter(|a| alumni::draft_day(w, a.round) <= today).collect();
    let top = class.first()?;
    let room = Room::of(c.pos);
    let here: Vec<_> = class.iter().filter(|a| Room::of(a.position) == room).collect();
    let mut pt = Point::new(Kind::YourDraft)
        .on(first_night)
        .count("picks", class.len() as i64)
        .text("picked", words::counted(class.len() as u32, "player"))
        .text("top", top.name.clone())
        .text("round", words::round_word(top.round as u32))
        .text("club", top.club.clone())
        .state("spot", if here.is_empty() { "no" } else { "yes" });
    if let Some(a) = here.first() {
        pt = pt.text("at_spot", a.name.clone()).text("spot_round", words::round_word(a.round as u32));
    }
    Some(pt)
}

/// His times in the last month: a track meet, a laser forty at a showcase
/// or another school's camp, and the forty he ran at your camp.
fn times(c: &Ctx, prog: usize, out: &mut Vec<Point>) {
    let (w, today) = (c.w, c.r.today);
    let all = w.times.of(c.p.player.id);
    if let Some(t) = all
        .iter()
        .filter(|t| {
            matches!(t.at, At::Meet | At::StateMeet)
                && matches!(t.event, Timed::M100 | Timed::M200)
                && sheet::in_window(Kind::TrackTime, t.date, today)
        })
        .max_by_key(|t| (t.at == At::StateMeet, t.date, t.event == Timed::M100))
    {
        out.push(
            Point::new(Kind::TrackTime)
                .on(t.date)
                .text("event", if t.event == Timed::M100 { "100 meters" } else { "200 meters" })
                .measured("time", t.secs as f64, 2)
                .text("when", words::when_words(t.date, today))
                .state("meet", if t.at == At::StateMeet { "state" } else { "season" }),
        );
    }
    let laser = || all.iter().filter(|t| t.event == Timed::Forty && t.timed == Clock::Laser);
    if let Some(t) = laser()
        .filter(|t| {
            let there = match t.at {
                At::Regional | At::National => true,
                At::Camp(j) => j as usize != prog && (j as usize) < w.programs.len(),
                _ => false,
            };
            there && sheet::in_window(Kind::CampForty, t.date, today)
        })
        .max_by_key(|t| t.date)
    {
        // A showcase posts its times; another school's camp keeps its own.
        let (place, public) = match t.at {
            At::National => ("the national showcase".to_string(), true),
            At::Camp(j) => (format!("{}'s camp", w.program_name(j as usize)), false),
            _ => ("a regional showcase".to_string(), true),
        };
        out.push(
            Point::new(Kind::CampForty)
                .on(t.date)
                .measured("forty", t.secs as f64, 2)
                .text("place", place)
                .text("when", words::when_words(t.date, today))
                .known_if(public),
        );
    }
    if let Some(t) = laser()
        .filter(|t| t.at == At::Camp(prog as u16) && sheet::in_window(Kind::YourCamp, t.date, today))
        .max_by_key(|t| t.date)
    {
        out.push(
            Point::new(Kind::YourCamp)
                .on(t.date)
                .measured("forty", t.secs as f64, 2)
                .text("when", words::when_words(t.date, today)),
        );
    }
}

/// The services' update in the last month: where he stands in the
/// national order and how far he moved. Their order is public; their read
/// of him is not, and isn't read.
fn rank_update(c: &Ctx) -> Option<Point> {
    let today = c.r.today;
    let ranks = services::class_ranks(c.w, c.p.class_of)?;
    if !sheet::in_window(Kind::RankUpdate, ranks.date, today) {
        return None;
    }
    let id = c.p.player.id;
    let now = ranks.ids.iter().position(|&x| x == id)? as i64 + 1;
    let was = ranks.prev.iter().position(|&x| x == id).map(|k| k as i64 + 1);
    let (dir, moved) = match was {
        None => ("new", 0),
        Some(b) if b > now => ("up", b - now),
        Some(b) if b < now => ("down", now - b),
        Some(_) => ("same", 0),
    };
    let mut pt = Point::new(Kind::RankUpdate)
        .on(ranks.date)
        .count("rank", now)
        .text("when", words::when_words(ranks.date, today))
        .state("move", dir)
        .state("size", if moved >= BIG_MOVE { "big" } else { "small" });
    if moved > 0 {
        pt = pt.count("places", moved);
    }
    if let Some(b) = was {
        pt = pt.count("was", b);
    }
    Some(pt)
}

/// The top schools he named in the last month, and whether you are on the
/// list. The list says who is in, not who leads.
fn top_schools(c: &Ctx, prog: usize) -> Option<Point> {
    let (w, today) = (c.w, c.r.today);
    let id = c.p.player.id;
    let t = w.scouting.rankings.tops.iter().rev().find(|t| t.id == id)?;
    if !sheet::in_window(Kind::TopSchools, t.date, today) {
        return None;
    }
    let names: Vec<String> = t
        .schools
        .iter()
        .filter(|&&j| (j as usize) < w.programs.len())
        .map(|&j| w.program_name(j as usize).to_string())
        .collect();
    Some(
        Point::new(Kind::TopSchools)
            .on(t.date)
            .count("schools", names.len() as i64)
            .text("list", words::list(&names))
            .text("when", words::when_words(t.date, today))
            .state("in", if t.schools.contains(&(prog as u16)) { "yes" } else { "no" }),
    )
}

/// An insider's newest pick for him in the last month.
fn insider_pick(c: &Ctx, prog: usize) -> Option<Point> {
    let (w, today) = (c.w, c.r.today);
    let id = c.p.player.id;
    let k = w
        .scouting
        .rankings
        .picks
        .iter()
        .filter(|k| {
            k.id == id && (k.program as usize) < w.programs.len() && sheet::in_window(Kind::InsiderPick, k.date, today)
        })
        .max_by_key(|k| (k.date, k.desk))?;
    let j = k.program as usize;
    let mut pt = Point::new(Kind::InsiderPick)
        .on(k.date)
        .text("school", w.program_name(j))
        .text("when", words::when_words(k.date, today))
        .state("for", if j == prog { "you" } else { "other" })
        .refer(Ref::Team(Team::College(j)));
    if let Some(d) = w.scouting.desks.get(k.desk as usize) {
        pt = pt.text("insider", services::insider_name(d));
    }
    Some(pt)
}

/// His honors and his team's run, for two months after his state's
/// finals: all-state, the all-star game he was invited to, a title or a
/// playoff run.
fn honors(c: &Ctx, out: &mut Vec<Point>) {
    if c.p.stage != Stage::HighSchool {
        return;
    }
    let (w, today) = (c.w, c.r.today);
    let Some(hs) = w.high_schools.get(c.p.hs) else { return };
    let (Some(s), Some(state)) = (hs.seasons.last(), w.states.get(hs.state)) else { return };
    let day = crate::world::hs_year::state_season_dates(&state.code, s.year).2;
    if !sheet::in_window(Kind::AllState, day, today) {
        return;
    }
    let team = match c.p.cal.all_state {
        0 => None,
        1 => Some("second"),
        _ => Some("first"),
    };
    if team.is_some() || c.p.cal.hs_all_star.is_some() {
        let mut pt = Point::new(Kind::AllState).on(day).state("team", team.unwrap_or("none"));
        if let Some(game) = c.p.cal.hs_all_star {
            pt = pt.text("game", game);
        }
        out.push(pt);
    }
    if s.state_title || s.semifinal || s.playoff_wins > 0 {
        let finish = if s.state_title {
            "title"
        } else if s.semifinal {
            "semifinal"
        } else {
            "playoffs"
        };
        let mut pt = Point::new(Kind::TitleRun)
            .on(day)
            .text("school", Team::High(c.p.hs).name(w))
            .text("record", format!("{}-{}", s.wins, s.losses))
            .count("playoff_wins", s.playoff_wins as i64)
            .state("finish", finish);
        if let Some(t) = &s.title {
            pt = pt.text("title", t.clone());
        }
        out.push(pt);
    }
}

/// Where you stand in the final poll and the August poll, for a month
/// after each. An unranked team has no point to make.
fn polls(c: &Ctx, prog: usize, out: &mut Vec<Point>) {
    let (w, today) = (c.w, c.r.today);
    let now = w.college.as_ref().map_or(&[][..], |s| s.polls.as_slice());
    for (poll, kind) in [(PollKind::Final, Kind::FinalPoll), (PollKind::ApPreseason, Kind::AugustPoll)] {
        let Some(p) = now
            .iter()
            .chain(w.prev_polls.iter())
            .filter(|p| p.kind == poll && sheet::in_window(kind, p.date, today))
            .max_by_key(|p| p.date)
        else {
            continue;
        };
        let Some(rank) = p.top.iter().position(|&t| t == prog) else { continue };
        out.push(
            Point::new(kind)
                .on(p.date)
                .count("rank", rank as i64 + 1)
                .text("when", words::when_words(p.date, today))
                .state("top", if rank < TOP_TEN { "ten" } else { "ranked" }),
        );
    }
}

/// Your spring, for a month after the spring game: the system you are
/// installing on his side of the ball.
fn your_spring(c: &Ctx, prog: usize) -> Option<Point> {
    let (w, today) = (c.w, c.r.today);
    let day = w.dates.spring_game;
    if c.pos.is_specialist() || !sheet::in_window(Kind::YourSpring, day, today) {
        return None;
    }
    let inst = &w.programs[prog].installed;
    let known = if words::on_offense(c.pos) { inst.off_known } else { inst.def_known };
    if known >= 1.0 {
        return None;
    }
    let system = crate::world::systems::system_words(c.pos, inst)?;
    Some(Point::new(Kind::YourSpring).on(day).text("system", system).state("news", "system"))
}

/// The schedule ahead: your opener before the season starts, and a game
/// you play near his home.
fn schedule(c: &Ctx, prog: usize) -> Option<Point> {
    let (w, today) = (c.w, c.r.today);
    let all = w.college_fixtures_in(Some(prog), false);
    let started = all.iter().any(|f| f.date <= today);
    let ahead = || all.iter().filter(|f| f.date > today && !f.cancelled);
    let near = ahead()
        .filter(|f| f.home != prog && !f.neutral && f.home < w.programs.len())
        .map(|f| (f, program::miles(c, f.home)))
        .filter(|x| x.1 <= NEAR_GAME)
        .min_by_key(|x| x.0.date);
    let opener = if started { None } else { ahead().min_by_key(|f| f.date) };
    if opener.is_none() && near.is_none() {
        return None;
    }
    let mut pt = Point::new(Kind::Schedule)
        .state("opener", if opener.is_some() { "yes" } else { "no" })
        .state("near", if near.is_some() { "yes" } else { "no" });
    if let Some(f) = opener {
        let opp = if f.home == prog { f.away } else { f.home };
        pt = pt
            .text("opener", w.program_name(opp))
            .text("opener_day", words::date_words(f.date))
            .state("opener_at", if f.home == prog { "home" } else { "away" })
            .refer(Ref::Team(Team::College(opp)));
    }
    if let Some((f, m)) = near {
        pt = pt
            .text("near", w.program_name(f.home))
            .text("near_day", words::date_words(f.date))
            .count("near_miles", ((m / 10.0).round() as i64 * 10).max(10))
            .refer(Ref::Team(Team::College(f.home)));
    }
    Some(pt)
}

/// Your job in the news: the head coach is on the hot seat (as the
/// grapevine reads his last two seasons).
fn your_job(c: &Ctx, prog: usize) -> Option<Point> {
    let w = c.w;
    let k = w.programs[prog].head_coach?;
    program::hot_seat(w, k).then(|| {
        Point::new(Kind::YourJob).text("coach", format!("Coach {}", last_name(&w.coaches[k].name))).refer(Ref::Coach(k))
    })
}

/// A transfer: the day he entered the portal, for a month.
fn entered_portal(c: &Ctx) -> Option<Point> {
    let today = c.r.today;
    let e = c.p.cal.portal?;
    if e.on_hold() || !sheet::in_window(Kind::EnteredPortal, e.entered, today) {
        return None;
    }
    Some(
        Point::new(Kind::EnteredPortal)
            .on(e.entered)
            .text("when", words::when_words(e.entered, today))
            .count("days", e.entered.days_to(today) as i64),
    )
}

/// What is coming (rule 9): official visits opening, the end of a dead
/// period, your camp, signing day, and a transfer's deadline. A high
/// school recruit's own decision day is his plan and is never read.
fn ahead(c: &Ctx, out: &mut Vec<Point>) {
    let (w, today) = (c.w, c.r.today);
    let y = &w.dates;
    let soon = |d: Date| (1..=AHEAD).contains(&today.days_to(d));
    let point = |k: Kind, d: Date| {
        Point::new(k).due(d).text("when", words::date_words(d)).count("days", today.days_to(d) as i64)
    };
    if c.p.stage == Stage::HighSchool && c.p.cal.signed.is_none() {
        // The class that signs next.
        let next = c.p.class_of == y.season + 1;
        if next && soon(y.official_visits_from) {
            out.push(point(Kind::VisitsOpen, y.official_visits_from));
        }
        if recruiting_period(y, today) == Period::Dead
            && let Some(d) = (1..=AHEAD).map(|k| today.add(k)).find(|&d| recruiting_period(y, d) != Period::Dead)
        {
            out.push(point(Kind::DeadEnds, d).text("then", recruiting_period(y, d).name()));
        }
        // One-day camps: the second and third Saturdays of June.
        let year = today.year();
        if let Some(d) = [2, 3].into_iter().map(|n| Date::nth_weekday(year, 6, Weekday::Sat, n)).find(|&d| soon(d)) {
            out.push(point(Kind::CampDate, d));
        }
        if next && let Some(d) = y.early_signing.into_iter().chain([y.signing_day]).find(|&d| soon(d)) {
            out.push(
                point(Kind::SigningDay, d).state("day", if Some(d) == y.early_signing { "early" } else { "february" }),
            );
        }
    }
    // A transfer's deadline is his school's and the portal's, not a plan:
    // you know it once you have talked.
    if let Some(e) = c.p.cal.portal
        && !e.on_hold()
        && (0..=AHEAD).contains(&today.days_to(e.decide_by))
    {
        out.push(
            point(Kind::DecisionDay, e.decide_by)
                .state("still", if e.looking { "looking" } else { "deciding" })
                .known_if(program::contacted(c)),
        );
    }
}
