//! Commitments to the people you answer to (Cain 2026-11-16: "it also says
//! take on more, so why isn't he like 'Okay so you promise to recruit the
//! #1'"). Before this a typed commitment to your boss had no home: the
//! ear's promises were all promises to a player (a start, a role, touches),
//! so "we'll keep the state's #1 kid home" dropped, and the goals meeting's
//! "take on more" was only its one chip. Now the ear reads a commitment as
//! a goal by its key (`Commitment`), and the game weighs it here:
//!
//! - your boss writes down a goal he can judge, in place of his own of
//!   that kind for its season when yours asks more (`asked::hold_goal`, the
//!   bar a money ask puts up), and says whether he believes it from your
//!   record; one already on his list, or below his own, changes nothing,
//!   and one he can't measure ("we'll add playmakers") gets his ask for a
//!   number;
//! - a booster hears it on what he wants (`cast::Want`), logged as the
//!   booster meeting's chip logs it (`talks::PromiseKind::Booster`).
//!
//! The goals: the season's (wins, the postseason, a title, the rival, a
//! clean program, grades, the young players, the crowds) and the class
//! goals of the recruiting list (`ground::campaign::goals`) that one class
//! settles and that need no name (the state's #1, keep N of the state's
//! top 10 home, a top-N class), each judged by the review as his own are
//! (`boss::goal_met`). Every size is Claude's (proposed, not confirmed).

use super::Team;
use super::asked;
use super::boss::{self, GoalKind};
use super::cast::Want;
use crate::calendar::Level;
use crate::world::World;
use crate::world::ground::campaign::{self, goals};

/// What the ear heard you commit to: a goal's key (`keys`), its number
/// when said, whether you said every year, and your words.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Commitment {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(default)]
    pub every: bool,
    #[serde(default)]
    pub words: String,
}

/// The season's goals by their key, as the ear names them.
pub const SEASON_KEYS: [&str; 8] =
    ["wins", "postseason", "title", "beat_rival", "clean", "grades", "develop", "crowds"];

/// The class goals the ear may name: one class settles them, they're judged
/// when it signs, and they need no name to be judged (a rival is the
/// program's own). `conference_rank` is the boss's own class goal.
pub fn class_keys() -> Vec<&'static str> {
    goals::GOALS
        .iter()
        .filter(|d| d.classes == 1 && !d.later && matches!(d.subj, goals::Subj::None | goals::Subj::Rival))
        .map(|d| d.key)
        .collect()
}

/// Every key the ear may name, then "none" for a commitment nothing can
/// measure.
pub fn keys() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = SEASON_KEYS.to_vec();
    v.extend(class_keys());
    v.push("none");
    v
}

/// The goal a commitment names at your seat, or why there's none.
pub fn goal_of(w: &World, team: Team, c: &Commitment) -> Result<GoalKind, Took> {
    let college = matches!(team, Team::College(_));
    Ok(match c.key.as_str() {
        "wins" => match c.n {
            Some(n) if (2..=17).contains(&n) => GoalKind::Wins(n as u8),
            _ => return Err(Took::Vague),
        },
        "postseason" => GoalKind::Postseason,
        "title" => GoalKind::Title,
        "beat_rival" => GoalKind::BeatRival,
        "clean" => GoalKind::Clean,
        "grades" => GoalKind::Grades,
        "develop" => GoalKind::Develop,
        "crowds" => GoalKind::Crowds,
        // The boss's own class goal: a top-N class in the conference.
        "conference_rank" if college => match c.n {
            Some(n) if (1..=12).contains(&n) => GoalKind::Recruiting(n as u8),
            _ => return Err(Took::Vague),
        },
        k => {
            let Some(d) = goals::by_key(k).filter(|_| class_keys().contains(&k)) else { return Err(Took::Vague) };
            let Team::College(prog) = team else { return Err(Took::NotHere) };
            let of = match d.subj {
                goals::Subj::Rival => match goals::rival(w, prog) {
                    Some(r) => r as u32,
                    None => return Err(Took::NotHere),
                },
                _ => 0,
            };
            let n = if d.n == 0 { 0 } else { c.n.filter(|&n| n > 0).unwrap_or(d.n) };
            GoalKind::Campaign(goals::Aim { goal: d.id, n, of, class: campaign::class_now(w) })
        }
    })
}

