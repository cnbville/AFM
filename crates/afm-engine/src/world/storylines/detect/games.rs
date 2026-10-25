//! After every game: the runs (an unbeaten start, a losing streak), the
//! games that answer a story (the rivalry, the revenge game, the old
//! coach's return, senior day, the trophy), who took the snaps (a
//! quarterback battle's first start, a benching, the starter back from
//! injury), the kicks (a kicker's slump), and the chases (a record, a
//! coach's milestone, six wins for a bowl).

use super::{K, Who, end, head, on, open_of, pname, s_coach, s_pair, s_player, s_season, tell, tname};
use crate::calendar::Level;
use crate::game::GameStats;
use crate::positions::Position;
use crate::world::World;
use crate::world::stories::TeamKey;
use crate::world::storylines::{close, fact, open_id};

/// One finished game as the detectors see it.
pub(crate) struct Played<'a> {
    pub level: Level,
    /// Home, away.
    pub teams: [TeamKey; 2],
    pub score: [i32; 2],
    pub stats: Option<&'a GameStats>,
    pub postseason: bool,
    /// The home side's senior day.
    pub senior_day: bool,
    /// The game for the title (national, state, the Super Bowl).
    pub title: bool,
    /// A conference's title game.
    pub conf_title: bool,
    /// A rivalry game (a listed college rivalry, NFL division rivals, high
    /// school rivals).
    pub rivalry: bool,
}

/// Wins in a row that make an unbeaten start a story, and losses in a row
/// that make a streak one, by level.
fn unbeaten_at(level: Level) -> u32 {
    match level {
        Level::Nfl => 5,
        Level::HighSchool => 7,
        _ => 6,
    }
}
fn losing_at(level: Level) -> i16 {
    match level {
        Level::HighSchool => 5,
        _ => 4,
    }
}

/// The rivalry-pair kinds a game between their two teams answers.
const PAIR_KINDS: [K; 8] = [
    K::RivalryStreak,
    K::CantBeatRival,
    K::RevengeGame,
    K::CoachReturn,
    K::TrophyGame,
    K::Rematch,
    K::BulletinBoard,
    K::OldTeam,
];

pub(crate) fn after(w: &mut World, g: &Played) {
    if !w.recording || !(on(w, g.teams[0]) || on(w, g.teams[1])) {
        return;
    }
    if w.storylines.watch.runs_year != w.year {
        w.storylines.watch.runs.clear();
        w.storylines.watch.kicks.clear();
        w.storylines.watch.met.clear();
        w.storylines.watch.runs_year = w.year;
    }
    let tie = g.score[0] == g.score[1];
    let pair = s_pair(g.teams[0], g.teams[1]);
    let winner = if tie {
        0
    } else if g.score[0] > g.score[1] {
        super::tcode(g.teams[0])
    } else {
        super::tcode(g.teams[1])
    };
    if g.rivalry {
        let e = w.storylines.watch.rivals.entry(pair).or_insert((0, 0));
        *e =
            if winner != 0 && e.0 == winner { (winner, e.1.saturating_add(1)) } else { (winner, (winner != 0) as u16) };
    }
    w.storylines.watch.met.insert(pair, winner);
    for side in 0..2 {
        let (t, o) = (g.teams[side], g.teams[1 - side]);
        let won = !tie && g.score[side] > g.score[1 - side];
        let lost = !tie && !won;
        // The run.
        let run = {
            let r = w.storylines.watch.runs.entry(super::tcode(t)).or_insert(0);
            *r = if won {
                (*r).max(0) + 1
            } else if lost {
                (*r).min(0) - 1
            } else {
                0
            };
            *r
        };
        // The pair stories this game answers.
        pairs(w, g, t, o, won);
        if !on(w, t) {
            continue;
        }
        runs(w, g, t, o, won, lost, run);
        if let Some(stats) = g.stats {
            snaps(w, t, stats, side);
            kicks(w, t, stats, side);
            chases(w, t, stats, side);
        }
        if won {
            milestone(w, t);
        }
        if g.senior_day && side == 0 {
            for (id, _) in open_of(w, K::LastHomeGame) {
                if w.storylines.get(id).is_some_and(|s| s.team == Some(t)) {
                    close(w, id, if won { "won" } else { "lost" }, None);
                }
            }
        }
        let year = w.year;
        if g.postseason && lost {
            end(w, K::TitleDefense, s_season(t, year), "dethroned", None);
        }
        if g.title && won {
            end(w, K::Drought, s_season(t, year), "ended", None);
            end(w, K::TitleDefense, s_season(t, year), "repeated", None);
            end(w, K::Dynasty, s_season(t, year), "goes_on", None);
        }
        if g.title || g.conf_title {
            for (id, _) in open_of(w, K::ConferenceRace) {
                if w.storylines.get(id).is_some_and(|s| s.team == Some(t) && s.season == year)
                    && (g.conf_title || g.title)
                {
                    close(w, id, if won { "won" } else { "lost" }, None);
                }
            }
        }
        if g.title
            && let Some(c) = head(w, t)
        {
            end(w, K::BigOne, s_coach(c), if won { "won_it" } else { "again" }, None);
        }
        // The players who opted out of a bowl: how the team did without them.
        if g.postseason {
            for (id, _) in open_of(w, K::OptOut) {
                if w.storylines.get(id).is_some_and(|s| s.team == Some(t)) {
                    close(w, id, if won { "won_without" } else { "lost_without" }, None);
                }
            }
        }
        bowl_chase(w, t);
    }
}

