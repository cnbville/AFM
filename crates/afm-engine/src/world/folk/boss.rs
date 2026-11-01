//! Bosses as people, their goals and your meetings with them (people topic
//! 12; final build lane 6). Every boss is a person drawn from the team's
//! cast (the athletic director and the president at a college, the owner
//! and his president at an NFL club, the principal, the athletic director
//! and the superintendent at a high school) with trust in you that moves
//! with results against his goals, how you handle trouble, money and the
//! media, and how you treat him.
//!
//! Goals are set in meetings on each level's real calendar: your first
//! meeting, then before each season (the college AD in summer, the owner
//! after the draft, the principal before school starts), a check-in at
//! midseason and a review after the last game. You negotiate them: argue
//! for a lower bar, take a higher one for more time and money. He tells you
//! in words, never as a score. What you can ask for: staff money, a
//! facility project, recruiting money, NIL money, roster spots, an extension.
//!
//! His trust feeds your own reviews (`review_push`) and a big booster's
//! patience (`booster_push`). Sizes are Claude's picks (proposed, not
//! confirmed).

use super::cast::{self, Cast, CastRef, Role, Want};
use super::scene::{self, Doc, DocRow, Kind, Pick, Scene, Who};
use super::voice::{Facts, Speaker};
use super::{Team, facts};
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GoalKind {
    /// Win at least this many games.
    Wins(u8),
    /// A bowl (college), the playoffs (NFL and high school).
    Postseason,
    /// A conference or division title.
    Title,
    BeatRival,
    /// A recruiting class in the top this many of the conference.
    Recruiting(u8),
    /// No arrests, no NCAA cases, no suspensions that reach the news.
    Clean,
    /// Grades: no player academically ineligible (APR at a college).
    Grades,
    /// Develop the young players: a patient owner's rebuild.
    Develop,
    /// Fill the stadium.
    Crowds,
}

impl GoalKind {
    pub fn words(self, level: Level) -> String {
        match self {
            GoalKind::Wins(n) => format!("win at least {n} {}", super::noun(n, "game")),
            GoalKind::Postseason => match level {
                Level::College => "go to a bowl".into(),
                _ => "make the playoffs".into(),
            },
            GoalKind::Title => match level {
                Level::Nfl => "win the division".into(),
                Level::HighSchool => "win the district".into(),
                _ => "win the conference".into(),
            },
            GoalKind::BeatRival => "beat the rival".into(),
            GoalKind::Recruiting(n) => format!("sign a top-{n} class in the conference"),
            GoalKind::Clean => "keep the program out of trouble".into(),
            GoalKind::Grades => match level {
                Level::College => "keep the academic progress rate up".into(),
                _ => "keep the players eligible: two at most ruled out for grades".into(),
            },
            GoalKind::Develop => "develop the young players".into(),
            GoalKind::Crowds => "fill the stadium".into(),
        }
    }
}

/// A goal your boss set, and how it ended.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Goal {
    pub kind: GoalKind,
    pub season: i32,
    /// You argued it down (-1), took it as set (0) or raised it (+1).
    pub deal: i8,
    pub met: Option<bool>,
    pub set_on: Date,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AskKind {
    StaffMoney,
    Facility,
    RecruitingMoney,
    NilMoney,
    RosterSpots,
    Extension,
}

impl AskKind {
    pub fn words(self) -> &'static str {
        match self {
            AskKind::StaffMoney => "more money for your staff",
            AskKind::Facility => "a facility project",
            AskKind::RecruitingMoney => "a bigger recruiting budget",
            AskKind::NilMoney => "more NIL and revenue-sharing money",
            AskKind::RosterSpots => "practice-squad money",
            AskKind::Extension => "an extension",
        }
    }
    fn code(self) -> u32 {
        match self {
            AskKind::StaffMoney => 0,
            AskKind::Facility => 1,
            AskKind::RecruitingMoney => 2,
            AskKind::NilMoney => 3,
            AskKind::RosterSpots => 4,
            AskKind::Extension => 5,
        }
    }
    fn from_code(c: u32) -> AskKind {
        match c {
            1 => AskKind::Facility,
            2 => AskKind::RecruitingMoney,
            3 => AskKind::NilMoney,
            4 => AskKind::RosterSpots,
            5 => AskKind::Extension,
            _ => AskKind::StaffMoney,
        }
    }
    fn at(self, level: Level) -> bool {
        match self {
            AskKind::NilMoney | AskKind::RecruitingMoney => level == Level::College,
            AskKind::RosterSpots => level == Level::Nfl,
            _ => true,
        }
    }
}

