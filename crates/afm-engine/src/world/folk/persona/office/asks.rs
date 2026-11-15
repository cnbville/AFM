//! What you ask of the people in your building (section 8.3), each run by
//! the built action a chip runs:
//!
//! - the team doctor: a second opinion (`life::second_opinion`) and the
//!   pressure to clear a hurt man (`life::rush_back`), weighed by
//!   `doctor::clear_score`;
//! - a coordinator: a plan setting on his side of the ball, weighed as you
//!   said it (`plan_ask`: which setting, how far, asked or ordered) against
//!   his own read (`plan_view`: his unit's last game, the case he's making,
//!   how sure he is, his trust in you), and put in by the staff meeting's
//!   own step (`dialogue::shift_plan`), kept as its case;
//! - your boss: money (the staff pool, recruiting, the collective, a
//!   facility, practice-squad money), a hire's money, time and his backing
//!   with the big booster, answered by the meeting's own odds and roll
//!   (`boss::ask_yes`, `ask_now`, `ask_time`);
//! - a booster: money for the collective;
//! - the compliance office: what the rules allow.
//!
//! Anything else falls to lane N3's requests.

use super::super::Them;
use super::super::ear::Turn;
use super::super::terms::{Give, Request};
use super::super::wants::{Kind as WhoKind, Ran, WantKind, Willing};
use super::doctor;
use crate::calendar::Level;
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::folk::boss::{self, AskKind};
use crate::world::folk::cast::{self, Role as CastRole};
use crate::world::folk::{self as folk, Team};

/// A coach whose job is with your team (not you).
pub fn on_your_staff(w: &World, c: usize) -> bool {
    let Some(job) = w.coaches.get(c).and_then(|x| x.job) else { return false };
    let you = w.career.as_ref().map(|x| x.coach);
    you != Some(c)
        && match (Team::seat(w), job.employer) {
            (Some(Team::College(p)), Employer::College(q)) => p == q,
            (Some(Team::Nfl(t)), Employer::Nfl(q)) => t == q,
            (Some(Team::High(h)), Employer::HighSchool(q)) => h == q,
            (Some(Team::Other(f)), Employer::Other(q)) => f == q,
            _ => false,
        }
}

/// The boss ask a money request is, from what you said it's for: the staff
/// pool unless you named recruiting, the collective, a facility or
/// practice-squad money.
pub fn money_kind(words: &str) -> AskKind {
    let w = words.to_lowercase();
    let has = |xs: &[&str]| xs.iter().any(|x| w.contains(x));
    if has(&["facilit", "building", "weight room", "stadium", "locker room", "indoor"]) {
        AskKind::Facility
    } else if has(&["nil", "collective", "revenue"]) {
        AskKind::NilMoney
    } else if has(&["recruit", "travel", "visit"]) {
        AskKind::RecruitingMoney
    } else if has(&["practice squad", "practice-squad", "roster"]) {
        AskKind::RosterSpots
    } else {
        AskKind::StaffMoney
    }
}

/// The sit-down's plan chips, by the staff meeting's move (`dialogue::
/// MOVES`): a click is the plain ask, one step, as the chip always was.
pub const CHIP_WORDS: [&str; 6] = [
    "Bring more pressure.",
    "Play more zone.",
    "Run the ball more.",
    "Throw it more.",
    "Back off the blitz.",
    "Play more man.",
];

/// A plan ask as you said it: the staff meeting's move (0 more pressure, 4
/// less; 1 more zone, 5 more man; 2 run more, 3 pass more), how far in the
/// meeting's own steps (`dialogue::plan_step`: a little is half a step, a
/// lot two, a share the steps it takes to get there), whether you made it
/// your call rather than asked, and whether it was the sit-down's chip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanAsk {
    pub m: u8,
    pub steps: f64,
    pub order: bool,
    pub chip: bool,
}