/// Unbeaten starts and losing streaks.
fn runs(w: &mut World, g: &Played, t: TeamKey, o: TeamKey, won: bool, lost: bool, run: i16) {
    let year = w.year;
    let (wins, losses) = crate::world::storylines::tell::record_now(w, t);
    let name = tname(w, t);
    let oname = tname(w, o);
    let score = format!("{}-{}", g.score[0].max(g.score[1]), g.score[0].min(g.score[1]));
    if lost && let Some(id) = open_id(w, K::Unbeaten, s_season(t, year)) {
        close(w, id, "lost", Some(format!("{name}'s unbeaten run ends at {wins}, beaten {score} by {oname}.")));
    }
    if won && losses == 0 && wins >= unbeaten_at(g.level) {
        let s = super::team_spot(w, K::Unbeaten, t, s_season(t, year))
            .nums([wins as i32, 0, 0, 0])
            .fact(format!("{name} is {wins}-0 after beating {oname} {score}."), 6.0)
            .stakes(if wins >= 9 { 15 } else { 0 })
            .public();
        tell(w, s);
    }
    let at = losing_at(g.level);
    if won && let Some(id) = open_id(w, K::LosingStreak, s_season(t, year)) {
        close(w, id, "snapped", Some(format!("{name} snaps the losing streak, {score} over {oname}.")));
    }
    if lost && -run >= at {
        let n = -run;
        let s = super::team_spot(w, K::LosingStreak, t, s_season(t, year))
            .nums([n as i32, wins as i32, losses as i32, 0])
            .fact(format!("{name} has lost {n} straight after {score} against {oname}."), 4.0)
            .stakes(if n >= 7 { 10 } else { 0 });
        let id = tell(w, s);
        // The streak puts the head coach's seat in play.
        if n >= 5
            && let Some(c) = head(w, t)
            && let Some(_) = id
        {
            super::weekly::seat_look(w, t, c, true);
        }
    }
}