/// Something you asked for, and his answer.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Ask {
    pub kind: AskKind,
    pub date: Date,
    pub granted: bool,
    pub words: String,
}

/// Your boss at this team.
pub fn boss(w: &World, team: Team) -> Cast {
    cast::get(w, CastRef::new(team, cast::boss_role(team), 0))
}

/// His trust in you, 0-1.
pub fn trust(w: &World, team: Team) -> f32 {
    let b = boss(w, team);
    w.folk.mine.trust_of(b.key()).unwrap_or(0.5)
}

/// How he feels about you, in words (no meters).
pub fn trust_words(t: f32) -> &'static str {
    match t {
        x if x > 0.75 => "He backs you completely and says so in public.",
        x if x > 0.6 => "He likes what he sees.",
        x if x > 0.45 => "He's waiting to see.",
        x if x > 0.3 => "He has doubts, and the people around him know it.",
        _ => "He has lost faith in you. The board is asking him questions.",
    }
}

/// The goals that fit the team and the boss: his situation read from the
/// team's last seasons, its level, and his own wants (topic 12).
fn draft_goals(w: &World, team: Team) -> Vec<GoalKind> {
    let level = team.level();
    let last = facts::seasons(w, team, 3);
    let games = last.last().map_or(12, |s| s.wins + s.losses).max(10) as f64;
    let avg_wp = if last.is_empty() {
        0.5
    } else {
        last.iter().map(|s| s.wins as f64 / (s.wins + s.losses).max(1) as f64).sum::<f64>() / last.len() as f64
    };
    let b = boss(w, team);
    let mut v = Vec::new();
    // Wins: a little above the last three years' average, by his patience.
    let bar = (avg_wp + 0.08 - 0.01 * (b.patience - 10.0)).clamp(0.25, 0.85);
    let need = (bar * games).round() as u8;
    match level {
        Level::Nfl => {
            let rebuild = matches!(team, Team::Nfl(t) if w.clubs.get(t).is_some_and(|c| c.office.mode == crate::world::pro::office::Mode::Rebuild));
            if rebuild {
                v.push(GoalKind::Develop);
                v.push(GoalKind::Wins(need.clamp(4, 9)));
            } else {
                v.push(GoalKind::Postseason);
                v.push(GoalKind::Wins(need.clamp(8, 13)));
            }
        }
        Level::College => {
            v.push(GoalKind::Wins(need.clamp(4, 11)));
            if avg_wp > 0.5 {
                v.push(GoalKind::Postseason);
            }
            if avg_wp > 0.7 {
                v.push(GoalKind::Title);
            }
            v.push(GoalKind::Recruiting(if avg_wp > 0.6 { 3 } else { 6 }));
            v.push(GoalKind::Grades);
        }
        _ => {
            v.push(GoalKind::Wins(need.clamp(3, 10)));
            if avg_wp > 0.55 {
                v.push(GoalKind::Postseason);
            }
            v.push(GoalKind::Grades);
        }
    }
    if b.honesty >= 12.0 || level == Level::HighSchool {
        v.push(GoalKind::Clean);
    }
    if facts::rival(w, team).is_some()
        && (b.ego >= 12.0
            || cast::all(w, team, Role::Booster)
                .iter()
                .any(|c| c.booster.as_ref().is_some_and(|x| x.wants.contains(&Want::BeatRival))))
    {
        v.push(GoalKind::BeatRival);
    }
    if level != Level::HighSchool && b.tight >= 13.0 {
        v.push(GoalKind::Crowds);
    }
    v.truncate(5);
    v
}