/// The most steps one ask moves the plan in a week. Proposed, not
/// confirmed.
pub const MAX_STEPS: f64 = 3.0;
/// A little and a lot, in steps. Proposed, not confirmed.
pub const LITTLE: f64 = 0.5;
pub const LOT: f64 = 2.0;

/// A share said as a number: "40%", "40 percent", or ".4".
fn share_of(w: &str) -> Option<f64> {
    let b = w.as_bytes();
    for (k, _) in w.match_indices(|c: char| c.is_ascii_digit() || c == '.') {
        if k > 0 && (b[k - 1].is_ascii_digit() || b[k - 1] == b'.') {
            continue;
        }
        let num: String = w[k..].chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        let rest = w[k + num.len()..].trim_start();
        let Ok(x) = num.trim_end_matches('.').parse::<f64>() else { continue };
        if rest.starts_with('%') || rest.starts_with("percent") || rest.starts_with("per cent") {
            return Some((x / 100.0).clamp(0.0, 1.0));
        }
        if x > 0.0 && x < 1.0 && (num.starts_with('.') || num.starts_with("0.")) {
            return Some(x.clamp(0.0, 1.0));
        }
    }
    None
}

/// A plan ask from your words (the ear's value: the setting, more or less,
/// how much, and whether you made it your call). None when it names no
/// setting the plan has.
pub fn plan_ask(w: &World, words: &str) -> Option<PlanAsk> {
    if let Some(m) = CHIP_WORDS.iter().position(|c| *c == words) {
        return Some(PlanAsk { m: m as u8, steps: 1.0, order: false, chip: true });
    }
    let low = words.to_lowercase();
    let has = |xs: &[&str]| xs.iter().any(|x| low.contains(x));
    let word = |x: &str| low.split(|c: char| !c.is_alphanumeric()).any(|t| t == x);
    let less = has(&["less", "fewer", "back off", "stop", "cut", "not as", "lower", "down to", "reduce"]);
    // The setting, and its two moves (up, down) on the plan's own scale.
    let (setting, up, down) = if has(&["blitz", "pressure", "send more", "bring more", "rush more", "pass rush"]) {
        ("pressure", 0u8, 4u8)
    } else if word("zone") {
        ("zone", 1, 5)
    } else if word("man") || has(&["man coverage", "man-to-man", "press man"]) {
        ("man", 5, 1)
    } else if has(&["throw", "pass"]) {
        ("pass", 3, 2)
    } else if word("run") || has(&["running", "run the ball", "ground"]) {
        ("run", 2, 3)
    } else {
        return None;
    };
    let order = has(&[
        "order",
        "my call",
        "i want",
        "do it",
        "just do",
        "final",
        "insist",
        "make it happen",
        "i'm telling",
        "not asking",
    ]);
    // A share said: the steps it takes to get there from where the plan is.
    if let Some(x) = share_of(&low) {
        // Zone is man coverage's other side, passing the run share's.
        let target = match setting {
            "zone" | "pass" => 1.0 - x,
            _ => x,
        };
        let key = if setting == "pressure" {
            0
        } else if matches!(setting, "zone" | "man") {
            5
        } else {
            2
        };
        let (now, _) = crate::world::folk::dialogue::plan_step(w, key, 0.0);
        let step = crate::world::folk::dialogue::PLAN_STEP[key as usize];
        let raise = target > now;
        let m = match key {
            0 => {
                if raise {
                    0
                } else {
                    4
                }
            }
            5 => {
                if raise {
                    5
                } else {
                    1
                }
            }
            _ => {
                if raise {
                    2
                } else {
                    3
                }
            }
        };
        let steps = ((target - now).abs() / step).min(MAX_STEPS);
        return Some(PlanAsk { m, steps, order, chip: false });
    }
    let steps = if has(&["a little", "a bit", "a touch", "slightly", "some more", "a tad", "nudge", "tweak"]) {
        LITTLE
    } else if has(&[
        "a lot",
        "lot more",
        "way more",
        "much more",
        "heavy",
        "every down",
        "all game",
        "a ton",
        "big",
        "everything",
        "all the time",
    ]) {
        LOT
    } else {
        1.0
    };
    Some(PlanAsk { m: if less { down } else { up }, steps, order, chip: false })
}