/// The rivalry-pair stories a game between two teams answers.
fn pairs(w: &mut World, g: &Played, t: TeamKey, o: TeamKey, won: bool) {
    let ids: Vec<(u32, u16)> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.phase.open() && s.team == Some(t) && s.other == Some(o))
        .filter(|s| PAIR_KINDS.iter().any(|k| k.id() == s.kind))
        .map(|s| (s.id, s.kind))
        .collect();
    let score = format!("{}-{}", g.score[0].max(g.score[1]), g.score[0].min(g.score[1]));
    for (id, kind) in ids {
        let k = K::ALL[kind as usize];
        let ending = match (k, won) {
            (K::RivalryStreak, true) => "extended",
            (K::RivalryStreak, false) => "snapped",
            (K::CantBeatRival, true) => "beat_them",
            (K::CantBeatRival, false) => "lost_again",
            (K::RevengeGame, true) => "got_it",
            (K::RevengeGame, false) => "denied",
            (K::CoachReturn | K::OldTeam, true) => "won",
            (K::CoachReturn | K::OldTeam, false) => "lost",
            (K::TrophyGame, true) => "kept",
            (K::TrophyGame, false) => "lost",
            (K::Rematch, true) => "avenged",
            (K::Rematch, false) => "again",
            (K::BulletinBoard, true) => "backed_it",
            (_, _) => "ate_it",
        };
        let text = format!("{} {} {} {score}.", tname(w, t), if won { "beat" } else { "lost to" }, tname(w, o));
        fact(w, id, text, 4.0);
        close(w, id, ending, None);
    }
}