/// How your boss took a commitment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Took {
    /// He writes it down: the goal for its season, in place of his own of
    /// that kind. `felt` is how much he believes it (0-1), `record` what
    /// he read it from.
    Held { goal: GoalKind, felt: f32, record: Option<u32> },
    /// Already on his list at that bar or higher.
    Already(GoalKind),
    /// Less than his own goal of that kind, which stays.
    Below { yours: GoalKind, his: GoalKind },
    /// Nothing he can measure as said.
    Vague,
    /// Not a goal at this level (a class goal in the NFL).
    NotHere,
    /// A booster heard it on what he wants, and holds you to it.
    Booster(Want),
    /// A booster heard it, and it isn't what he cares about.
    NotHisWant,
}

impl Took {
    /// Whether it went on the books.
    pub fn held(self) -> bool {
        matches!(self, Took::Held { .. } | Took::Booster(_))
    }

    /// "Heard as": what the commitment did, in a few words.
    pub fn heard(self) -> &'static str {
        match self {
            Took::Held { felt, .. } if felt < asked::DOUBT as f32 => "a commitment he wrote down, doubting it",
            Took::Held { .. } => "a commitment he wrote down",
            Took::Already(_) => "a commitment already on his list",
            Took::Below { .. } => "a commitment below his own bar",
            Took::Vague => "a commitment he can't measure",
            Took::NotHere => "a commitment nobody here judges",
            Took::Booster(_) => "a commitment he'll hold you to",
            Took::NotHisWant => "a commitment he doesn't care about",
        }
    }

    /// His trust in you for it (`hold`): a bar he believes, a little; one
    /// he doubts costs a little.
    pub fn trust(self) -> f32 {
        match self {
            Took::Held { felt, .. } if felt < asked::DOUBT as f32 => DOUBTED_TRUST,
            Took::Held { .. } => HELD_TRUST,
            _ => 0.0,
        }
    }
}

/// His trust in you when he writes down a bar he believes, and when he
/// doubts it (a coach promising what his record doesn't back). Proposed,
/// not confirmed.
pub const HELD_TRUST: f32 = 0.02;
pub const DOUBTED_TRUST: f32 = -0.01;

/// The goal he holds now for a goal's season, of the same kind.
fn his_goal(w: &World, team: Team, g: GoalKind) -> Option<GoalKind> {
    let season = asked::bar_season(w, team, g);
    w.folk.goals.iter().find(|x| x.season == season && asked::same(&x.kind, &g)).map(|x| x.kind).or_else(|| {
        // Before he sets the season's goals, the ones he would set.
        if w.folk.goals.iter().any(|x| x.season == season && !x.talk) {
            None
        } else {
            boss::draft_goals(w, team).into_iter().find(|x| asked::same(x, &g))
        }
    })
}

/// A class goal with a number: whether a higher number asks more (a count:
/// keep 6 of the top 10 home) or less (a rank: a top-25 class).
fn more_is_higher(a: &goals::Aim) -> bool {
    !a.def().name.contains("top-{n}")
}

/// Whether yours asks more than his.
fn above(yours: GoalKind, his: GoalKind) -> Option<bool> {
    Some(match (yours, his) {
        (GoalKind::Wins(a), GoalKind::Wins(b)) => a > b,
        (GoalKind::Recruiting(a), GoalKind::Recruiting(b)) => a < b,
        (GoalKind::Campaign(a), GoalKind::Campaign(b)) if a.goal == b.goal => {
            if a.n == b.n {
                false
            } else if more_is_higher(&a) {
                a.n > b.n
            } else {
                a.n < b.n
            }
        }
        _ if asked::same(&yours, &his) => false,
        _ => return None,
    })
}

/// How much he believes it (0-1) and the record he read: your record at
/// it (`asked::credible`; for a class goal the same goal on the last class,
/// `goals::likely`), a sharp judge of people trusting it less.
fn belief(w: &World, team: Team, g: GoalKind) -> (f32, Option<u32>) {
    let (cred, record) = match (g, team) {
        (GoalKind::Campaign(a), Team::College(prog)) => (goals::likely(w, prog, &a, 0.0) as f64, None),
        _ => asked::credible(w, team, g),
    };
    let b = boss::boss(w, team);
    let sharp = (b.judgement / 20.0).clamp(0.0, 1.0);
    ((cred + (1.0 - cred) * (1.0 - sharp) * 0.5) as f32, record)
}