/// A plan move's side: true offense (the run share), false defense.
fn move_offense(m: u8) -> bool {
    matches!(m, 2 | 3)
}

/// The move the other way (more pressure and less, zone and man, run and
/// pass).
fn opposite(m: u8) -> u8 {
    match m {
        0 => 4,
        4 => 0,
        1 => 5,
        5 => 1,
        2 => 3,
        _ => 2,
    }
}

fn clamp1(x: f64) -> f64 {
    x.clamp(-1.0, 1.0)
}

/// What a coordinator reads from his unit's last game, toward one move:
/// -1 (the game says no) to 1 (the game says yes), and the numbers he
/// points at, in words. A defense: how often its rush got home, the big
/// plays it gave up and the points; an offense: yards a carry, how often
/// it ran and the points. Proposed, not confirmed.
fn game_lean(w: &World, m: u8) -> (f64, Vec<String>) {
    let Some((_, me)) = w.seat_team() else { return (0.0, Vec::new()) };
    let Some(g) = w
        .seat_games
        .iter()
        .rev()
        .find(|g| (g.teams[0] == me || g.teams[1] == me) && g.date <= w.today && g.date.days_to(w.today) <= 14)
    else {
        return (0.0, Vec::new());
    };
    let side = if g.teams[0] == me { 0 } else { 1 };
    let st = &g.stats;
    let (ours, theirs) = (&st.team[side], &st.team[1 - side]);
    if move_offense(m) {
        let carries = ours.plays.saturating_sub(ours.dropbacks).max(1) as f64;
        let ypc = ours.rush_yards / carries;
        let share = carries / ours.plays.max(1) as f64;
        let scored = st.score[side];
        // Running well and not often says run more; already running a lot,
        // or getting nothing on the ground, says throw.
        let run = 0.6 * clamp1((ypc - 4.2) / 1.0) - 0.4 * clamp1((share - 0.44) / 0.1);
        // A unit that scored 28 doesn't want to change much at all.
        let works = 0.3 * clamp1((scored as f64 - 24.0) / 8.0).max(0.0);
        let lean = if m == 2 { run } else { -run } - works;
        let words = vec![format!(
            "Last game: {:.1} yards a carry, runs on {}% of the plays, {} points.",
            ypc,
            (100.0 * share).round() as i64,
            scored
        )];
        (clamp1(lean), words)
    } else {
        let pressure = theirs.pressures as f64 / theirs.dropbacks.max(1) as f64;
        let big = theirs.explosive as f64;
        let allowed = st.score[1 - side];
        // Not getting home says send more; giving up big plays behind it
        // says sit back and keep it in front (zone).
        let press = 0.6 * clamp1((0.30 - pressure) / 0.10) - 0.4 * clamp1((big - 4.0) / 3.0);
        let zone = 0.7 * clamp1((big - 4.0) / 3.0);
        let works = 0.3 * clamp1((21.0 - allowed as f64) / 8.0).max(0.0);
        let lean = match m {
            0 => press,
            4 => -press,
            1 => zone,
            _ => -zone,
        } - works;
        let words = vec![format!(
            "Last game: their passer pressured on {}% of dropbacks, {} big plays given up, {} points allowed.",
            (100.0 * pressure).round() as i64,
            theirs.explosive,
            allowed
        )];
        (clamp1(lean), words)
    }
}

/// The case he is making this week, if any: an open staff meeting where
/// he has the floor, else last meeting's case of his you haven't settled
/// in his favour, within a week.
fn his_case(w: &World, c: usize) -> Option<u8> {
    let open = w
        .folk
        .scenes
        .iter()
        .find(|s| !s.done && s.kind == crate::world::folk::scene::Kind::StaffMeeting && s.a as usize == c)
        .map(|s| (s.b as u8).min(3));
    open.or_else(|| {
        w.folk
            .dialogue
            .case
            .filter(|x| x.coach == c && x.said != 0 && x.mine.is_none() && x.date.days_to(w.today) <= 7)
            .map(|x| x.kind)
    })
}