/// Who took the snaps at quarterback: a battle's first start, a benching,
/// the starter back from injury.
fn snaps(w: &mut World, t: TeamKey, stats: &GameStats, side: usize) {
    let year = w.year;
    let lines = &stats.players[side];
    let mut qbs: Vec<(u32, u32)> = lines
        .iter()
        .filter(|(_, l)| l.att > 0)
        .filter(|(id, _)| {
            crate::world::storylines::pi(w, **id).is_some_and(|i| w.people[i].player.position == Position::QB)
        })
        .map(|(id, l)| (*id, l.att))
        .collect();
    qbs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let Some(&(starter, att)) = qbs.first() else { return };
    let total: u32 = qbs.iter().map(|x| x.1).sum();
    let split = qbs.get(1).is_some_and(|x| x.1 * 10 >= total.max(1) * 35) && att * 10 < total * 65;
    let before = w.storylines.watch.starters.insert(super::tcode(t), starter);
    let name = |w: &World, id: u32| crate::world::storylines::pi(w, id).map_or_else(String::new, |i| pname(w, i));
    // A battle's first start.
    if let Some(id) = open_id(w, K::QbBattle, s_season(t, year)) {
        let s = w.storylines.get(id).cloned();
        if let Some(s) = s {
            let (lead, rival) = (s.lead_player(), s.rival_player());
            let ending = if split {
                "split"
            } else if Some(starter) == lead {
                "kept_job"
            } else if Some(starter) == rival {
                "won_job"
            } else {
                "injury"
            };
            let (who, team) = (name(w, starter), tname(w, t));
            let text = match ending {
                "split" => {
                    let second = qbs.get(1).map_or_else(String::new, |x| name(w, x.0));
                    format!("{who} and {second} split the snaps at quarterback for {team}.")
                }
                "kept_job" => format!("{who} keeps the job at quarterback for {team}."),
                "won_job" => format!("{who} wins the job at quarterback for {team}."),
                _ => format!("{who} starts at quarterback for {team}."),
            };
            close(w, id, ending, Some(text));
        }
    }
    // A controversy answered by a benching.
    for (id, _) in open_of(w, K::QbControversy) {
        let Some(s) = w.storylines.get(id).cloned() else { continue };
        if s.team != Some(t) {
            continue;
        }
        if s.rival_player() == Some(starter) {
            let text = format!(
                "{} gets the start over {}.",
                name(w, starter),
                s.lead_player().map_or_else(String::new, |x| name(w, x))
            );
            close(w, id, "benched", Some(text));
        }
    }
    // The next man up: the starter back.
    for (id, _) in open_of(w, K::NextManUp) {
        let Some(s) = w.storylines.get(id).cloned() else { continue };
        if s.team != Some(t) {
            continue;
        }
        let (lead, rival) = (s.lead_player(), s.rival_player());
        let back = rival.is_some_and(|r| qbs.iter().any(|x| x.0 == r) || !hurt(w, r));
        if back {
            let (ending, text) = if Some(starter) == lead {
                let back = rival.map_or_else(String::new, |r| name(w, r));
                (
                    "kept_job",
                    format!(
                        "{} keeps the job at quarterback for {} with {back} healthy.",
                        name(w, starter),
                        tname(w, t)
                    ),
                )
            } else {
                ("gave_back", format!("{} is back under center for {}.", name(w, starter), tname(w, t)))
            };
            close(w, id, ending, Some(text));
        }
    }
    // The starter is hurt: the next man up.
    if let Some(prev) = before
        && prev != starter
        && hurt(w, prev)
        && on(w, t)
        && let Some(i) = crate::world::storylines::pi(w, starter)
    {
        let s = super::player_spot(w, K::NextManUp, i)
            .rival(Who::Player(prev))
            .fact(
                format!("{} starts at quarterback for {} with {} hurt.", name(w, starter), tname(w, t), name(w, prev)),
                4.0,
            )
            .public();
        tell(w, s);
    }
    // A freshman under center: a true freshman in college, a rookie in the
    // NFL, a ninth or tenth grader in high school.
    if let Some(i) = crate::world::storylines::pi(w, starter) {
        let p = &w.people[i];
        let young = match t {
            TeamKey::College(_) => p.college_seasons == 0,
            TeamKey::Nfl(_) => p.pro.as_ref().is_some_and(|c| c.first_year == year),
            TeamKey::Hs(_) => p.class_of - year >= 3,
            TeamKey::Other(_) => false,
        };
        let subject = s_player(starter);
        if young && open_id(w, K::FreshmanStarter, subject).is_none() {
            let s = super::player_spot(w, K::FreshmanStarter, i)
                .fact(format!("{} makes his first start at quarterback for {}.", pname(w, i), tname(w, t)), 4.0)
                .public();
            tell(w, s);
        }
    }
    for (id, _) in open_of(w, K::FreshmanStarter) {
        let Some(s) = w.storylines.get(id) else { continue };
        if s.team == Some(t) && s.lead_player() != Some(starter) && s.lead_player().is_some_and(|l| !hurt(w, l)) {
            let young = s.lead_player().map_or_else(String::new, |l| name(w, l));
            let text = format!("{} takes over from {young} at quarterback for {}.", name(w, starter), tname(w, t));
            close(w, id, "benched", Some(text));
        }
    }
    // A two-quarterback system shows itself.
    if split
        && open_id(w, K::TwoQb, s_season(t, year)).is_none()
        && matches!(t, TeamKey::College(_) | TeamKey::Hs(_))
        && let Some(&(b, _)) = qbs.get(1)
    {
        let ia = crate::world::storylines::pi(w, starter);
        let ib = crate::world::storylines::pi(w, b);
        if let (Some(ia), Some(ib)) = (ia, ib) {
            let s = super::team_spot(w, K::TwoQb, t, s_season(t, year))
                .lead(Who::Player(starter))
                .rival(Who::Player(b))
                .fact(
                    format!(
                        "{} and {} split the snaps at quarterback for {}.",
                        pname(w, ia),
                        pname(w, ib),
                        tname(w, t)
                    ),
                    3.0,
                );
            tell(w, s);
        }
    }
}

/// Whether a player is out hurt.
pub(crate) fn hurt(w: &World, id: u32) -> bool {
    let Some(i) = crate::world::storylines::pi(w, id) else { return false };
    let p = &w.people[i];
    p.pro.as_ref().is_some_and(|c| c.out_weeks > 0)
        || w.college
            .as_ref()
            .is_some_and(|c| c.out.get(&id).is_some_and(|why| why.contains("injur") || why.contains("hurt")))
}

