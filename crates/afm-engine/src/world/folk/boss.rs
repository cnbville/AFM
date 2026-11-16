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
use super::claims;
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
    /// A season term you bargained (lane N2; `terms.rs`): a rank, wins, a
    /// postseason, a title, a player's season.
    Measure(super::usage::Usage),
    /// A recruiting goal from the list of 63 (Ground lane G6,
    /// `ground::campaign::goals`): the one or two he holds you to in his
    /// talk after signing day.
    Campaign(crate::world::ground::campaign::goals::Aim),
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
            // A season term as a goal, said like the others ("reach at
            // least 9 wins this season").
            GoalKind::Measure(u) => {
                let s = super::terms::words(&u);
                match u.what {
                    super::usage::What::Honor => s.replacen("makes ", "make ", 1),
                    super::usage::What::Title => format!("win {s}"),
                    super::usage::What::Rank => format!("finish {s}"),
                    _ => format!("reach {s}"),
                }
            }
            GoalKind::Campaign(a) => {
                let d = a.def();
                let name = d.name.replace("{n}", &a.n.to_string());
                let mut c = name.chars();
                let name = c.next().map_or(String::new(), |f| f.to_lowercase().collect::<String>() + c.as_str());
                format!("meet the class goal you agreed: {name}")
            }
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
    /// A bar you put up yourself in a talk (live talks v3, `folk::asked`):
    /// the goals meeting keeps it in place of its own goal of that kind.
    /// False in a file from before it.
    #[serde(default)]
    pub talk: bool,
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
    pub(crate) fn code(self) -> u32 {
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
    pub fn at(self, level: Level) -> bool {
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

/// How the people you answer to feel about you, in a few words: one scale
/// for the boss page, the Home card and the school desk (bug 32).
pub fn trust_short(t: f32) -> &'static str {
    match t {
        t if t >= 0.75 => "Firmly behind you",
        t if t >= 0.6 => "Pleased with you",
        t if t >= 0.45 => "Watching how it goes",
        t if t >= 0.3 => "Losing patience",
        _ => "Wants a change",
    }
}

/// How your boss feels about you, as a sentence on the same scale, with
/// the boss's own pronoun.
pub fn trust_words(t: f32, she: bool) -> String {
    let (he, him) = if she { ("She", "her") } else { ("He", "him") };
    match t {
        t if t >= 0.75 => format!("{he} is firmly behind you and says so in public."),
        t if t >= 0.6 => format!("{he} is pleased with you."),
        t if t >= 0.45 => format!("{he} is watching how it goes."),
        t if t >= 0.3 => format!("{he} is losing patience, and the people around {him} know it."),
        _ => format!("{he} wants a change. The people above {him} are asking questions."),
    }
}

/// Where the season stands against the bar he set (talk facts item 15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    /// On course for a win or more over his bar.
    Ahead,
    /// Within a win of it either way.
    On,
    /// On course to miss it by a win or more.
    Behind,
}

impl Pace {
    /// The word the bank part is named by (`boss.mid.reply.ahead`).
    pub fn key(self) -> &'static str {
        match self {
            Pace::Ahead => "ahead",
            Pace::On => "on",
            Pace::Behind => "behind",
        }
    }
}

/// The record is ahead or behind his bar when the wins it is on course
/// for are this many over or under it. Claude's size, proposed, not
/// confirmed.
pub const PACE_BAND: f64 = 1.0;

/// How far an answer to a season review with a miss moves his trust: owning
/// the miss or asking what he needs to see, and pointing to a goal that was
/// met. The midseason check-in's own two sizes. Cain's yes, 2026-11-13.
pub const REVIEW_OWN_TRUST: f32 = 0.02;
pub const REVIEW_POINT_TRUST: f32 = 0.04;