/// How your coordinator takes a plan ask: his view (-1 against, 1 he
/// wanted it), and what he says about it. From his unit's last game, the
/// case he is making, how sure of himself he is (his football mind and his
/// ego), his trust in you and how big a change you asked for. Proposed,
/// not confirmed.
pub fn plan_view(w: &World, c: usize, ask: &PlanAsk) -> (f64, Vec<String>) {
    let him = Them::Coach(c);
    let mk = super::super::makeup::makeup(w, him);
    let z = super::super::makeup::Makeup::z;
    let (lean, mut words) = game_lean(w, ask.m);
    let case = his_case(w, c);
    let pushed = match case {
        Some(k) if k == ask.m => 0.8,
        Some(k) if opposite(k) == ask.m => -0.8,
        _ => 0.0,
    };
    if let Some(k) = case {
        words.push(format!("He has been asking for {} himself.", crate::world::folk::dialogue::MOVES[k as usize]));
    }
    // A sure man holds his read harder, for or against.
    let sure = (1.0 + 0.3 * z(mk.iq) + 0.3 * z(mk.ego)).clamp(0.5, 1.6);
    let size = -0.15 * (ask.steps - 1.0).max(0.0) + if ask.steps <= LITTLE { 0.1 } else { 0.0 };
    let trust = 0.4 * (super::trust(w, him) as f64 - 0.5);
    (clamp1((lean + pushed) * sure + size + trust), words)
}

/// At or over this he's for it; down to `PUSH_BACK` he'd go halfway,
/// below it he'd stay with the plan, unless you make it your call.
/// Proposed, not confirmed.
pub const AGREES: f64 = -0.15;
pub const PUSH_BACK: f64 = -0.45;

/// The plan's guideline for a move now and where the ask takes it, in
/// words ("pressure from 30% to 46%").
fn plan_words(w: &World, m: u8, steps: f64) -> String {
    let (now, to) = crate::world::folk::dialogue::plan_step(w, m, steps);
    let pct = |x: f64| (100.0 * x).round() as i64;
    let what = crate::world::folk::dialogue::PLAN_WORDS[m as usize].to_lowercase();
    format!("{what} from {}% to {}%", pct(now), pct(to))
}

/// Whether the plan can move at all that way (it's at its end).
fn room_to_move(w: &World, m: u8, steps: f64) -> bool {
    let (now, to) = crate::world::folk::dialogue::plan_step(w, m, steps.max(0.25));
    (to - now).abs() > 0.004
}

/// What a coordinator says about a plan ask, beside his answer: his side of
/// the ball, the numbers he points at, and how he takes it. Empty for any
/// other ask or person.
pub fn words(w: &World, him: Them, ask: &Request, wl: &Willing) -> Vec<String> {
    let (Them::Coach(c), Request::Plan(said)) = (him, ask) else { return Vec::new() };
    if !on_your_staff(w, c) {
        return Vec::new();
    }
    let Some(a) = plan_ask(w, said) else {
        return vec!["He asks what you want changed: the pressure, the coverage, or the run and pass mix.".into()];
    };
    if a.chip {
        return Vec::new();
    }
    let role = w.coaches[c].job.map(|j| j.role);
    let his_side = match role {
        Some(Role::OffensiveCoordinator) => move_offense(a.m),
        Some(Role::DefensiveCoordinator) => !move_offense(a.m),
        _ => false,
    };
    if !his_side {
        let other = if move_offense(a.m) { "offensive" } else { "defensive" };
        return vec![format!("That's the {other} coordinator's call, not his.")];
    }
    if !room_to_move(w, a.m, a.steps) {
        return vec![format!("The plan is as far that way as it goes: {}.", plan_words(w, a.m, 0.0))];
    }
    let (view, mut out) = plan_view(w, c, &a);
    out.push(match wl {
        Willing::Yes if view >= AGREES => format!("He's for it: {}.", plan_words(w, a.m, a.steps)),
        Willing::Yes => format!(
            "He doesn't like it and says so, but it's your call: {}. It's on you if it goes wrong.",
            plan_words(w, a.m, a.steps)
        ),
        _ => String::new(),
    });
    out.retain(|x| !x.is_empty());
    out
}