/// The season the goals are for: the next one before kickoff.
fn goal_season(w: &World, team: Team) -> i32 {
    let kickoff = match team.level() {
        Level::Nfl => w.dates.nfl_kickoff,
        _ => w.dates.fall_camp,
    };
    if w.today <= kickoff.add(30) { w.year } else { w.year + 1 }
}

fn meeting_open(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let level = team.level();
    let b = boss(w, team);
    s.who.push(Who::Cast(b.r.token()));
    let spk = Speaker::cast(w, &b);
    let mut f = super::takeover::base_facts(w, team);
    f.set("boss", b.first().to_string());
    let mut pen = scene::pen(w, s, 1);
    // s.c: 0 the first meeting or a preseason one, 1 midseason, 2 the review.
    match s.c {
        1 => {
            let (wn, l) = facts::record_now(w, team);
            f.set("record", format!("{wn}-{l}"));
            s.narrate(format!("{} wants a word at the midpoint. You're {wn}-{l}.", b.link()));
            let line = pen.line(&["boss.mid.open"], &spk, &f);
            s.them(
                &b.name,
                if line.is_empty() { format!("{wn} and {l}. Talk to me. What do you see?") } else { line },
                Some(boss_tell(&b, trust(w, team))),
            );
            s.pick(
                30,
                "\"We're ahead of where I thought. Here's why.\"",
                "Tell him straight",
                "calm",
                "Honest about the good; he likes being told.",
            );
            s.pick(
                31,
                "\"I'm not happy either. Here's what changes.\"",
                "Take the blame",
                "firm",
                "Owning it builds trust when it's true.",
            );
            s.pick(
                32,
                "\"The injuries have killed us.\"",
                "Deflect",
                "calm",
                "He hears an excuse unless it's plainly true.",
            );
        }
        2 => review_open(w, s, team, &b, &mut pen, &mut f),
        3 => {
            // You asked for the meeting: he gives you the time and waits.
            s.narrate(format!(
                "{} {}",
                b.link(),
                if b.patience <= 7.0 {
                    format!("checks {} watch as you sit down. You asked for this one.", b.his())
                } else {
                    format!("closes the door behind you. You asked for the time; {} gave it.", b.he())
                }
            ));
            s.them(
                &b.name,
                if b.warmth >= 13.0 { "Come on in. What's on your mind?" } else { "You wanted to see me." },
                Some(boss_tell(&b, trust(w, team))),
            );
            add_ask_picks(w, s, team);
        }
        _ => {
            let goals = draft_goals(w, team);
            let season = goal_season(w, team);
            s.narrate(format!(
                "{} {}",
                b.link(),
                match level {
                    Level::Nfl => "has his notes for the season ahead.",
                    Level::College => "goes through what he expects this season.",
                    _ => "goes through what the school expects of the program.",
                }
            ));
            let line = pen.line(&["boss.goals.open", "?boss.goals.point"], &spk, &f);
            s.them(
                &b.name,
                if line.is_empty() { "Here's what I need from you this year.".into() } else { line },
                Some(boss_tell(&b, trust(w, team))),
            );
            let mut doc = Doc {
                title: format!("{}'s goals for {season}", b.first()),
                note: "Set in this meeting. You can argue them down or take on more.".into(),
                cols: vec!["Goal".into(), "Why he wants it".into()],
                rows: Vec::new(),
            };
            for g in &goals {
                doc.rows.push(DocRow {
                    pid: None,
                    cells: vec![super::takeover::cap_first(&g.words(level)), why_goal(w, team, *g)],
                });
            }
            s.docs.push(doc);
            w.folk.goals.retain(|g| g.season != season);
            for g in goals {
                w.folk.goals.push(Goal { kind: g, season, deal: 0, met: None, set_on: w.today });
            }
            s.pick(
                20,
                "\"Done. I'd set them higher myself.\"",
                "Promise",
                "fiery",
                "He likes the ambition; the bar goes up and you'll be judged by it.",
            );
            s.pick(21, "\"Those are fair. We'll get there.\"", "Praise", "calm", "Goals as set.");
            s.pick(
                22,
                "\"The roster's young. Give me a year to build it.\"",
                "Tell him straight",
                "calm",
                "A lower bar now; a patient boss agrees, an impatient one remembers you asked.",
            );
        }
    }
    pen.put(w);
}