/// A kicker's tries: a slump, its end, his job.
fn kicks(w: &mut World, t: TeamKey, stats: &GameStats, side: usize) {
    let kicked: Vec<(u32, u16, u16)> = stats.players[side]
        .iter()
        .map(|(id, l)| (*id, l.fg_made.iter().sum::<u32>() as u16, l.fg_att.iter().sum::<u32>() as u16))
        .filter(|x| x.2 > 0)
        .collect();
    for &(id, made, att) in &kicked {
        let k = w.storylines.watch.kicks.entry(id).or_insert([0; 4]);
        k[0] += made;
        k[1] += att;
        // The last few games' tries, eased so a good stretch shows.
        k[2] = k[2] / 2 + made;
        k[3] = k[3] / 2 + att;
        let [m, a, rm, ra] = *k;
        let Some(i) = crate::world::storylines::pi(w, id) else { continue };
        let slump = a >= 7 && (m as f32) < 0.65 * a as f32;
        if slump {
            let s = super::player_spot(w, K::KickerSlump, i)
                .nums([m as i32, a as i32, 0, 0])
                .fact(format!("{} has made {m} of {a} field goals this season.", pname(w, i)), 3.0);
            tell(w, s);
        }
        if let Some(sid) = open_id(w, K::KickerSlump, s_player(id))
            && ra >= 4
            && rm >= ra
            && w.storylines.get(sid).is_some_and(|s| s.age(w.today) >= 14)
        {
            close(w, sid, "fixed", Some(format!("{} has found it again: everything through lately.", pname(w, i))));
        }
    }
    // Another kicker took the kicks: the slumping one lost his job.
    for (sid, subject) in open_of(w, K::KickerSlump) {
        let id = subject as u32;
        let mine = w.storylines.get(sid).is_some_and(|s| s.team == Some(t));
        if mine && !kicked.is_empty() && !kicked.iter().any(|x| x.0 == id) {
            close(w, sid, "lost_job", None);
        }
    }
}

/// The chases a game moves: a school or league record.
fn chases(w: &mut World, t: TeamKey, stats: &GameStats, side: usize) {
    let mut open = open_of(w, K::RecordChase);
    open.extend(open_of(w, K::CareerRecord));
    for (sid, subject) in open {
        let id = subject as u32;
        if !stats.players[side].contains_key(&id) {
            continue;
        }
        let Some(s) = w.storylines.get(sid).cloned() else { continue };
        if s.team != Some(t) {
            continue;
        }
        let now = super::weekly::record_value(w, id, s.nums[2]);
        if now > s.nums[1] as f64 {
            let Some(i) = crate::world::storylines::pi(w, id) else { continue };
            let text = format!(
                "{} breaks the {} record with {} {}.",
                pname(w, i),
                super::weekly::record_scope(s.nums[2]),
                crate::world::folk::facts::thousands(now.round() as u32),
                super::weekly::stat_name(s.nums[2])
            );
            close(w, sid, "broke_it", Some(text));
        } else if let Some(k) = w.storylines.idx(sid) {
            w.storylines.open[k].nums[0] = now.round() as i32;
        }
    }
}

/// A coach's milestone win.
fn milestone(w: &mut World, t: TeamKey) {
    let Some(c) = head(w, t) else { return };
    let wins = super::career_wins(w, c);
    for kind in [K::CoachMilestone, K::YourMilestone] {
        if let Some(id) = open_id(w, kind, s_coach(c))
            && w.storylines.get(id).is_some_and(|s| wins as i32 >= s.nums[0])
        {
            let text = format!("{} gets career win number {wins}.", super::cname(w, c));
            close(w, id, "reached", Some(text));
        }
    }
}

/// Six wins for a bowl, or not.
fn bowl_chase(w: &mut World, t: TeamKey) {
    let year = w.year;
    let Some(id) = open_id(w, K::BowlChase, s_season(t, year)) else { return };
    let (wins, losses) = crate::world::storylines::tell::record_now(w, t);
    if wins >= 6 {
        close(w, id, "made_it", Some(format!("{} gets to six wins and a bowl.", tname(w, t))));
    } else if losses >= 7 {
        close(w, id, "fell_short", Some(format!("{} can't get to six wins.", tname(w, t))));
    }
}

/// A team's quarterback who started its last game.
pub(crate) fn last_starter(w: &World, t: TeamKey) -> Option<u32> {
    w.storylines.watch.starters.get(&super::tcode(t)).copied()
}