/// His counter to a plan ask he won't take as it is: halfway, or the plan
/// as it stands, with his numbers.
fn plan_counter(w: &World, a: &PlanAsk, view: f64) -> super::super::terms::Counter {
    let ask = if view >= PUSH_BACK && a.steps > LITTLE * 0.5 {
        format!(
            "He'd go halfway: {}. If you want all of it, it's your call to make.",
            plan_words(w, a.m, a.steps / 2.0)
        )
    } else {
        "He'd stay with the plan as it is. If you want it anyway, it's your call to make.".into()
    };
    super::super::terms::Counter { asks: vec![ask], ..Default::default() }
}

/// How willing a coordinator is to make a plan change (the sit-down's chip
/// is the plain ask he always took on his side of the ball).
fn plan_willing(w: &World, c: usize, words: &str) -> Option<Willing> {
    let a = plan_ask(w, words)?;
    let role = w.coaches[c].job.map(|j| j.role);
    let his_side = match role {
        Some(Role::OffensiveCoordinator) => move_offense(a.m),
        Some(Role::DefensiveCoordinator) => !move_offense(a.m),
        _ => false,
    };
    if !his_side || !room_to_move(w, a.m, a.steps) {
        return Some(Willing::No);
    }
    if a.chip {
        return Some(Willing::Yes);
    }
    let (view, _) = plan_view(w, c, &a);
    Some(if view >= AGREES || a.order { Willing::Yes } else { Willing::If(plan_counter(w, &a, view)) })
}

/// What it costs his trust in you to put in a change he argued against,
/// at most the office's floor a line (`CAP_DOWN`): more the more he was
/// against it, the more ego and temper he has, less the more professional
/// he is. Proposed, not confirmed.
pub fn protest_cost(w: &World, c: usize, view: f64) -> f32 {
    let mk = super::super::makeup::makeup(w, Them::Coach(c));
    let z = super::super::makeup::Makeup::z;
    let against = (AGREES - view).max(0.0);
    let feel = (1.0 + 0.5 * z(mk.ego) + 0.3 * z(mk.temper) - 0.3 * z(mk.professionalism)).clamp(0.4, 1.8);
    (-(0.01 + 0.03 * against) * feel).max(f64::from(super::CAP_DOWN)) as f32
}

/// Puts a plan change in, as he agreed or as you ordered: the plan moves by
/// the steps you asked for, and the week is kept as the staff meeting
/// keeps a case (`dialogue::Case`): one he was for is his to own, one he
/// argued against is your call, and his unit's next game settles it.
fn plan_run(w: &mut World, c: usize, words: &str) -> Option<Ran> {
    let a = plan_ask(w, words)?;
    let mut ran = Ran { honest: true, ..Ran::default() };
    let name = w.coaches[c].name.clone();
    if a.chip {
        crate::world::folk::dialogue::shift_plan(w, a.m, 1.0);
        ran.done
            .push(format!("{name} changes the plan: {} this week.", crate::world::folk::dialogue::MOVES[a.m as usize]));
        ran.report.push("You got it. It's in the plan.".into());
        return Some(ran);
    }
    let (view, _) = plan_view(w, c, &a);
    let moved = plan_words(w, a.m, a.steps);
    crate::world::folk::dialogue::shift_plan(w, a.m, a.steps);
    let side_kind = if move_offense(a.m) { a.m } else { a.m.min(1) };
    let before = before_points(w, a.m);
    let agreed = view >= AGREES;
    w.folk.dialogue.case = Some(crate::world::folk::dialogue::Case {
        coach: c,
        kind: side_kind,
        said: if agreed { 0 } else { 2 },
        date: w.today,
        before,
        mine: (!agreed).then_some(a.m),
    });
    if agreed {
        ran.done.push(format!("{name} puts it in the plan: {moved} this week."));
        ran.report.push(format!("He's for it: {moved}."));
    } else {
        let cost = protest_cost(w, c, view);
        super::nudge(w, Them::Coach(c), cost);
        ran.done.push(format!(
            "{name} puts it in the plan because you told him to: {moved} this week. He argued against it."
        ));
        ran.report.push(format!("He'll put it in, {moved}, and he wants it known it was your call."));
    }
    Some(ran)
}