fn boss_tell(b: &Cast, t: f32) -> String {
    if t > 0.65 {
        format!(
            "{} leans back. {} is comfortable with you.",
            b.first(),
            b.he().to_string().to_uppercase().chars().next().map_or(String::new(), |c| c.to_string()) + &b.he()[1..]
        )
    } else if t < 0.35 {
        format!("{} doesn't sit down.", b.first())
    } else if b.temper >= 14.0 {
        format!("{} taps the table while you talk.", b.first())
    } else {
        format!("{} takes notes.", b.first())
    }
}

fn why_goal(w: &World, team: Team, g: GoalKind) -> String {
    let last = facts::seasons(w, team, 1);
    match g {
        GoalKind::Wins(_) => {
            last.last().map_or("The fans expect it.".into(), |s| format!("Last year was {}.", facts::season_words(s)))
        }
        GoalKind::Postseason => "The money and the fans follow the postseason.".into(),
        GoalKind::Title => "This program measures itself in titles.".into(),
        GoalKind::BeatRival => {
            facts::rival(w, team).map_or("It's the game people remember.".into(), |(r, game, a, b)| {
                if a + b > 0 {
                    format!("{} ({game}): the series is {a}-{b}.", r.name(w))
                } else {
                    format!("{} ({game}).", r.name(w))
                }
            })
        }
        GoalKind::Recruiting(_) => "The rankings are how the boosters judge a staff in February.".into(),
        GoalKind::Clean => "One arrest in the paper is a week of phone calls for him.".into(),
        GoalKind::Grades => "The academic people report to the board.".into(),
        GoalKind::Develop => "A rebuild: he wants to see the young players grow.".into(),
        GoalKind::Crowds => "Empty seats are lost money, and he counts it.".into(),
    }
}

fn review_open(w: &mut World, s: &mut Scene, team: Team, b: &Cast, pen: &mut super::voice::Pen, f: &mut Facts) {
    let level = team.level();
    let season = s.b as i32;
    let mut doc = Doc {
        title: format!("The {season} review"),
        note: "What he set, and how it ended.".into(),
        cols: vec!["Goal".into(), "How it ended".into()],
        rows: Vec::new(),
    };
    let mut met = 0;
    let mut total = 0;
    let goals: Vec<Goal> = w.folk.goals.iter().filter(|g| g.season == season).cloned().collect();
    for g in &goals {
        let ok = goal_met(w, team, g.kind, season);
        total += 1;
        met += ok as u32;
        doc.rows.push(DocRow {
            pid: None,
            cells: vec![
                super::takeover::cap_first(&g.kind.words(level)),
                if ok { "Met".into() } else { "Missed".into() },
            ],
        });
    }
    let ended: Vec<(GoalKind, bool)> = goals.iter().map(|g| (g.kind, goal_met(w, team, g.kind, season))).collect();
    for g in w.folk.goals.iter_mut().filter(|g| g.season == season) {
        g.met = ended.iter().find(|x| x.0 == g.kind).map(|x| x.1);
    }
    s.docs.push(doc);
    let share = if total == 0 { 0.5 } else { met as f32 / total as f32 };
    w.folk.mine.nudge(b.key(), 0.25 * (share - 0.5));
    s.narrate(format!("{} has the season in front of {}: {met} of {total} goals met.", b.link(), b.him()));
    f.set("met", met.to_string());
    let key = if share >= 0.67 {
        "boss.review.good"
    } else if share >= 0.34 {
        "boss.review.mixed"
    } else {
        "boss.review.bad"
    };
    let spk = Speaker::cast(w, b);
    let line = pen.line(&[key], &spk, f);
    s.them(
        &b.name,
        if line.is_empty() {
            (if share >= 0.67 {
                "That's the year we talked about. Let's keep it going."
            } else if share >= 0.34 {
                "Some of it was what I wanted. Some of it wasn't."
            } else {
                "I'm going to be honest with you: this isn't what we talked about."
            })
            .to_string()
        } else {
            line
        },
        Some(trust_words(trust(w, team)).to_string()),
    );
    add_ask_picks(w, s, team);
}