/// Your boss weighs a commitment, without changing the world.
pub fn weigh(w: &World, team: Team, c: &Commitment) -> Took {
    let g = match goal_of(w, team, c) {
        Ok(g) => g,
        Err(t) => return t,
    };
    if let Some(his) = his_goal(w, team, g)
        && above(g, his) != Some(true)
    {
        return if above(his, g) == Some(true) { Took::Below { yours: g, his } } else { Took::Already(his) };
    }
    let (felt, record) = belief(w, team, g);
    Took::Held { goal: g, felt, record }
}

/// What a booster hears in a commitment: the want it speaks to.
pub fn want_of(c: &Commitment) -> Option<Want> {
    Some(match c.key.as_str() {
        "beat_rival" | "beat_rival_class" | "head_to_head" | "backyard" => Want::BeatRival,
        "wins" | "postseason" | "title" => Want::WinNow,
        "clean" | "clean_class" => Want::Clean,
        _ => return None,
    })
}

/// A booster weighs it: on one of his wants he holds you to it.
pub fn booster(wants: &[Want], c: &Commitment) -> Took {
    match want_of(c) {
        Some(x) if wants.contains(&x) => Took::Booster(x),
        _ => Took::NotHisWant,
    }
}

/// What he says to it, for the voice.
pub fn words(w: &World, team: Team, c: &Commitment, t: Took) -> Vec<String> {
    let level = team.level();
    let mut out = Vec::new();
    match t {
        Took::Held { goal, felt, record } => {
            out.push(format!("He writes it down: the goal is now to {}. The coach is judged on it.", goal_words(w, goal, level)));
            out.push(if felt < asked::DOUBT as f32 {
                format!("He doesn't believe it yet: {}.", record_words(w, team, goal, record))
            } else if felt >= 0.7 {
                "He believes it.".into()
            } else {
                "He'll believe it when he sees it.".into()
            });
            if c.every {
                out.push("He holds the coach to this year's; next year's is for next year's goals meeting.".into());
            }
        }
        // Each in the third person, as what he does: the voice says it as
        // himself ("That's already on my list"), never word for word ("on
        // his list", the talks audit of 2026-11-16).
        Took::Already(g) => out.push(format!(
            "He already has that on his sheet ({}), so nothing changes: he holds the coach to it.",
            goal_words(w, g, level)
        )),
        Took::Below { his, .. } => out.push(format!(
            "It asks less than his own goal ({}), which stays; he tells the coach that asking for time is how a bar comes down.",
            goal_words(w, his, level)
        )),
        // The examples steered him to the class whatever was said (Cain's
        // draft-picks line got "how you'll measure the class", the talks
        // audit of 2026-11-16).
        Took::Vague => out.push(
            "He can't hold the coach to that as said: he asks what he can measure it by, a number he can check when the season is over."
                .into(),
        ),
        Took::NotHere => out.push("That isn't something he judges at this level.".into()),
        Took::Booster(x) => out.push(format!("He'll hold the coach to it: {}.", x.short())),
        Took::NotHisWant => out.push("He's glad to hear it, but it isn't what he's after from the coach.".into()),
    }
    out
}

/// A goal as something to do, for a sentence ("the goal is now to win 8
/// games", "you're judged on whether you meet the class goal: the state's
/// #1").
pub fn goal_words(w: &World, g: GoalKind, level: Level) -> String {
    match g {
        GoalKind::Campaign(a) => format!("meet the class goal: {}", lower_first(&goals::words(w, &a))),
        _ => g.words(level),
    }
}

/// A goal as the sheet on the table lists it: a class goal by its name
/// ("The state's #1"), any other as something to do ("Win 8 games").
pub fn sheet_words(w: &World, g: GoalKind, level: Level) -> String {
    match g {
        GoalKind::Campaign(a) => goals::words(w, &a),
        _ => super::takeover::cap_first(&g.words(level)),
    }
}

fn lower_first(s: &str) -> String {
    let mut ch = s.chars();
    ch.next().map_or(String::new(), |f| f.to_lowercase().collect::<String>() + ch.as_str())
}