/// His unit's points in the last game (scored for the offense, allowed for
/// the defense), as the staff meeting keeps them.
fn before_points(w: &World, m: u8) -> i32 {
    let Some((_, me)) = w.seat_team() else { return 0 };
    let Some(g) = w.seat_games.iter().rev().find(|g| g.teams[0] == me || g.teams[1] == me) else { return 0 };
    let side = if g.teams[0] == me { 0 } else { 1 };
    if move_offense(m) { g.stats.score[side] } else { g.stats.score[1 - side] }
}

/// Your boss at your seat, when he's the man across.
fn your_boss(w: &World, him: Them) -> Option<Team> {
    let team = Team::seat(w)?;
    (super::super::makeup::kind(w, him) == WhoKind::Boss && matches!(him, Them::Cast(..))).then_some(team)
}

/// Whether you already asked him for time this season (the goals meeting
/// keeps it on the goals).
fn asked_time(w: &World, team: Team) -> bool {
    let season = match team.level() {
        Level::Nfl | Level::College | Level::HighSchool => w.year,
        _ => w.year,
    };
    w.folk.goals.iter().any(|g| g.season >= season && g.deal == -1)
}

/// How willing he is, for the people and asks this lane plays; None
/// leaves it to lane N3.
pub fn willing(w: &World, him: Them, ask: &Request) -> Option<Willing> {
    let team = Team::seat(w)?;
    let yes = |b: bool| if b { Willing::Yes } else { Willing::No };
    if doctor::is_doctor(w, him) {
        return match ask {
            Request::SecondOpinion(id) => {
                let i = folk::person(w, *id)?;
                Some(yes(doctor::chart(w, i).hurt()))
            }
            Request::Clear(id) => {
                let i = folk::person(w, *id)?;
                if !doctor::chart(w, i).hurt() {
                    return Some(Willing::No);
                }
                Some(yes(doctor::clear_score(w, him, team) >= doctor::CLEARS))
            }
            _ => None,
        };
    }
    if let Them::Coach(c) = him
        && on_your_staff(w, c)
    {
        return match ask {
            Request::Plan(words) => plan_willing(w, c, words),
            _ => None,
        };
    }
    if let Some(team) = your_boss(w, him) {
        let level = team.level();
        let money = |k: AskKind| Some(yes(k.at(level) && !boss::asked_lately(w, k) && boss::ask_yes(w, team, k)));
        return match ask {
            Request::Money(words) => money(money_kind(words)),
            Request::Hire(_) => money(AskKind::StaffMoney),
            Request::Time => {
                let b = boss::boss(w, team);
                Some(yes(!asked_time(w, team) && b.patience >= 11.0))
            }
            Request::Backing => {
                let Team::College(_) = team else { return Some(Willing::No) };
                Some(yes(boss::trust(w, team) >= BACKING_TRUST && big_booster(w, team).is_some()))
            }
            _ => None,
        };
    }
    if super::super::makeup::kind(w, him) == WhoKind::Booster
        && let Them::Cast(r, _) = him
        && r.team == team
    {
        return match ask {
            Request::Fund => {
                let Team::College(_) = team else { return Some(Willing::No) };
                let t = super::trust(w, him);
                Some(yes(t >= FUND_TRUST && !boss::asked_lately(w, AskKind::NilMoney)))
            }
            _ => None,
        };
    }
    if matches!(him, Them::Drawn(WhoKind::Compliance, _)) {
        return match ask {
            Request::RulesQuestion => Some(Willing::Yes),
            _ => None,
        };
    }
    None
}