/// Whether a goal was met in a season, from the world's record.
pub fn goal_met(w: &World, team: Team, k: GoalKind, season: i32) -> bool {
    let s = facts::seasons(w, team, 6).into_iter().find(|s| s.year == season);
    let Some(s) = s else { return false };
    match k {
        GoalKind::Wins(n) => s.wins >= n as u32,
        GoalKind::Postseason => match team {
            Team::College(_) => s.wins >= 6,
            Team::Nfl(_) | Team::Other(_) => !s.note.is_empty(),
            // Your own school knows whether it made the field; the record
            // stands in for anyone else's.
            Team::High(h) => {
                let desk = &w.front_office.school;
                if desk.school == Some(h) && desk.year == season { desk.in_playoffs } else { s.wins >= 7 }
            }
        },
        GoalKind::Title => s.note.contains("champion") || s.note.contains("won"),
        GoalKind::BeatRival => want_met(w, Want::BeatRival),
        GoalKind::Recruiting(n) => match team {
            Team::College(p) => {
                let conf = &w.programs[p].conference;
                let mut scores: Vec<(usize, f64)> = w
                    .programs
                    .iter()
                    .enumerate()
                    .filter(|(_, q)| &q.conference == conf)
                    .filter_map(|(k, q)| q.class_scores.iter().find(|x| x.0 == season).map(|x| (k, x.1)))
                    .collect();
                scores.sort_by(|a, b| b.1.total_cmp(&a.1));
                scores.iter().position(|x| x.0 == p).is_some_and(|r| r < n as usize)
            }
            _ => true,
        },
        GoalKind::Clean => !w
            .folk
            .courts
            .closed
            .iter()
            .chain(w.folk.courts.open.iter())
            .any(|c| c.opened.year() == season && team.roster(w).iter().any(|&i| w.people[i].player.id == c.id)),
        GoalKind::Grades => {
            team.roster(w)
                .iter()
                .filter(|&&i| {
                    let c = &w.people[i].cal;
                    c.academic_sit || c.fall_ineligible || c.hs_grade_out > 0
                })
                .count()
                <= 2
        }
        GoalKind::Develop => true,
        GoalKind::Crowds => s.wins * 2 >= s.losses * 2,
    }
}

/// Whether a booster's want is met now (promises, takeover item 14).
pub fn want_met(w: &World, want: Want) -> bool {
    let Some(team) = Team::seat(w) else { return true };
    match want {
        Want::BeatRival => {
            let Some((r, ..)) = facts::rival(w, team) else { return true };
            let (_, ri) = r.parts();
            let (_, me) = team.parts();
            w.seat_games.iter().rev().find(|g| g.teams.contains(&ri) && g.teams.contains(&me)).is_some_and(|g| {
                let side = if g.teams[0] == me { 0 } else { 1 };
                g.stats.score[side] > g.stats.score[1 - side]
            })
        }
        Want::Clean => {
            w.folk.courts.open.iter().all(|c| !team.roster(w).iter().any(|&i| w.people[i].player.id == c.id))
        }
        Want::WinNow => facts::seasons(w, team, 1).last().is_some_and(|s| s.wins > s.losses),
        Want::Access | Want::SayInHires | Want::SayOnPlayer | Want::OldSchool | Want::Building => true,
        Want::NilDeals => match team {
            Team::College(p) => w.programs[p].collective.boosters() > 0.0,
            _ => true,
        },
    }
}

