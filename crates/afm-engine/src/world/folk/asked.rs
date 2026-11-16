//! What you ask your boss or a booster for in your own words (live talks
//! v3), weighed as said: what you put up for it, his trust in you, his
//! money and his nature, each answer with its real reason. A typed ask
//! never lands on a meeting's pick or its canned answer; the meeting's
//! picks keep their own roll (`boss::ask_yes`, `ask_now`, `ask_time`).
//!
//! Money (the staff pool, recruiting, the collective, a facility,
//! practice-squad spots, a hire's money) and your extension start from the
//! chance the meeting reads (his trust in you, his budget, the goals you've
//! met, how tight he keeps the money, his warmth), then:
//!
//! - a yes on the same thing in the last 300 days stands, and the money
//!   isn't found twice (the meeting offers each ask once in that window);
//! - each no on it in that window weighs against it;
//! - a bar you put up for it (a top-3 class, nine wins, a bowl, a title,
//!   the rival) pulls him toward yes when it's above the bar he set, or
//!   would set, and he believes it from your record; a stretch he doesn't
//!   believe costs you (a sharp judge of people doubts sooner).
//!
//! A yes puts your bar on his goals for its season, and the goals meeting
//! keeps it. Time weighs his patience, his trust, how new you are in the
//! job and whether you took on a higher bar, against the season's goals in
//! force; his backing with the big booster weighs his trust and his nature;
//! a booster's money weighs his trust in you and what he wants, once a
//! year. Every size here is Claude's (proposed, not confirmed).

use super::Team;
use super::boss::{self, Ask, AskKind, Goal, GoalKind};
use super::cast::{Cast, Want};
use super::facts;
use crate::calendar::Level;
use crate::world::World;

/// The most a bar you put up adds to his yes chance, at full belief.
/// Proposed, not confirmed.
pub const BAR_PULL: f64 = 0.25;
/// Below this belief he doesn't buy the bar. Proposed, not confirmed.
pub const DOUBT: f64 = 0.35;
/// What a bar he doesn't buy costs the ask. Proposed, not confirmed.
pub const BAR_DOUBT: f64 = 0.05;
/// What's left of the chance for each no he gave the same ask in the
/// window. Proposed, not confirmed.
pub const LATELY_NO: f64 = 0.3;
/// His trust in you after a yes that carries your bar, on top of the ask's
/// own cost. Proposed, not confirmed.
pub const BAR_TRUST: f32 = 0.03;
/// The public bar's lift for a bar you took on in a talk (the goals
/// meeting's higher bar is 0.15). Proposed, not confirmed.
pub const BAR_EXPECT: f32 = 0.08;
/// The grace a boss gives a coach in his first year in the job, and his
/// second, toward time. Proposed, not confirmed.
pub const GRACE: [f64; 2] = [0.2, 0.1];
/// What a no to time costs: an impatient boss (under 11) and a patient one.
/// Proposed, not confirmed.
pub const TIME_NO: [f32; 2] = [-0.05, -0.02];
/// Days a yes stands and a no weighs: the boss meeting's own window.
pub const WINDOW: i32 = 300;
/// The words a booster's gift is kept with after his name
/// (`office::asks::run`), so your boss's record tells his own grants from
/// a booster's.
pub const GIFT: &str = " put more into the collective.";

/// What you put up for an ask: a bar he can hold you to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Case {
    pub bar: Option<GoalKind>,
}