/// Your boss's trust he needs before he'll vouch for you to the big
/// booster. Proposed, not confirmed.
pub const BACKING_TRUST: f32 = 0.5;
/// What his backing does: the big booster's trust in you, and what it
/// costs your boss's. Proposed, not confirmed.
pub const BACKING_LIFT: f32 = 0.04;
pub const BACKING_COST: f32 = -0.01;
/// A booster's trust in you before he'll put money in the collective, and
/// how much more it holds (the boss's own NIL yes is 8%). Proposed, not
/// confirmed.
pub const FUND_TRUST: f32 = 0.5;
pub const FUND_RAISE: f64 = 0.05;

fn big_booster(w: &World, team: Team) -> Option<cast::Cast> {
    cast::all(w, team, CastRole::Booster).into_iter().find(|c| c.booster.as_ref().is_some_and(|b| b.big))
}

/// Runs what he agreed to, for the people and asks this lane plays; None
/// leaves it to lane N3.
pub fn run(w: &mut World, him: Them, ask: &Request) -> Option<Ran> {
    let team = Team::seat(w)?;
    let mut ran = Ran { honest: true, ..Ran::default() };
    if doctor::is_doctor(w, him) {
        match ask {
            Request::SecondOpinion(id) => {
                let line = crate::world::folk::life::second_opinion(w, *id);
                ran.done.push(line);
                ran.report.push("I'll set it up with the specialist.".into());
            }
            Request::Clear(id) => {
                let (cleared, line) = crate::world::folk::life::rush_back(w, team, *id);
                ran.done.push(line);
                ran.report.push(if cleared {
                    "He's cleared. It's on you if it goes again.".into()
                } else {
                    "No. Not with this injury.".into()
                });
            }
            _ => return None,
        }
        return Some(ran);
    }
    if let Them::Coach(c) = him
        && on_your_staff(w, c)
    {
        let Request::Plan(words) = ask else { return None };
        return plan_run(w, c, words);
    }
    if let Some(team) = your_boss(w, him) {
        let k = match ask {
            Request::Money(words) => money_kind(words),
            Request::Hire(_) => AskKind::StaffMoney,
            Request::Time => {
                let gave = boss::ask_time(w, team);
                ran.done.push(if gave {
                    "He lowered the bar for this year.".into()
                } else {
                    "He kept the bar where it was, and noted that you asked.".into()
                });
                ran.report.push(if gave {
                    "All right. A year to build. But I want to see it moving.".into()
                } else {
                    "I didn't hire you to ask for time.".into()
                });
                return Some(ran);
            }
            Request::Backing => {
                let b = boss::boss(w, team);
                if let Some(big) = big_booster(w, team) {
                    w.folk.mine.nudge(big.key(), BACKING_LIFT);
                    w.folk.mine.nudge(b.key(), BACKING_COST);
                    ran.done.push(format!("{} talked to {} for you.", b.name, big.name));
                    ran.report.push("I'll make the call. He'll hear it from me.".into());
                }
                return Some(ran);
            }
            _ => return None,
        };
        let (yes, words) = boss::ask_now(w, team, k);
        ran.done.push(format!("You asked for {}: {}.", k.words(), if yes { "granted" } else { "no" }));
        ran.report.push(words.into());
        return Some(ran);
    }
    if super::super::makeup::kind(w, him) == WhoKind::Booster {
        let Request::Fund = ask else { return None };
        let (Team::College(p), Them::Cast(r, _)) = (team, him) else { return None };
        w.programs[p].collective.base *= 1.0 + FUND_RAISE;
        let name = cast::get(w, r).name;
        w.folk.asks.push(boss::Ask {
            kind: AskKind::NilMoney,
            date: w.today,
            granted: true,
            words: format!("{name} put more into the collective."),
        });
        ran.done.push(format!("{name} puts more money into the collective."));
        ran.report.push("I'll make some calls. You'll see it.".into());
        return Some(ran);
    }
    if matches!(him, Them::Drawn(WhoKind::Compliance, _)) {
        let Request::RulesQuestion = ask else { return None };
        ran.report.extend(super::rules::words(w));
        ran.done.push("Compliance walked you through what's allowed today.".into());
        return Some(ran);
    }
    None
}