fn add_ask_picks(w: &World, s: &mut Scene, team: Team) {
    let level = team.level();
    for k in [
        AskKind::StaffMoney,
        AskKind::Facility,
        AskKind::RecruitingMoney,
        AskKind::NilMoney,
        AskKind::RosterSpots,
        AskKind::Extension,
    ] {
        if !k.at(level) || w.folk.asks.iter().any(|a| a.kind == k && a.date.days_to(w.today) < 300) {
            continue;
        }
        s.pick(
            40 + k.code(),
            format!("Ask for {}", k.words()),
            "Ask",
            "calm",
            "His answer reads his money, his trust in you and his nature.",
        );
    }
    s.pick(49, "Nothing today. Thank him for his time.", "Praise", "warm", "Ends the meeting well.");
}

fn meeting_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let b = boss(w, team);
    s.you(&scene::your_name(w), picked.label.clone());
    let season = goal_season(w, team);
    match code {
        20 => {
            for g in w.folk.goals.iter_mut().filter(|g| g.season == season) {
                g.deal = 1;
                if let GoalKind::Wins(n) = g.kind {
                    g.kind = GoalKind::Wins(n + 1);
                }
            }
            w.folk.mine.nudge(b.key(), 0.06);
            w.folk.mine.expectation = (w.folk.mine.expectation + 0.15).min(1.0);
            s.them(&b.name, "Good. That's the coach I hired.", None);
            s.outcome.push("You took on a higher bar.".into());
            add_ask_picks(w, s, team);
            return;
        }
        21 => {
            w.folk.mine.nudge(b.key(), 0.02);
            s.them(&b.name, "Good. Then we understand each other.", None);
            s.outcome.push("You agreed to his goals as set.".into());
            add_ask_picks(w, s, team);
            return;
        }
        22 => {
            let patient = b.patience >= 11.0;
            for g in w.folk.goals.iter_mut().filter(|g| g.season == season) {
                g.deal = -1;
                if patient && let GoalKind::Wins(n) = g.kind {
                    g.kind = GoalKind::Wins(n.saturating_sub(2).max(2));
                }
            }
            w.folk.goals.retain(|g| {
                !(g.season == season && patient && matches!(g.kind, GoalKind::Title | GoalKind::Postseason))
            });
            w.folk.mine.nudge(b.key(), if patient { 0.0 } else { -0.05 });
            s.them(
                &b.name,
                if patient {
                    "All right. A year to build. But I want to see it moving."
                } else {
                    "I didn't hire you to ask for time."
                },
                None,
            );
            s.outcome.push(if patient {
                "He lowered the bar for this year.".to_string()
            } else {
                "He kept the bar where it was, and noted that you asked.".to_string()
            });
            add_ask_picks(w, s, team);
            return;
        }
        30 | 31 => {
            w.folk.mine.nudge(b.key(), if code == 31 { 0.04 } else { 0.02 });
            s.them(&b.name, "Okay. Keep me posted. I don't want to read it in the paper first.", None);
        }
        32 => {
            let hurt = team
                .roster(w)
                .iter()
                .filter(|&&i| {
                    w.people[i].pro.as_ref().is_some_and(|c| c.out_weeks > 0)
                        || super::rules::sitting(w, w.people[i].player.id, w.today)
                })
                .count();
            let true_excuse = hurt >= 5;
            w.folk.mine.nudge(b.key(), if true_excuse { 0.01 } else { -0.04 });
            s.them(
                &b.name,
                if true_excuse { "I know. I've seen the injury list." } else { "Everybody has injuries." },
                None,
            );
        }
        40..=45 => {
            ask(w, s, team, AskKind::from_code(code - 40));
            add_ask_picks(w, s, team);
            if s.picks.len() <= 1 || s.step >= 3 {
                s.end();
            }
            return;
        }
        _ => {}
    }
    s.end();
}