/// How the season is going against his win goal: the wins the record is on
/// course for over the whole schedule, against the bar he set (half the
/// games when he set none). None before a game has been played.
pub fn pace(w: &World, team: Team) -> Option<Pace> {
    let (wn, l) = facts::record_now(w, team);
    let played = (wn + l) as f64;
    if played == 0.0 {
        return None;
    }
    let total = (facts::team_games(w, team).len() as f64).max(played);
    let bar = w
        .folk
        .goals
        .iter()
        .filter(|g| g.season == w.year)
        .find_map(|g| match g.kind {
            GoalKind::Wins(n) => Some(n as f64),
            _ => None,
        })
        .unwrap_or(total / 2.0);
    let on_course = wn as f64 / played * total;
    Some(if on_course >= bar + PACE_BAND {
        Pace::Ahead
    } else if on_course <= bar - PACE_BAND {
        Pace::Behind
    } else {
        Pace::On
    })
}

/// His reply from the bank, or today's fixed line while the part has no
/// version he can say (talk facts item 15). `k` keeps each step's pen apart.
fn reply(w: &mut World, s: &Scene, b: &Cast, key: &str, f: &Facts, k: u64, fallback: &str) -> String {
    let spk = Speaker::cast(w, b);
    scene::bank_line(w, s, k, key, &spk, f, fallback)
}