impl Case {
    /// The bar in what you said the ask is for (the ear's words for the
    /// request): a top-N recruiting class, a number of wins, a title, the
    /// postseason or the rival. Nothing when you put nothing up.
    pub fn read(words: &str, level: Level) -> Case {
        let l = words.to_lowercase();
        let toks: Vec<&str> = l.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty()).collect();
        let has = |xs: &[&str]| xs.iter().any(|x| l.contains(x));
        let word = |xs: &[&str]| toks.iter().any(|t| xs.iter().any(|x| t.starts_with(x)));
        let pairs = || toks.windows(2);
        // The first top-N that is a class rank ("from a top 40 class to a
        // top 3" is a top 3).
        if level == Level::College && word(&["class", "recruit", "signing"]) {
            let top =
                pairs().find_map(|p| if p[0] == "top" { number(p[1]).filter(|n| (1..=10).contains(n)) } else { None });
            if let Some(n) = top {
                return Case { bar: Some(GoalKind::Recruiting(n as u8)) };
            }
        }
        // Wins said as a bar: "nine wins", "a 10-win season", "win 9
        // games". A count of games alone ("we lost 5 games") is not one.
        let wins = pairs().find_map(|p| {
            let n = if matches!(p[1], "win" | "wins") {
                number(p[0])
            } else if p[0] == "win" {
                number(p[1])
            } else {
                None
            };
            n.filter(|n| (2..=17).contains(n))
        });
        if let Some(n) = wins {
            return Case { bar: Some(GoalKind::Wins(n as u8)) };
        }
        if has(&[
            "win the conference",
            "win the division",
            "win the district",
            "conference title",
            "conference championship",
            "division title",
            "district title",
            "state title",
            "championship",
        ]) {
            return Case { bar: Some(GoalKind::Title) };
        }
        if word(&["bowl", "playoff", "postseason"]) {
            return Case { bar: Some(GoalKind::Postseason) };
        }
        if word(&["rival"]) {
            return Case { bar: Some(GoalKind::BeatRival) };
        }
        Case::default()
    }

    /// A key for the day's roll, so the same ask with the same bar gets the
    /// same answer that day and a new bar is weighed on its own.
    fn key(&self) -> u64 {
        match self.bar {
            None => 0,
            Some(GoalKind::Recruiting(n)) => 0x100 | n as u64,
            Some(GoalKind::Wins(n)) => 0x200 | n as u64,
            Some(GoalKind::Title) => 0x300,
            Some(GoalKind::Postseason) => 0x400,
            Some(GoalKind::BeatRival) => 0x500,
            Some(_) => 0x600,
        }
    }
}

/// A bar in words that read the same at every level ("a top-3 class in the
/// conference", "9 wins"), for the confirm card.
pub fn bar_words(g: GoalKind) -> String {
    match g {
        GoalKind::Recruiting(n) => format!("a top-{n} class in the conference"),
        GoalKind::Wins(n) => format!("{n} wins"),
        GoalKind::Title => "a title".into(),
        GoalKind::Postseason => "the postseason".into(),
        GoalKind::BeatRival => "beating the rival".into(),
        GoalKind::Measure(u) => super::terms::words(&u),
        other => other.words(Level::College),
    }
}

/// A number as said: "3", "3rd", "three".
fn number(t: &str) -> Option<u32> {
    if let Ok(n) = t.parse::<u32>() {
        return Some(n);
    }
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() && matches!(&t[digits.len()..], "st" | "nd" | "rd" | "th") {
        return digits.parse().ok();
    }
    const WORDS: [&str; 17] = [
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
    ];
    WORDS.iter().position(|w| *w == t).map(|k| k as u32 + 1)
}

/// Why he answered as he did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Why {
    /// No such money at this level.
    NotHere,
    /// No big booster to talk to.
    NoBooster,
    /// He said no to this so many days ago.
    AskedNo(u32),
    /// He already found you this money so many days ago.
    Stretched(u32),
    /// He already extended you so many days ago.
    Extended(u32),
    /// The money isn't there.
    Budget,
    /// He keeps the money tight.
    Tight,
    /// He doesn't trust you enough yet.
    Trust,
    /// He doesn't believe the bar you put up: your record at it.
    Doubts(GoalKind, Option<u32>),
    /// Too early in your contract for an extension.
    TooSoon,
    /// No goals set yet for the season: time is for the goals meeting.
    NoBar,
    /// You already asked for time this season.
    TimeAsked,
    /// You took on a higher bar this season and now ask for time.
    YourWord,
    /// He isn't a patient man.
    Impatient,
    /// He doesn't spend his standing with the booster lightly.
    Standing,
    /// A booster who put money in so many days ago.
    Gave(u32),
    /// A booster who wants wins first.
    WantsWins,
    /// Close, but not now.
    Close,
    /// Yes, and he'll hold you to your bar.
    Bar(GoalKind),
}

/// His answer and its reasons, the main one first.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub yes: bool,
    pub why: Vec<Why>,
}