fn record_words(w: &World, team: Team, g: GoalKind, record: Option<u32>) -> String {
    match (g, team) {
        (GoalKind::Campaign(a), Team::College(prog)) => {
            let last = goals::Aim { class: a.class - 1, ..a };
            let r = goals::read(w, prog, &last, last.class);
            if r.waiting || r.words.is_empty() {
                "nothing on the record backs it".into()
            } else {
                format!("last year's class: {}", r.words)
            }
        }
        _ => asked::record(g, record),
    }
}

/// Makes a commitment your boss took real: the goal on his list for its
/// season (`asked::hold_goal`; said in his office, it puts up no public
/// bar) and his trust in you. Returns the outcome line.
/// A second time the same line runs (it can't: the turn is applied once)
/// would hold it again, which changes nothing.
pub fn hold(w: &mut World, team: Team, t: Took, she: bool) -> Option<String> {
    let Took::Held { goal, felt, .. } = t else { return None };
    let b = boss::boss(w, team);
    asked::hold_goal(w, team, goal);
    let doubt = felt < asked::DOUBT as f32;
    w.folk.mine.nudge(b.key(), t.trust());
    let he = if she { "She" } else { "He" };
    Some(format!(
        "{he} wrote it down: you're judged on whether you {}.{}",
        goal_words(w, goal, team.level()),
        if doubt { format!(" {he} doesn't believe it yet.") } else { String::new() }
    ))
}

/// Who heard a commitment, and how he took it: your boss weighs it as a
/// goal, a booster on what he wants. None for anyone else (a commitment to
/// a player is a promise to him, through P1).
pub fn took(w: &World, him: super::persona::Them, c: &Commitment) -> Option<Took> {
    use super::persona::Them;
    use super::persona::wants::Kind;
    let Them::Cast(r, _) = him else { return None };
    match super::persona::makeup::kind(w, him) {
        Kind::Boss => Some(weigh(w, Team::seat(w)?, c)),
        Kind::Booster => {
            let cast = super::cast::get(w, r);
            Some(booster(&cast.booster.map(|b| b.wants).unwrap_or_default(), c))
        }
        _ => None,
    }
}

/// What he says to it, for the voice, by who he is.
pub fn said(w: &World, c: &Commitment, t: Took) -> Vec<String> {
    match Team::seat(w) {
        Some(team) => words(w, team, c, t),
        None => Vec::new(),
    }
}

/// Makes it real once the line is said: the goal on your boss's list, or
/// the booster's promise logged as his meeting's chip logs it. Returns the
/// outcome line.
pub fn land(w: &mut World, him: super::persona::Them, t: Took) -> Option<String> {
    use super::persona::Them;
    let Them::Cast(r, term) = him else { return None };
    let she = super::cast::get(w, r).she;
    match t {
        Took::Held { .. } => hold(w, Team::seat(w)?, t, she),
        Took::Booster(want) => {
            let c = super::cast::get(w, r);
            if c.term != term {
                return None;
            }
            super::talks::promise(
                w,
                super::talks::PromiseTo::Cast(r, term),
                super::talks::PromiseKind::Booster(want),
                0,
            );
            Some(format!("You promised {} {}.", c.name, want.short()))
        }
        _ => None,
    }
}

/// The goals sheet on a goals meeting, drawn again from his list once a
/// commitment changed it, so the table shows your bar where his was.
pub fn show(w: &mut World, scene: u32) {
    let Some(team) = Team::seat(w) else { return };
    let level = team.level();
    let season = boss::goal_season(w, team);
    let Some(k) = w.folk.scenes.iter().position(|s| s.id == scene) else { return };
    let Some(d) = w.folk.scenes[k].docs.iter().position(|d| d.title.ends_with(&format!("goals for {season}"))) else {
        return;
    };
    let rows: Vec<super::scene::DocRow> = w
        .folk
        .goals
        .iter()
        .filter(|g| g.season == season)
        .map(|g| super::scene::DocRow {
            pid: None,
            cells: vec![
                sheet_words(w, g.kind, level),
                if g.talk && g.deal == 1 {
                    "The bar you put up yourself.".into()
                } else if g.talk {
                    "Set in an earlier talk.".into()
                } else {
                    boss::why_goal(w, team, g.kind)
                },
            ],
        })
        .collect();
    w.folk.scenes[k].docs[d].rows = rows;
}