/// The goals that fit the team and the boss: his situation read from the
/// team's last seasons, its level, and his own wants (topic 12).
pub(crate) fn draft_goals(w: &World, team: Team) -> Vec<GoalKind> {
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
            // His class goal came in his talk after signing day when he
            // held you to one there (Ground lane G6).
            if !crate::world::ground::campaign::ad_set(w, goal_season(w, team)) {
                v.push(GoalKind::Recruiting(if avg_wp > 0.6 { 3 } else { 6 }));
            }
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
pub fn goal_season(w: &World, team: Team) -> i32 {
    let kickoff = match team {
        // Another league's opener (the CFL's June, a spring league's
        // March; minor bug B40).
        Team::Other(f) => crate::world::pro::other::season_span(crate::world::pro::other::league_of(f), w.year)
            .map_or(w.dates.nfl_kickoff, |x| x.0),
        _ if team.level() == Level::Nfl => w.dates.nfl_kickoff,
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
            // What you can say rests on where the season stands and who is
            // hurt (talk facts item 15): an option is on the table because
            // its fact is true.
            let pace = pace(w, team);
            let rec = format!("{wn}-{l}");
            let (label, hint) = match pace {
                Some(Pace::Ahead) => (
                    format!("\"We're {rec}, ahead of where you thought. Here's why.\""),
                    "Honest about the good; he likes being told.",
                ),
                Some(Pace::Behind) => (
                    format!("\"We're {rec}, and it isn't good enough. Here's what I see.\""),
                    "Straight talk about a bad stretch; he likes being told early.",
                ),
                _ => (
                    format!("\"We're {rec}, about where we should be. Here's what I see.\""),
                    "Straight talk; he likes being told.",
                ),
            };
            s.pick(30, label, "Tell him straight", "calm", hint);
            if pace == Some(Pace::Behind) || l > wn {
                s.pick(
                    31,
                    "\"I'm not happy either. Here's what changes.\"",
                    "Take the blame",
                    "firm",
                    "Owning it builds trust when it's true.",
                );
            }
            if let Some(p) = claims::team(w, team).get(claims::Kind::StartersOut) {
                let names = p.get("names").unwrap_or("our starters");
                let starters = p.get("starters").unwrap_or("starters");
                // Past three the sheet names the first three: "A, B and C"
                // reads "A, B, C and others".
                let names = if p.get("out").and_then(|x| x.parse::<u32>().ok()).is_some_and(|n| n > 3) {
                    format!("{} and others", names.replacen(" and ", ", ", 1))
                } else {
                    names.to_string()
                };
                s.pick(
                    32,
                    format!("\"We've been without {names}. That's {starters} out.\""),
                    "Deflect",
                    "calm",
                    "He hears an excuse unless it's plainly true.",
                );
            }
        }
        2 => review_open(w, s, team, &b, &mut pen, &mut f),
        crate::world::ground::campaign::talk::CODE => {
            pen.put(w);
            crate::world::ground::campaign::talk::open(w, s, team);
            return;
        }
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
            let season = goal_season(w, team);
            // His lines name the season he sets the goals for, not the
            // calendar year the meeting falls in (a December takeover sets
            // next season's goals).
            f.set("year", season.to_string());
            // A bar you put up in a talk stays, in place of his own of that
            // kind (`asked::hold`).
            let same = |a: &GoalKind, b: &GoalKind| std::mem::discriminant(a) == std::mem::discriminant(b);
            let kept: Vec<GoalKind> =
                w.folk.goals.iter().filter(|g| g.season == season && g.talk).map(|g| g.kind).collect();
            let goals: Vec<GoalKind> =
                draft_goals(w, team).into_iter().filter(|g| !kept.iter().any(|k| same(k, g))).collect();
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
            for g in &kept {
                doc.rows.push(DocRow {
                    pid: None,
                    cells: vec![super::takeover::cap_first(&g.words(level)), "The bar you put up yourself.".into()],
                });
            }
            for g in &goals {
                doc.rows.push(DocRow {
                    pid: None,
                    cells: vec![super::takeover::cap_first(&g.words(level)), why_goal(w, team, *g)],
                });
            }
            s.docs.push(doc);
            w.folk.goals.retain(|g| g.season != season || g.talk);
            for g in goals {
                w.folk.goals.push(Goal { kind: g, season, deal: 0, met: None, set_on: w.today, talk: false });
            }
            s.pick(
                20,
                "\"Done. I'd set them higher myself.\"",
                "Promise",
                "fiery",
                "He likes the ambition; the bar goes up and you'll be judged by it.",
            );
            s.pick(21, "\"Those are fair. We'll get there.\"", "Praise", "calm", "Goals as set.");
            // Asking for time rests on a reason you can name: a losing year.
            // Without one it is only an ask (talk facts item 15).
            let ask_time = match facts::seasons(w, team, 1).last() {
                Some(l) if l.losses > l.wins => {
                    format!("\"We were {}-{} last year. Give me a year to build it.\"", l.wins, l.losses)
                }
                _ => "\"Give me a year to build this the right way.\"".to_string(),
            };
            s.pick(
                22,
                ask_time,
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

pub(crate) fn why_goal(w: &World, team: Team, g: GoalKind) -> String {
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
        GoalKind::Measure(_) => "It's the bar you agreed with him.".into(),
        GoalKind::Campaign(_) => "He set it the day after signing day.".into(),
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
        Some(trust_words(trust(w, team), b.she)),
    );
    // A review that missed something can be answered (talk facts item 15).
    // How the season ended moved his trust above; the answer adds a little
    // (`REVIEW_OWN_TRUST`, `REVIEW_POINT_TRUST`).
    if met < total {
        let missed = ended.iter().find(|x| !x.1).map(|x| x.0.words(level));
        let hit = ended.iter().find(|x| x.1).map(|x| x.0.words(level));
        if let Some(m) = missed {
            s.pick(
                50,
                format!("\"We fell short of the goal to {m}. That's on me, and here is what changes.\""),
                "Take the blame",
                "firm",
                "He hears you own it.",
            );
        }
        if let Some(h) = hit {
            s.pick(
                51,
                format!("\"We hit the goal to {h}. That part of the plan worked.\""),
                "Tell him straight",
                "calm",
                "Fair, if the rest isn't dressed up behind it.",
            );
        }
        s.pick(52, "\"What do you need to see from us next year?\"", "Ask", "calm", "He tells you what he'll judge.");
    }
    add_ask_picks(w, s, team);
}

/// A program's recruiting class's place in its conference in a season
/// (1 the best), from the classes' scores; None when it signed none.
pub fn class_rank(w: &World, p: usize, season: i32) -> Option<u32> {
    let conf = &w.programs[p].conference;
    let mut scores: Vec<(usize, f64)> = w
        .programs
        .iter()
        .enumerate()
        .filter(|(_, q)| &q.conference == conf)
        .filter_map(|(k, q)| q.class_scores.iter().find(|x| x.0 == season).map(|x| (k, x.1)))
        .collect();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    scores.iter().position(|x| x.0 == p).map(|r| r as u32 + 1)
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
            Team::College(p) => class_rank(w, p, season).is_some_and(|r| r <= n as u32),
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
        GoalKind::Measure(u) => super::terms::met(w, super::terms::Who::Team(team), &u, season) == Some(true),
        GoalKind::Campaign(a) => crate::world::ground::campaign::met(w, team, &a),
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
    if s.c == crate::world::ground::campaign::talk::CODE {
        crate::world::ground::campaign::talk::answer(w, s, code, picked);
        return;
    }
    let b = boss(w, team);
    s.you(&scene::your_name(w), picked.label.clone());
    let season = goal_season(w, team);
    // The facts his reply may name (talk facts item 15).
    let mut f = super::takeover::base_facts(w, team);
    f.set("boss", b.first().to_string());
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
            let line = reply(w, s, &b, "boss.goals.reply.higher", &f, 3, "Good. That's the coach I hired.");
            s.them(&b.name, line, None);
            s.outcome.push("You took on a higher bar.".into());
            add_ask_picks(w, s, team);
            return;
        }
        21 => {
            w.folk.mine.nudge(b.key(), 0.02);
            let line = reply(w, s, &b, "boss.goals.reply.fair", &f, 3, "Good. Then we understand each other.");
            s.them(&b.name, line, None);
            s.outcome.push("You agreed to his goals as set.".into());
            add_ask_picks(w, s, team);
            return;
        }
        22 => {
            let patient = ask_time(w, team);
            let (key, line) = if patient {
                ("boss.goals.reply.lowered", "All right. A year to build. But I want to see it moving.")
            } else {
                ("boss.goals.reply.kept", "I didn't hire you to ask for time.")
            };
            let line = reply(w, s, &b, key, &f, 3, line);
            s.them(&b.name, line, None);
            s.outcome.push(if patient {
                "He lowered the bar for this year.".to_string()
            } else {
                "He kept the bar where it was, and noted that you asked.".to_string()
            });
            add_ask_picks(w, s, team);
            return;
        }
        super::backers::NAMING | 151 => {
            super::backers::naming_answer(w, s, team, code);
            add_ask_picks(w, s, team);
            if s.picks.len() <= 1 {
                s.end();
            }
            return;
        }
        50..=52 => {
            // Answering the review (talk facts item 15). How the season ended
            // is already in his trust (`review_open`); the answer adds a
            // little on top: owning the miss or asking, and more for
            // pointing to a goal that was met.
            w.folk.mine.nudge(b.key(), if code == 51 { REVIEW_POINT_TRUST } else { REVIEW_OWN_TRUST });
            let level = team.level();
            let goals: Vec<Goal> = w.folk.goals.iter().filter(|g| g.season == s.b as i32).cloned().collect();
            let met = goals.iter().filter(|g| g.met == Some(true)).count();
            let share = if goals.is_empty() { 0.5 } else { met as f32 / goals.len() as f32 };
            f.set(
                "review",
                if share >= 0.67 {
                    "good"
                } else if share >= 0.34 {
                    "mixed"
                } else {
                    "bad"
                },
            );
            f.set("met", met.to_string());
            let pick_of = |ok: bool| goals.iter().find(|g| g.met == Some(ok)).map(|g| g.kind.words(level));
            let (key, line, said) = match code {
                50 => (
                    "boss.review.reply.own",
                    "I appreciate you saying it. I'll hold you to it.",
                    "You took the review on yourself.",
                ),
                51 => (
                    "boss.review.reply.point",
                    "That's fair. It's the rest I'm worried about.",
                    "You pointed to what worked.",
                ),
                _ => (
                    "boss.review.reply.ask",
                    "Wins, a clean program, and fewer surprises. In that order.",
                    "You asked what he needs to see.",
                ),
            };
            if let Some(g) = pick_of(code == 51) {
                // A goal's words may carry a colon ("...eligible: two at
                // most ruled out"); a line takes the part before it.
                f.set("goal", g.split(':').next().unwrap_or(&g).to_string());
            }
            let line = reply(w, s, &b, key, &f, 4, line);
            s.them(&b.name, line, None);
            s.outcome.push(said.into());
            // An answer to the review is not an ask: the review keeps its four
            // ask rounds (`s.a` counts the answers given).
            s.a += 1;
            add_ask_picks(w, s, team);
            return;
        }
        30 | 31 => {
            w.folk.mine.nudge(b.key(), if code == 31 { 0.04 } else { 0.02 });
            let (wn, l) = facts::record_now(w, team);
            f.set("record", format!("{wn}-{l}"));
            // "I'm not happy either" is answered as a year that isn't going
            // well, unless the pace says it is: a losing record ahead of
            // a low bar is a year on course.
            let at = pace(w, team);
            let at = if code == 31 && at != Some(Pace::Behind) { Some(Pace::On) } else { at };
            let key = format!("boss.mid.reply.{}", at.map_or("on", Pace::key));
            let line =
                reply(w, s, &b, &key, &f, 2, "Okay. Keep me posted. I don't want to read it in the paper first.");
            s.them(&b.name, line, None);
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
            // His words follow the list the option stood on (the trainers'
            // count of last season's starters out) as well as his own count,
            // so he never calls a long list light. The nudge above is the
            // old one.
            let listed = claims::team(w, team)
                .get(claims::Kind::StartersOut)
                .and_then(|p| p.get("out"))
                .and_then(|x| x.parse::<u32>().ok())
                .unwrap_or(0);
            let (key, line) = if true_excuse || listed >= 5 {
                ("boss.mid.reply.injury_true", "I know. I've seen the injury list.")
            } else {
                ("boss.mid.reply.injury_thin", "Everybody has injuries.")
            };
            let line = reply(w, s, &b, key, &f, 2, line);
            s.them(&b.name, line, None);
        }
        40..=45 => {
            let k = AskKind::from_code(code - 40);
            // A pick still on the table after you asked it in words (or on
            // another pick) gets no second answer.
            if asked_lately(w, k) {
                s.them(&b.name, "We've been over that one already.", None);
                s.outcome.push(format!("You asked for {} again: {} had already answered.", k.words(), b.he()));
                add_ask_picks(w, s, team);
                return;
            }
            ask(w, s, team, k);
            // A granted facility while you owe a booster your vote on its
            // name: he asks where you stand (talk facts lane L8).
            if k == AskKind::Facility
                && w.folk.asks.last().is_some_and(|a| a.kind == k && a.granted && a.date == w.today)
                && matches!(team, Team::College(_))
                && super::backers::naming_picks(w, s)
            {
                return;
            }
            add_ask_picks(w, s, team);
            if s.picks.len() <= 1 || u64::from(s.step) >= 3 + s.a.min(2) {
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

/// One yes to a bigger recruiting budget: a tenth more for travel and
/// visits, from today (proposed, not confirmed).
const RECRUIT_RAISE: f32 = 0.1;

/// One yes to more money for the staff pool: a tenth more.
const STAFF_RAISE: f32 = 0.1;

/// What each raise he has already given you of the same kind leaves of his
/// yes chance: half. A budget he has already stretched for you once is
/// harder to stretch again, so the raises can't stack without end (a third
/// yes comes at a quarter of the first's chance; proposed, not confirmed).
pub(crate) const GIVEN_CUT: f64 = 0.5;

/// His yes chance's share left after the raises of this kind he has
/// already granted you in this job (the staff pool's and the recruiting
/// budget's; a new job starts at none).
pub(crate) fn given_cut(w: &World, k: AskKind) -> f64 {
    let given = match k {
        AskKind::StaffMoney => w.folk.staff.pool_bonus / STAFF_RAISE,
        AskKind::RecruitingMoney => w.folk.staff.recruit_bonus / RECRUIT_RAISE,
        _ => 0.0,
    };
    GIVEN_CUT.powf(given.max(0.0).round() as f64)
}

/// His money as the asks read it: a college's budget against a big
/// program's, the NFL's full, another league's thin, a high school's
/// resources.
pub(crate) fn rich(w: &World, team: Team) -> f64 {
    match team {
        Team::College(p) => (crate::world::staff::program_budget(&w.programs[p]) / 80.0e6).min(1.5),
        Team::Nfl(_) => 1.0,
        Team::Other(_) => 0.4,
        Team::High(h) => w.high_schools[h].resources,
    }
}

/// His yes chance for an ask today (topic 12): his money, his trust in you,
/// the goals you've met and his personality.
pub(crate) fn chance(w: &World, team: Team, k: AskKind) -> f64 {
    let b = boss(w, team);
    let t = trust(w, team) as f64;
    let rich = rich(w, team);
    let goals_met = w.folk.goals.iter().filter(|g| g.met == Some(true)).count() as f64;
    (0.15 + 0.6 * (t - 0.4) + 0.2 * rich + 0.06 * goals_met - 0.03 * (b.tight - 10.0) + 0.02 * (b.warmth - 10.0)
        - if k == AskKind::Extension && w.folk.mine.since.is_some_and(|d| d.days_to(w.today) < 500) {
            0.3
        } else {
            0.0
        })
    .clamp(0.03, 0.92)
        * given_cut(w, k)
}

/// Whether he says yes to an ask from the meeting's picks today (topic 12):
/// his chance through the day's keyed roll. A typed ask is weighed as said
/// instead (`asked::money`).
pub fn ask_yes(w: &World, team: Team, k: AskKind) -> bool {
    super::roll(w, 0xA5C0 ^ w.today.0 as u64, k.code() as u64) < chance(w, team, k)
}

/// Whether you asked for this lately (the meeting offers it once in 300
/// days).
pub fn asked_lately(w: &World, k: AskKind) -> bool {
    w.folk.asks.iter().any(|a| a.kind == k && a.date.days_to(w.today) < 300)
}

/// You ask for something (topic 12) and he answers: what a yes does, his
/// trust, and the ask kept. Returns his yes and his words (the meeting's
/// pick and a typed ask both come here).
pub fn ask_now(w: &mut World, team: Team, k: AskKind) -> (bool, &'static str) {
    let b = boss(w, team);
    let yes = ask_yes(w, team, k);
    let words = if yes {
        grant(w, team, k)
    } else {
        match k {
            AskKind::Extension => "Not yet. Win and we'll talk.",
            AskKind::Facility => "Not this year. The money's committed.",
            _ => "I can't do it right now.",
        }
    };
    w.folk.mine.nudge(b.key(), if yes { -0.01 } else { -0.005 });
    w.folk.asks.push(Ask { kind: k, date: w.today, granted: yes, words: words.into() });
    if w.folk.asks.len() > 40 {
        w.folk.asks.remove(0);
    }
    (yes, words)
}

/// What a yes does, and the meeting's words for it.
pub(crate) fn grant(w: &mut World, team: Team, k: AskKind) -> &'static str {
    match k {
        AskKind::StaffMoney => {
            w.folk.staff.pool_bonus += STAFF_RAISE;
            match team {
                Team::College(pg) => crate::world::finance::college::raise_staff(w, pg, STAFF_RAISE as f64),
                Team::Nfl(t) => crate::world::finance::nfl::raise_staff(w, t, STAFF_RAISE as f64),
                _ => {}
            }
            "Yes. Ten percent more for the staff pool, starting now."
        }
        AskKind::Facility => {
            if let Team::College(pg) = team {
                w.programs[pg].facilities = (w.programs[pg].facilities + 0.03).min(1.0);
            }
            "Yes. We'll break ground after the season."
        }
        AskKind::RecruitingMoney => {
            w.folk.staff.recruit_bonus += RECRUIT_RAISE;
            if let Team::College(pg) = team {
                crate::world::finance::college::raise_recruiting(w, pg, RECRUIT_RAISE as f64);
            }
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
}

/// You ask him for time (the goals meeting's "it's a rebuild"; live talks
/// lane N5c's ask for time): a patient boss (11+) lowers this season's
/// wins bar by two and drops a title or postseason goal; an impatient one
/// keeps it and holds the asking against you. Returns whether he gave it.
pub fn ask_time(w: &mut World, team: Team) -> bool {
    let b = boss(w, team);
    let patient = b.patience >= 11.0;
    settle_time(w, goal_season(w, team), patient);
    w.folk.mine.nudge(b.key(), if patient { 0.0 } else { -0.05 });
    patient
}

/// A season's goals after you asked for time: marked as asked, and when he
/// gave it, the wins bar two lower and a title or postseason goal dropped.
pub(crate) fn settle_time(w: &mut World, season: i32, gave: bool) {
    for g in w.folk.goals.iter_mut().filter(|g| g.season == season) {
        g.deal = -1;
        if gave && let GoalKind::Wins(n) = g.kind {
            g.kind = GoalKind::Wins(n.saturating_sub(2).max(2));
        }
    }
    w.folk.goals.retain(|g| !(g.season == season && gave && matches!(g.kind, GoalKind::Title | GoalKind::Postseason)));
}

/// You ask for something in the meeting (topic 12).
fn ask(w: &mut World, s: &mut Scene, team: Team, k: AskKind) {
    let b = boss(w, team);
    let (yes, words) = ask_now(w, team, k);
    s.them(&b.name, words, None);
    s.outcome.push(format!("You asked for {}: {}.", k.words(), if yes { "granted" } else { "no" }));
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    meeting_open(w, s);
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    meeting_answer(w, s, code, picked);
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    if s.c == crate::world::ground::campaign::talk::CODE {
        crate::world::ground::campaign::talk::hand(w, s);
        return;
    }
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

/// How far the public bar moves your review, in seasons' shares (a share
/// of the season's games: 0.06 is about one game of a 17-game NFL year).
/// Claude's size (proposed, not confirmed).
pub const PUBLIC_BAR: f64 = 0.06;

/// The public bar you set (`Mine.expectation`, -1 to 1: championships
/// promised at your introduction or to the team, a higher bar taken on with
/// the boss, or "it's a process"), as the people who judge you hold it to
/// the season: the same record reads that much worse, or better, in your
/// review (`staff::college_review`, `staff::nfl_firing`, and your high
/// school's season review). A raised bar weighs fully for three seasons,
/// half in the fourth and fifth, then your record is the bar. A lowered
/// one buys only the first season, as its hint says. A season that clears
/// the bar is not hurt: the reviews fire only on a read past their own
/// line. `tenure`: full seasons in the job. Proposed, not confirmed.
pub fn public_bar(w: &World, tenure: i32) -> f64 {
    let e = w.folk.mine.expectation as f64;
    let fade = match (e > 0.0, tenure) {
        (true, ..=3) => 1.0,
        (true, 4 | 5) => 0.5,
        (false, ..=1) => 1.0,
        _ => 0.0,
    };
    PUBLIC_BAR * e * fade
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
    // Another league meets on its own season (minor bug B40): goals three
    // weeks before the opener, as camp opens, the midpoint, and the review
    // after the title game.
    let (pre, mid, post) = if let Team::Other(f) = team {
        let k = crate::world::pro::other::league_of(f);
        let Some((start, half, end)) = crate::world::pro::other::season_span(k, w.year) else { return };
        (start.add(-21), half.add(2), end.add(4))
    } else {
        match team.level() {
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
            _ => {
                (w.dates.hs_school_start.add(-12), w.dates.hs_school_start.add(45), w.dates.december_graduation.add(-5))
            }
        }
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
        if c == 2 && matches!(team, Team::Nfl(_)) {
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