impl Answer {
    fn no(why: Why) -> Answer {
        Answer { yes: false, why: vec![why] }
    }
}

/// The most reasons an answer gives.
const MAX_WHY: usize = 2;

fn within(w: &World, a: &Ask) -> bool {
    a.date.days_to(w.today) < WINDOW
}

fn days(w: &World, a: &Ask) -> u32 {
    a.date.days_to(w.today).max(0) as u32
}

/// Whether an ask on record is your boss's own answer, not a booster's
/// gift.
fn by_boss(a: &Ask) -> bool {
    !a.words.ends_with(GIFT)
}

/// Days since your boss said yes to this, when it was in the window.
pub fn granted(w: &World, k: AskKind) -> Option<u32> {
    w.folk.asks.iter().rev().find(|a| a.kind == k && a.granted && by_boss(a) && within(w, a)).map(|a| days(w, a))
}

/// Your boss's answer to money or your extension, asked in your words.
pub fn money(w: &World, team: Team, k: AskKind, case: &Case) -> Answer {
    let level = team.level();
    if !k.at(level) {
        return Answer::no(Why::NotHere);
    }
    // A yes in the window stands: the same money isn't found twice.
    if let Some(d) = granted(w, k) {
        return Answer::no(if k == AskKind::Extension { Why::Extended(d) } else { Why::Stretched(d) });
    }
    let b = boss::boss(w, team);
    let mut p = boss::chance(w, team, k);
    let mut drags = Vec::new();
    let nos: Vec<&Ask> =
        w.folk.asks.iter().filter(|a| a.kind == k && !a.granted && by_boss(a) && within(w, a)).collect();
    if let Some(last) = nos.last() {
        p *= LATELY_NO.powi(nos.len() as i32);
        drags.push(Why::AskedNo(days(w, last)));
    }
    weigh_bar(w, team, &b, case, &mut p, &mut drags);
    if k == AskKind::Extension && w.folk.mine.since.is_some_and(|d| d.days_to(w.today) < 500) {
        drags.push(Why::TooSoon);
    }
    if boss::rich(w, team) < 0.6 {
        drags.push(Why::Budget);
    } else if b.tight >= 13.0 {
        drags.push(Why::Tight);
    }
    if boss::trust(w, team) < 0.45 {
        drags.push(Why::Trust);
    }
    // A bar on the table is weighed on its own roll, and a yes holds you to
    // it whether he believed it or not; one that asks no more than his own
    // is nothing on the table.
    let on_table = on_table(w, team, case);
    let roll = super::roll(w, 0x7A5C ^ w.today.0 as u64 ^ on_table.key() << 20, k.code() as u64);
    answer(roll < p.clamp(0.02, 0.95), on_table.bar, drags)
}

/// What you put up that asks more than his own bar: the rest is nothing on
/// the table.
pub fn on_table(w: &World, team: Team, case: &Case) -> Case {
    Case { bar: case.bar.filter(|&g| raises(w, team, g)) }
}

/// A bar you put up, weighed: his belief in it from your record and how
/// sharp a judge he is move the chance. A bar no higher than his own is
/// nothing on the table.
fn weigh_bar(w: &World, team: Team, b: &Cast, case: &Case, p: &mut f64, drags: &mut Vec<Why>) {
    let Some(g) = on_table(w, team, case).bar else { return };
    let (cred, record) = credible(w, team, g);
    let sharp = (b.judgement / 20.0).clamp(0.0, 1.0);
    let felt = cred + (1.0 - cred) * (1.0 - sharp) * 0.5;
    if felt < DOUBT {
        *p -= BAR_DOUBT;
        drags.insert(0, Why::Doubts(g, record));
    } else {
        *p += BAR_PULL * felt;
    }
}

fn answer(yes: bool, bar: Option<GoalKind>, drags: Vec<Why>) -> Answer {
    if yes {
        return Answer { yes, why: bar.map(Why::Bar).into_iter().collect() };
    }
    let mut why = drags;
    why.truncate(MAX_WHY);
    if why.is_empty() {
        why.push(Why::Close);
    }
    Answer { yes, why }
}