/// After a turn: a no from your boss to a money ask is kept as the
/// meeting keeps it (`ask_now`: the ask on record, a little trust), a no
/// to time costs what the goals meeting's does (`ask_time`), and a doctor
/// pushed to clear a man holds it against you (`doctor::REFUSED_*`).
pub fn after(w: &mut World, him: Them, t: &Turn) -> Vec<String> {
    let mut out = Vec::new();
    let Some(team) = Team::seat(w) else { return out };
    for (r, wl, _) in &t.asked {
        if *wl != Willing::No {
            continue;
        }
        if doctor::is_doctor(w, him) {
            if let Request::Clear(id) = r
                && folk::person(w, *id).is_some_and(|i| doctor::chart(w, i).hurt())
            {
                let by = if crate::world::folk::life::would_rush(w, team) {
                    doctor::REFUSED_PUSHED
                } else {
                    doctor::REFUSED_JUDGMENT
                };
                super::nudge(w, him, by);
                out.push("He won't clear him. The timeline holds.".into());
            }
            continue;
        }
        if your_boss(w, him).is_some() {
            let k = match r {
                Request::Money(words) => Some(money_kind(words)),
                Request::Hire(_) => Some(AskKind::StaffMoney),
                _ => None,
            };
            if let Some(k) = k
                && k.at(team.level())
                && !boss::asked_lately(w, k)
            {
                let (_, words) = boss::ask_now(w, team, k);
                out.push(format!("You asked for {}: no. \"{words}\"", k.words()));
            }
            if matches!(r, Request::Time) && !asked_time(w, team) {
                boss::ask_time(w, team);
                out.push("He kept the bar where it was, and noted that you asked.".into());
            }
        }
    }
    out
}

/// What one of your assistants asks for as a thing the game can do, when
/// he raises a want himself (lane N4's `HisAsk.gives`): his money is a
/// raise of `HIS_RAISE`, his next step a title, his say in the scheme the
/// play-calling (a coordinator who doesn't call it). Empty for anyone else.
pub fn his_gives(w: &World, him: Them, k: WantKind) -> Vec<Give> {
    let Them::Coach(c) = him else { return Vec::new() };
    if !on_your_staff(w, c) {
        return Vec::new();
    }
    let salary = w.folk.staff.deals.iter().find(|d| d.coach == c).map(|d| d.salary);
    let coordinator =
        w.coaches[c].job.is_some_and(|j| matches!(j.role, Role::OffensiveCoordinator | Role::DefensiveCoordinator));
    match k {
        WantKind::Money => salary.map(|s| vec![Give::Money(s * (1.0 + HIS_RAISE))]).unwrap_or_default(),
        WantKind::Career if !w.folk.hiring.titles.iter().any(|x| x.0 == c) => vec![Give::Title],
        WantKind::Scheme if coordinator && !calls_now(w, c) => vec![Give::Calling],
        _ => Vec::new(),
    }
}

/// The raise an assistant asks for himself. Proposed, not confirmed.
pub const HIS_RAISE: f64 = 0.1;

/// Whether your coordinator already calls his side's plays.
fn calls_now(w: &World, c: usize) -> bool {
    let Some(you) = w.career.as_ref().map(|k| k.coach) else { return false };
    let Some(y) = w.coaches[you].yours.as_ref() else { return false };
    match w.coaches[c].job.map(|j| j.role) {
        Some(Role::OffensiveCoordinator) => !y.calls_offense,
        Some(Role::DefensiveCoordinator) => !y.calls_defense,
        _ => false,
    }
}