/// You ask your boss for a meeting (topic 12): once a month at most; the
/// ask itself costs nothing but an hour.
pub fn request_meeting(w: &mut World) -> Result<u32, String> {
    let team = Team::seat(w).ok_or("You have no job.")?;
    let today = w.today;
    if w.folk.scenes.iter().any(|s| s.kind == Kind::BossMeeting && s.c == 3 && s.date.days_to(today) < 30) {
        return Err("You saw him less than a month ago. Asking again this soon reads as needy.".into());
    }
    if !super::talks::spend(w, 1.0) {
        return Err("No hours left this week.".into());
    }
    let mut s = Scene::new(Kind::BossMeeting, today, "You asked for a meeting");
    s.c = 3;
    s.team = Some(team);
    s.stops = false;
    s.place = match team.level() {
        Level::Nfl => "The owner's suite".into(),
        Level::College => "The athletic director's office".into(),
        _ => "The principal's office".into(),
    };
    Ok(scene::add(w, s))
}

/// You ask for something (topic 12): his answer reads his money, his trust
/// in you and his personality.
fn ask(w: &mut World, s: &mut Scene, team: Team, k: AskKind) {
    let b = boss(w, team);
    let t = trust(w, team) as f64;
    let rich = match team {
        Team::College(p) => (crate::world::staff::program_budget(&w.programs[p]) / 80.0e6).min(1.5),
        Team::Nfl(_) => 1.0,
        Team::Other(_) => 0.4,
        Team::High(h) => w.high_schools[h].resources,
    };
    let goals_met = w.folk.goals.iter().filter(|g| g.met == Some(true)).count() as f64;
    let p = (0.15 + 0.6 * (t - 0.4) + 0.2 * rich + 0.06 * goals_met - 0.03 * (b.tight - 10.0)
        + 0.02 * (b.warmth - 10.0)
        - if k == AskKind::Extension && w.folk.mine.since.is_some_and(|d| d.days_to(w.today) < 500) {
            0.3
        } else {
            0.0
        })
    .clamp(0.03, 0.92);
    let yes = super::roll(w, 0xA5C0 ^ w.today.0 as u64, k.code() as u64) < p;
    let words = if yes {
        match k {
            AskKind::StaffMoney => {
                w.folk.staff.pool_bonus += 0.1;
                "Yes. Ten percent more for the staff pool, starting now."
            }
            AskKind::Facility => {
                if let Team::College(pg) = team {
                    w.programs[pg].facilities = (w.programs[pg].facilities + 0.03).min(1.0);
                }
                "Yes. We'll break ground after the season."
            }
            AskKind::RecruitingMoney => {
                w.folk.staff.recruit_bonus += 0.1;
                "Yes. More travel money for the staff."
            }
            AskKind::NilMoney => {
                if let Team::College(pg) = team {
                    w.programs[pg].collective.base *= 1.08;
                }
                "I'll get the collective in a room. You'll see it."
            }
            AskKind::RosterSpots => "Yes. Keep the extra practice-squad guys.",
            AskKind::Extension => {
                let you = w.career.as_ref().map(|c| c.coach);
                if let Some(c) = you
                    && let Some(k) = w.coaches[c].contract.as_mut()
                {
                    k.through += 2;
                    k.salary *= 1.12;
                }
                "Done. Two more years, and a raise."
            }
        }
    } else {
        match k {
            AskKind::Extension => "Not yet. Win and we'll talk.",
            AskKind::Facility => "Not this year. The money's committed.",
            _ => "I can't do it right now.",
        }
    };
    w.folk.mine.nudge(b.key(), if yes { -0.01 } else { -0.005 });
    s.them(&b.name, words, None);
    s.outcome.push(format!("You asked for {}: {}.", k.words(), if yes { "granted" } else { "no" }));
    w.folk.asks.push(Ask { kind: k, date: w.today, granted: yes, words: words.into() });
    if w.folk.asks.len() > 40 {
        w.folk.asks.remove(0);
    }
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    meeting_open(w, s);
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    meeting_answer(w, s, code, picked);
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else { return };
    let b = boss(w, team);
    w.folk.mine.nudge(b.key(), -0.06);
    s.handed = Some("nobody: you sent word you couldn't make it".into());
    s.outcome.push(format!(
        "You skipped the meeting with {}. {} didn't forget it.",
        b.name,
        super::takeover::cap_first(b.he())
    ));
}