/// One goal and another of the same kind (a season term only when it is
/// the same term).
fn same(a: &GoalKind, b: &GoalKind) -> bool {
    match (a, b) {
        (GoalKind::Measure(_), GoalKind::Measure(_)) => a == b,
        _ => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

/// The season a bar is held for, the one its goal is judged by: a class's
/// rank is the class being signed now, which the boss judges after next
/// season (`boss::goal_met` reads a season's class); any other bar is for
/// the season the goals are for (`boss::goal_season`).
fn bar_season(w: &World, team: Team, g: GoalKind) -> i32 {
    match g {
        GoalKind::Recruiting(_) => w.year + 1,
        _ => boss::goal_season(w, team),
    }
}

/// Whether a bar is above the one he set for its season, or, before he
/// sets that season's goals, above the one he would set
/// (`boss::draft_goals`); a goal he wouldn't set at all is above.
fn raises(w: &World, team: Team, g: GoalKind) -> bool {
    let season = bar_season(w, team, g);
    let set = w.folk.goals.iter().find(|x| x.season == season && same(&x.kind, &g)).map(|x| x.kind);
    match set.or_else(|| boss::draft_goals(w, team).into_iter().find(|x| same(x, &g))) {
        None => true,
        Some(GoalKind::Recruiting(n)) => matches!(g, GoalKind::Recruiting(m) if m < n),
        Some(GoalKind::Wins(n)) => matches!(g, GoalKind::Wins(m) if m > n),
        Some(_) => false,
    }
}

/// How believable a bar is from the record (0 to 1), and the record it
/// reads: your program's better class in the conference of the last two,
/// or last season's wins.
fn credible(w: &World, team: Team, g: GoalKind) -> (f64, Option<u32>) {
    let last = facts::seasons(w, team, 1).into_iter().last();
    let share = last.as_ref().map(|s| s.wins as f64 / (s.wins + s.losses).max(1) as f64);
    match g {
        GoalKind::Recruiting(m) => {
            let Team::College(p) = team else { return (0.5, None) };
            let best = (w.year - 1..=w.year).filter_map(|s| boss::class_rank(w, p, s)).min();
            match best {
                Some(r) => ((1.0 - (r as f64 - m as f64).max(0.0) / 6.0).clamp(0.0, 1.0), Some(r)),
                None => (0.5, None),
            }
        }
        GoalKind::Wins(m) => match &last {
            Some(s) => ((1.0 - (m as f64 - s.wins as f64).max(0.0) / 4.0).clamp(0.0, 1.0), Some(s.wins)),
            None => (0.5, None),
        },
        GoalKind::Postseason => (share.map_or(0.5, |x| (x / 0.55).min(1.0)), last.map(|s| s.wins)),
        GoalKind::Title => {
            let won = last.as_ref().is_some_and(|s| s.note.contains("champion") || s.note.contains("won"));
            (if won { 0.9 } else { share.map_or(0.4, |x| 0.2 + 1.2 * (x - 0.5).max(0.0)) }, last.map(|s| s.wins))
        }
        _ => (0.6, None),
    }
}

/// The season whose goals are in force: the latest one a goals meeting set,
/// from this season to the one the goals are for (`boss::goal_season`);
/// None before he sets any.
pub fn in_force(w: &World, team: Team) -> Option<i32> {
    let top = boss::goal_season(w, team);
    w.folk.goals.iter().filter(|g| !g.talk && g.season >= w.year && g.season <= top).map(|g| g.season).max()
}

/// Your boss's answer to time, asked in your words.
pub fn time(w: &World, team: Team) -> Answer {
    let Some(season) = in_force(w, team) else { return Answer::no(Why::NoBar) };
    if w.folk.goals.iter().any(|g| g.season >= w.year && g.deal == -1) {
        return Answer::no(Why::TimeAsked);
    }
    let b = boss::boss(w, team);
    let t = boss::trust(w, team) as f64;
    let days = w.folk.mine.since.map_or(9999, |d| d.days_to(w.today));
    let grace = if days < 400 {
        GRACE[0]
    } else if days < 760 {
        GRACE[1]
    } else {
        0.0
    };
    let raised = w.folk.goals.iter().any(|g| g.season >= w.year && g.deal == 1);
    let p = 0.45 + 0.05 * (b.patience - 10.0) + 0.6 * (t - 0.5) + grace - if raised { 0.3 } else { 0.0 };
    let mut drags = Vec::new();
    if raised {
        drags.push(Why::YourWord);
    }
    if b.patience < 10.0 {
        drags.push(Why::Impatient);
    }
    if t < 0.45 {
        drags.push(Why::Trust);
    }
    let roll = super::roll(w, 0x7A5D ^ w.today.0 as u64, season as u64);
    answer(roll < p.clamp(0.02, 0.95), None, drags)
}

/// Your boss's answer to backing you with the big booster.
pub fn backing(w: &World, team: Team, has_booster: bool) -> Answer {
    if !matches!(team, Team::College(_)) || !has_booster {
        return Answer::no(Why::NoBooster);
    }
    let b = boss::boss(w, team);
    let t = boss::trust(w, team) as f64;
    let p = 0.3 + 1.4 * (t - 0.45) + 0.02 * (b.warmth - 10.0) - 0.02 * (b.ego - 10.0);
    let mut drags = Vec::new();
    if t < 0.55 {
        drags.push(Why::Trust);
    }
    if b.ego >= 14.0 {
        drags.push(Why::Standing);
    }
    let roll = super::roll(w, 0x7A5E ^ w.today.0 as u64, 0);
    answer(roll < p.clamp(0.02, 0.95), None, drags)
}

/// A booster's answer to putting more into the collective: his trust in
/// you (`trust`) and what he wants; once in the window.
pub fn fund(w: &World, team: Team, him: &Cast, trust: f32) -> Answer {
    if !matches!(team, Team::College(_)) {
        return Answer::no(Why::NotHere);
    }
    if let Some(d) = gave_lately(w, him) {
        return Answer::no(Why::Gave(d));
    }
    let t = trust as f64;
    let wants = him.booster.as_ref().map(|x| x.wants.clone()).unwrap_or_default();
    let winning = facts::seasons(w, team, 1).last().is_some_and(|s| s.wins > s.losses);
    let mut p = 0.15 + 1.0 * (t - 0.4);
    if wants.contains(&Want::NilDeals) {
        p += 0.1;
    }
    let wins_first = wants.contains(&Want::WinNow) && !winning;
    p += if wants.contains(&Want::WinNow) { if winning { 0.1 } else { -0.05 } } else { 0.0 };
    let mut drags = Vec::new();
    if wins_first {
        drags.push(Why::WantsWins);
    }
    if t < 0.5 {
        drags.push(Why::Trust);
    }
    let roll = super::roll(w, 0x7A5F ^ w.today.0 as u64, him.key());
    answer(roll < p.clamp(0.02, 0.95), None, drags)
}

/// Days since this booster put money in, when it was in the window.
pub fn gave_lately(w: &World, him: &Cast) -> Option<u32> {
    let words = format!("{}{GIFT}", him.name);
    w.folk
        .asks
        .iter()
        .rev()
        .find(|a| a.kind == AskKind::NilMoney && a.granted && a.words == words && within(w, a))
        .map(|a| days(w, a))
}

/// A record's place: "1st", "9th".
fn nth(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// How long ago, in words.
fn ago(days: u32) -> String {
    match days {
        0 => "earlier today".into(),
        1..=6 => "a few days ago".into(),
        7..=13 => "a week ago".into(),
        14..=59 => format!("{} weeks ago", days / 7),
        _ => format!("{} months ago", days / 30),
    }
}

/// The record a doubt reads, in words for him ("the better of the last two
/// classes ranked 9th in the conference").
fn record(g: GoalKind, r: Option<u32>) -> String {
    match (g, r) {
        (GoalKind::Recruiting(_), Some(r)) => {
            format!("the better of the last two classes ranked {} in the conference", nth(r))
        }
        (GoalKind::Recruiting(_), None) => "there's no class to judge it by".into(),
        (_, Some(n)) => format!("the team won {n} {} last season", super::noun(n, "game")),
        _ => "the record doesn't back it".into(),
    }
}

/// Who answers, and what was asked, for his words.
pub struct Said<'a> {
    pub level: Level,
    /// What you asked for, in words ("more NIL and revenue-sharing money").
    pub what: &'a str,
    /// The pronoun of the one across the table, for the outcome line.
    pub she: bool,
}

/// What he says, for the voice: the game's answer and its reasons, with
/// no pronoun for either side (the voice says it as himself; a sit-down
/// with no key shows it as is). Replaces the plain "He'll do it." and "He
/// won't do that."
pub fn voice(s: &Said, a: &Answer) -> Vec<String> {
    let mut out = vec![format!("{} to {}.", if a.yes { "Yes" } else { "No" }, s.what)];
    for why in &a.why {
        out.push(match *why {
            Why::NotHere => "That money doesn't exist here.".into(),
            Why::NoBooster => "There's no big booster to go to.".into(),
            Why::AskedNo(d) => format!("It was turned down {}, and nothing has changed since.", ago(d)),
            Why::Stretched(d) => {
                format!("That money was found for the coach {}: the budget won't stretch twice in a year.", ago(d))
            }
            Why::Extended(d) => format!("The coach was extended {}: it's too soon for another.", ago(d)),
            Why::Budget => "The money isn't in the budget.".into(),
            Why::Tight => "The budget is kept tight, and it won't open for this.".into(),
            Why::Trust => "There isn't enough trust in the coach yet for this.".into(),
            Why::Doubts(g, r) => format!("Not convinced the coach will {}: {}.", g.words(s.level), record(g, r)),
            Why::TooSoon => "It's too early in the coach's contract to talk about an extension.".into(),
            Why::NoBar => "The season's goals aren't set yet: that talk is for the goals meeting.".into(),
            Why::TimeAsked => "Time was already asked for this season, and answered.".into(),
            Why::YourWord => "The coach took on a higher bar this season and is held to it.".into(),
            Why::Impatient => "No patience for a rebuild: the job is to win now.".into(),
            Why::Standing => "Standing with the booster isn't spent lightly.".into(),
            Why::Gave(d) => format!("Money went into the collective {}: that's enough for this year.", ago(d)),
            Why::WantsWins => "Wins come first.".into(),
            Why::Close => "It's close, but not right now.".into(),
            Why::Bar(g) => format!("The coach will be held to it: the goal is now to {}.", g.words(s.level)),
        });
    }
    out
}

/// His reasons, in the third person, for the scene's outcome line.
pub fn because(s: &Said, a: &Answer) -> String {
    let he = if s.she { "She" } else { "He" };
    let v: Vec<String> = a
        .why
        .iter()
        .map(|why| match *why {
            Why::NotHere => "There's no such money here.".into(),
            Why::NoBooster => "There's no big booster to go to.".into(),
            Why::AskedNo(d) => format!("{he} turned it down {} and nothing has changed.", ago(d)),
            Why::Stretched(d) => format!("{he} already found you this money {}.", ago(d)),
            Why::Extended(d) => format!("{he} already extended you {}.", ago(d)),
            Why::Budget => "The money isn't in the budget.".into(),
            Why::Tight => format!("{he} keeps the money tight."),
            Why::Trust => format!("{he} doesn't trust you enough yet."),
            Why::Doubts(g, r) => {
                format!("{he} doesn't believe you'll {}: {}.", g.words(s.level), record(g, r))
            }
            Why::TooSoon => "It's too early in your contract.".into(),
            Why::NoBar => "The season's goals aren't set yet: that's for the goals meeting.".into(),
            Why::TimeAsked => "You already asked for time this season.".into(),
            Why::YourWord => "You took on a higher bar this season.".into(),
            Why::Impatient => format!("{he} isn't a patient {}.", if s.she { "woman" } else { "man" }),
            Why::Standing => {
                format!("{he} doesn't spend {} standing with the booster lightly.", if s.she { "her" } else { "his" })
            }
            Why::Gave(d) => format!("{he} put money in {}.", ago(d)),
            Why::WantsWins => format!("{he} wants to see wins first."),
            Why::Close => "It was close.".into(),
            Why::Bar(g) => format!("{he}'ll hold you to it: the goal is now to {}.", g.words(s.level)),
        })
        .collect();
    v.join(" ")
}

/// What a granted money ask does, in words for the outcome line (the sizes
/// are `boss::grant`'s).
fn effect(k: AskKind) -> &'static str {
    match k {
        AskKind::StaffMoney => "The staff pool is 10% bigger, starting now.",
        AskKind::Facility => "A facility project breaks ground after the season.",
        AskKind::RecruitingMoney => "The recruiting budget is 10% bigger.",
        AskKind::NilMoney => "The collective puts in 8% more.",
        AskKind::RosterSpots => "You keep the extra practice-squad players.",
        AskKind::Extension => "Two more years on your contract, and a raise.",
    }
}

/// Puts an ask on his record (the meeting's 300-day window reads it).
fn file(w: &mut World, k: AskKind, granted: bool, words: String) {
    w.folk.asks.push(Ask { kind: k, date: w.today, granted, words });
    if w.folk.asks.len() > 40 {
        w.folk.asks.remove(0);
    }
}

/// The bar you put up on his goals for its season, in place of his own of
/// that kind, kept by the goals meeting; the public bar a little higher.
fn hold(w: &mut World, team: Team, g: GoalKind) {
    let season = bar_season(w, team, g);
    w.folk.goals.retain(|x| !(x.season == season && same(&x.kind, &g)));
    w.folk.goals.push(Goal { kind: g, season, deal: 1, met: None, set_on: w.today, talk: true });
    w.folk.mine.expectation = (w.folk.mine.expectation + BAR_EXPECT).min(1.0);
}

/// A yes you confirmed: the money moves (`boss::grant`), your bar goes on
/// his goals, and the ask is kept on record. Returns the outcome lines.
/// `she` is the pronoun of the one across the table. A second yes on the
/// same thing (two confirms, or asking again) moves nothing.
pub fn take(w: &mut World, team: Team, k: AskKind, case: &Case, she: bool) -> Vec<String> {
    let he = if she { "She" } else { "He" };
    if granted(w, k).is_some() {
        return vec![format!("You asked for {}: {he} already said yes to that, and it stands as it was.", k.words())];
    }
    let b = boss::boss(w, team);
    let held = on_table(w, team, case).bar;
    boss::grant(w, team, k);
    let mut out = vec![format!("You asked for {}: granted. {}", k.words(), effect(k))];
    let mut trust = -0.01;
    if let Some(g) = held {
        hold(w, team, g);
        trust += BAR_TRUST;
        out.push(format!("{he}'ll hold you to it: the goal is now to {}.", g.words(team.level())));
    }
    w.folk.mine.nudge(b.key(), trust);
    file(w, k, true, out.join(" "));
    out
}

/// A no: a little of his trust (the meeting's own -0.005), the ask on
/// record. Returns the outcome line.
pub fn refuse(w: &mut World, team: Team, k: AskKind, case: &Case, a: &Answer, she: bool) -> String {
    let b = boss::boss(w, team);
    let s = Said { level: team.level(), what: k.words(), she };
    let put = case.bar.map(|g| format!(", putting up a goal to {}", g.words(s.level))).unwrap_or_default();
    let line = format!("You asked for {}{put}: no. {}", k.words(), because(&s, a));
    w.folk.mine.nudge(b.key(), -0.005);
    file(w, k, false, line.clone());
    line
}

/// Time, settled on the season in force: a yes lowers its bar
/// (`boss::settle_time`); a no keeps it, noted, at a cost to his trust
/// (once: asking again costs nothing more). Before he sets any goals there
/// is nothing to settle. Returns the outcome line.
pub fn settle_time(w: &mut World, team: Team, a: &Answer, she: bool) -> String {
    let he = if she { "She" } else { "He" };
    let s = Said { level: team.level(), what: "more time", she };
    let Some(season) = in_force(w, team) else {
        return format!("{he} hasn't set the season's goals yet: that talk is for the goals meeting.");
    };
    if a.yes {
        boss::settle_time(w, season, true);
        return format!("{he} lowered the bar for the season.");
    }
    if a.why.contains(&Why::TimeAsked) {
        return format!("{he} kept the bar where it was. {}", because(&s, a));
    }
    let b = boss::boss(w, team);
    boss::settle_time(w, season, false);
    w.folk.mine.nudge(b.key(), TIME_NO[usize::from(b.patience >= 11.0)]);
    format!("{he} kept the bar where it was, and noted that you asked. {}", because(&s, a))
}