/// Your review read by the coaching carousel (`staff::college_review`): his
/// trust moves the read a little, up to about a tenth of a bad season
/// either way (proposed, not confirmed).
pub fn review_push(w: &World, c: usize) -> f64 {
    if w.career.as_ref().map(|x| x.coach) != Some(c) {
        return 0.0;
    }
    let Some(team) = Team::seat(w) else { return 0.0 };
    let t = trust(w, team) as f64;
    let missed = w.folk.goals.iter().filter(|g| g.season == w.year - 1 && g.met == Some(false)).count() as f64;
    let met = w.folk.goals.iter().filter(|g| g.season == w.year - 1 && g.met == Some(true)).count() as f64;
    (0.1 * (0.5 - t) + 0.015 * (missed - met)).clamp(-0.1, 0.1)
}

/// A big booster's patience with you (read by the collective's yearly
/// update for your program): low trust and broken promises raise the
/// chance he walks away (up to +6 points).
pub fn booster_push(w: &World, p: usize) -> f64 {
    if Team::seat(w) != Some(Team::College(p)) {
        return 0.0;
    }
    let team = Team::College(p);
    let Some(big) = cast::all(w, team, Role::Booster).into_iter().find(|c| c.booster.as_ref().is_some_and(|b| b.big))
    else {
        return 0.0;
    };
    let t = w.folk.mine.trust_of(big.key()).unwrap_or(0.5) as f64;
    let broken = w
        .folk
        .promises
        .iter()
        .filter(|x| matches!(x.to, super::talks::PromiseTo::Cast(r, _) if r == big.r) && x.kept == Some(false))
        .count() as f64;
    (0.08 * (0.5 - t) + 0.03 * broken).clamp(-0.02, 0.06)
}

/// The meetings on the calendar: before the season, at midseason and after
/// the last game.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let Some(since) = w.folk.mine.since else { return };
    if since.days_to(d) < 20 {
        return;
    }
    let (pre, mid, post) = match team.level() {
        Level::Nfl => (
            w.dates.minicamp.1.add(2),
            w.dates.nfl_weeks.get(8).copied().unwrap_or(w.dates.nfl_kickoff.add(60)),
            w.dates.black_monday.add(2),
        ),
        Level::College => (
            w.dates.fall_camp.add(-10),
            w.dates.cfb_weeks.get(6).copied().unwrap_or(w.dates.fall_camp.add(70)).add(2),
            w.dates.bowl_season.1.add(4),
        ),
        _ => (w.dates.hs_school_start.add(-12), w.dates.hs_school_start.add(45), w.dates.december_graduation.add(-5)),
    };
    // Each meeting goes on the calendar the day before, so its morning
    // stops for you; set on the day itself, the end of that day would hand
    // it to your staff before you saw it.
    let when = d.add(1);
    let which = if when == pre {
        Some((0, "The season's goals"))
    } else if when == mid {
        Some((1, "The midseason check-in"))
    } else if when == post {
        Some((2, "The season review"))
    } else {
        None
    };
    if let Some((c, title)) = which {
        let mut s = Scene::new(Kind::BossMeeting, when, title);
        s.c = c;
        s.b = if c == 2 { w.year as u64 } else { 0 };
        if c == 2 && team.level() == Level::Nfl {
            s.b = (w.year - 1).max(0) as u64;
        }
        s.team = Some(team);
        s.stops = c != 1;
        s.place = match team.level() {
            Level::Nfl => "The owner's suite".into(),
            Level::College => "The athletic director's office".into(),
            _ => "The principal's office".into(),
        };
        scene::add(w, s);
    }
    // A boss hears about trouble in the papers (topic 12): trust slips when
    // your players' cases open.
    if d.weekday() == Weekday::Thu {
        let ids: Vec<u32> = team.roster(w).iter().map(|&i| w.people[i].player.id).collect();
        let fresh = w.folk.courts.open.iter().filter(|c| c.opened.days_to(d) < 7 && ids.contains(&c.id)).count();
        if fresh > 0 {
            let b = boss(w, team);
            w.folk.mine.nudge(b.key(), -0.03 * fresh as f32);
            let _ = Loudness::Small;
        }
    }
}
