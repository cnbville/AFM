//! What the talk is to him (Cain 2026-11-15: "give the AI targets"): who
//! you are to him, what rides on this talk for him, what he wants from it
//! (heaviest first), what he can point to, and how he takes it going
//! against him. Before this the voice knew only his name and role, so a
//! coordinator fighting for his job answered like a player asking about
//! his future.
//!
//! Words for the voice only, from where the world holds him (his kind, his
//! job, his deal, the meeting) and his own wants and makeup. No hidden
//! value leaves the engine; the only numbers are ones he'd know himself
//! (his years here, the team's record, the last season of his deal).
//! Every band and word here is Claude's (proposed, not confirmed).

use super::super::Them;
use super::super::makeup::{self, Makeup};
use super::super::wants::{Kind, Want, WantKind};
use crate::world::World;
use crate::world::folk::claims::rooms;
use crate::world::folk::scene::{Kind as SceneKind, Scene};
use crate::world::folk::{self as folk, Team, facts};

/// Where a holdover stands with you, the new head coach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holdover {
    /// You haven't said yet whether he stays.
    OnTheLine,
    /// You kept him for now; he's coaching for his job.
    Kept,
}

/// How long after you take over a holdover still feels on trial, in days
/// (about through spring ball). Proposed, not confirmed.
pub const HOLDOVER_DAYS: i32 = 240;

/// A coach the last staff left you, and where he stands: None for a man
/// you hired, one you let go, or once the takeover is long past.
pub fn holdover(w: &World, c: usize) -> Option<Holdover> {
    let t = w.folk.takeover.as_ref()?;
    let team = t.team?;
    if Team::seat(w) != Some(team) || makeup::coach_team(w, c) != Some(team) || makeup::your_coach(w) == Some(c) {
        return None;
    }
    if t.started.is_some_and(|d| d.days_to(w.today) > HOLDOVER_DAYS) || t.let_go.contains(&c) {
        return None;
    }
    if t.kept.contains(&c) {
        return Some(Holdover::Kept);
    }
    // A man you interviewed and hired is yours, not a holdover.
    if w.folk.staff.interviewed.iter().any(|x| x.0 == c) {
        return None;
    }
    // The takeover's meetings about him, or about the whole staff.
    w.folk
        .scenes
        .iter()
        .any(|s| s.kind == SceneKind::Holdover && (s.a == c as u64 || s.a == u64::MAX))
        .then_some(Holdover::OnTheLine)
}

/// How much keeping his job weighs for a coach on your staff, before the
/// wants are summed to 1 (`wanting::raw`): a holdover you haven't decided
/// on thinks of little else. Proposed, not confirmed.
pub fn job_weight(w: &World, c: usize) -> f32 {
    match holdover(w, c) {
        Some(Holdover::OnTheLine) => 0.6,
        Some(Holdover::Kept) => 0.25,
        None if w.folk.staff.deal(c).is_some_and(|d| d.through <= w.year) => 0.15,
        None => 0.05,
    }
}

/// A want of his in the second person, as the voice's card puts it.
pub fn you_want(k: WantKind, staff: bool) -> String {
    match k {
        WantKind::Role => "playing time, a real role".into(),
        WantKind::Money if staff => "your pay".into(),
        WantKind::Money => "the money".into(),
        WantKind::Security if staff => "keeping your job, on a deal that lasts".into(),
        WantKind::Security => "security: a place here that lasts".into(),
        WantKind::Winning => "winning".into(),
        WantKind::Place => "being close to home".into(),
        WantKind::Development => "coaching that gets you to the next level".into(),
        WantKind::Academics => "school and your degree".into(),
        WantKind::Name => "the name: the brand, being seen".into(),
        WantKind::Loyalty => "staying loyal to this place and its people".into(),
        WantKind::Family => "what's best for your family".into(),
        WantKind::Friends => "your friends and teammates".into(),
        WantKind::Fee => "your fee".into(),
        WantKind::Career if staff => "your own career: a bigger job one day, a title, calling plays".into(),
        WantKind::Career => "your own career: the next job, a title".into(),
        WantKind::Scheme => "your scheme and your say in it".into(),
        WantKind::Club => "your club: its value, its cap, your own job".into(),
        WantKind::Goals => "the goals you set him, the budget, the program's name".into(),
        WantKind::Booster(x) => format!("what your money buys: {}", x.short()),
        WantKind::Kids => "your kids' futures".into(),
    }
}

/// The most wants the card names. Proposed, not confirmed.
pub const MAX_WANTS: usize = 3;

/// Everything the voice's card says about what this talk is to him, in
/// order: who the coach is to him, what rides on it, what he wants, what
/// he can point to, how he takes a no.
pub fn stakes(w: &World, s: &Scene, him: Them, wants: &[Want]) -> Vec<String> {
    let kind = makeup::kind(w, him);
    let mut out = Vec::new();
    if let Some(x) = to_you(w, him, kind) {
        out.push(x);
    }
    let hold = match him {
        Them::Coach(c) if kind == Kind::Staff => holdover(w, c),
        _ => None,
    };
    if hold.is_some()
        && let Some(x) = who_is_who(w)
    {
        out.push(x);
    }
    if let Some(x) = this_talk(s, him, kind, hold) {
        out.push(x);
    }
    if kind == Kind::Boss && matches!(s.kind, SceneKind::BossMeeting | SceneKind::BossFirst) {
        out.extend(boss_talk(w, s));
    }
    let staff = matches!(kind, Kind::Staff | Kind::OtherCoach);
    let named: Vec<String> = wants
        .iter()
        .take(MAX_WANTS)
        .map(|x| {
            let w = you_want(x.kind, staff);
            if x.met { format!("{w} (he's given you that already)") } else { w }
        })
        .collect();
    if !named.is_empty() {
        out.push(format!("What you want, most first: {}", named.join("; ")));
    }
    let on_trial = hold.is_some() || s.kind == SceneKind::Firing;
    if let (Them::Coach(c), Kind::Staff) = (him, kind) {
        out.extend(his_case(w, c, on_trial));
        if on_trial {
            out.push(takes_a_no(&makeup::makeup(w, him)));
        }
    }
    out
}

/// Who's who in a takeover, so the voice never mixes the new head coach up
/// with the man he replaced: the man across the desk took the job, the
/// old head coach is gone.
fn who_is_who(w: &World) -> Option<String> {
    let t = w.folk.takeover.as_ref()?;
    let you = folk::scene::your_name(w);
    let mut out = format!("{you}, the man talking with you, is the new head coach here");
    if let Some(o) = t.old_coach.and_then(|o| w.coaches.get(o)) {
        let gone = o.job.is_none_or(|j| Some(Team::from_employer(j.employer)) != t.team);
        if gone {
            out += &format!(
                ". {} was the head coach before him and is gone: you worked for {}, and now you answer to {you}",
                o.name, o.name
            );
        }
    }
    Some(out)
}

/// Who the coach you're talking with is to him, by name, so the voice
/// never takes him for someone else.
fn to_you(w: &World, him: Them, kind: Kind) -> Option<String> {
    let yours = your_role(w);
    let you = folk::scene::your_name(w);
    Some(match kind {
        Kind::Staff if yours != "head coach" => {
            format!("{you}, the man talking with you, is a coach on the same staff as you, not your boss")
        }
        Kind::Staff => format!(
            "{you}, the man talking with you, is your boss, the {yours}: you work for him, and he decides whether you stay. You talk to him as a coach on his staff, not as a player"
        ),
        Kind::Boss => "You're his boss: you hired him, you judge his work and you decide his future".into(),
        Kind::Player => "He's your coach: you play for him, and he decides your role".into(),
        Kind::Recruit => "He's recruiting you: you haven't decided where you'll play, and he wants you".into(),
        Kind::Transfer => "You're in the transfer portal, and he wants you at his school".into(),
        Kind::Prospect => "His team may draft or sign you: he's sizing you up, and your future rides on it".into(),
        Kind::Family => {
            let son = match him {
                Them::Kin(id, _) => folk::person(w, id).map(|i| w.people[i].name.clone()),
                _ => None,
            };
            match son {
                Some(n) => format!("He coaches or recruits {n}, your family: you want what's best for him"),
                None => "He coaches or recruits your boy: you want what's best for him".into(),
            }
        }
        Kind::HsCoach => "He's a college coach after your players: your kids' futures come first".into(),
        Kind::Circle => "He's a coach after a young man you're close to".into(),
        Kind::Agent => {
            "You represent your client: your job is the best deal for him, and your fee comes from it".into()
        }
        Kind::Gm => "You run another club: any deal has to help your club".into(),
        Kind::OtherCoach => "He's a coach at another program: a peer, not your boss".into(),
        Kind::OtherScout => "You scout for another club: you answer to them, not to him".into(),
        Kind::Booster => "You give money to his program: you want it to buy what you care about".into(),
        Kind::Official => "You're a game official: you answer to the league, not to him".into(),
        Kind::Compliance => "You enforce the rules: you answer to the rules, not to him".into(),
        Kind::Outside => "You're not part of his program: you have your own job to do".into(),
    })
}

/// Your own job, in words: head coach unless you're an assistant.
pub fn your_role(w: &World) -> String {
    makeup::your_coach(w)
        .and_then(|c| w.coaches.get(c))
        .and_then(|c| c.job)
        .map_or_else(|| "head coach".to_string(), |j| j.role.name().to_string())
}

/// What rides on this meeting for him, by the meeting and his standing.
fn this_talk(s: &Scene, him: Them, kind: Kind, hold: Option<Holdover>) -> Option<String> {
    let about_him = matches!(him, Them::Coach(c) if s.a == c as u64 || s.a == u64::MAX);
    Some(match (s.kind, kind, hold) {
        (SceneKind::Holdover, Kind::Staff, Some(Holdover::OnTheLine)) if about_him => {
            "This meeting is about your job. He's the new head coach, and he's deciding whether to keep you or bring in his own man. You want to keep your job: make your case the way you would, ask what he needs from you, and if he lets you go, leave with your dignity".into()
        }
        (_, Kind::Staff, Some(Holdover::OnTheLine)) => {
            "Your job is on the line: he's the new head coach and hasn't said yet whether you stay. You want to keep it".into()
        }
        (_, Kind::Staff, Some(Holdover::Kept)) => {
            "He kept you on from the last staff for now, but you know you're coaching for your job this spring".into()
        }
        (SceneKind::Firing, Kind::Staff, _) => {
            "He called you in with the door closed, and you can tell it may be about your job. You want to keep it".into()
        }
        (SceneKind::Interview, _, _) => {
            "You're interviewing for a job on his staff: you want to show him you're the right man, if the job is right for you".into()
        }
        (SceneKind::StaffMeeting | SceneKind::Proposal, Kind::Staff, _) => {
            "You came in to argue for something you believe in, and you want him to say yes".into()
        }
        (SceneKind::Reference, _, _) => {
            "He's asking you about a coach you know: you tell what you know, as fairly as you would".into()
        }
        (SceneKind::JobInterview, _, _) => {
            "He's interviewing for your head coaching job: you want the right man for your program".into()
        }
        _ => return None,
    })
}

/// What rides on a meeting with your boss, for him (Cain 2026-11-16: the
/// athletic director asked the coach what he'd be judged on, in the meeting
/// where he sets it): the meeting he called and the goals on the table, in
/// his own terms. The goals meeting: he sets the goals for the season and
/// the coach is judged on them at the review; the coach may argue one down
/// or take on more, and what the coach commits to, he writes down.
fn boss_talk(w: &World, s: &Scene) -> Vec<String> {
    let Some(team) = s.team.or(Team::seat(w)) else { return Vec::new() };
    let level = team.level();
    let you = folk::scene::your_name(w);
    let goals_of = |season: i32| -> Vec<String> {
        w.folk
            .goals
            .iter()
            .filter(|g| g.season == season)
            .map(|g| {
                let words = folk::commit::goal_words(w, g.kind, level);
                if g.talk && g.deal == 1 { format!("{words} (his own bar: he took it on)") } else { words }
            })
            .collect()
    };
    let mut out = Vec::new();
    match s.c {
        // The season's goals meeting (and your first meeting).
        0 => {
            let season = folk::boss::goal_season(w, team);
            out.push(format!(
                "This is your goals meeting with {you}: you set his goals for the {season} season, the sheet in front of you both, and at the season's review you judge him on them. You lead with them, one by one, and why you want each"
            ));
            let g = goals_of(season);
            if !g.is_empty() {
                out.push(format!("Your goals for {season}: {}", g.join("; ")));
            }
            out.push(format!(
                "{you} may argue a goal down (he asks for time) or take on more: what he commits to that you can judge, you write down and hold him to. A plan or a slogan isn't a goal: ask him what you can measure it by"
            ));
        }
        1 => {
            let season = w.dates.season;
            let g = goals_of(season);
            out.push(format!(
                "This is the midseason check-in: you look at where the season stands against the goals you set{}",
                if g.is_empty() { String::new() } else { format!(" ({})", g.join("; ")) }
            ));
        }
        2 => out.push(format!(
            "This is the season review: you judge {you} on the goals you set for {}, the sheet in front of you both",
            s.b
        )),
        3 => out.push(format!("{you} asked for this meeting: hear what he wants, then decide")),
        _ => {}
    }
    out
}

/// What a coach of yours can point to, as facts he holds about himself
/// (`Topic::Job`, released when you ask about his job): his years here,
/// the team since he came, his unit's year, that he knows the players when
/// he's a holdover, and his deal.
pub fn case_facts(w: &World, c: usize) -> Vec<String> {
    let mut out = Vec::new();
    let role = facts::coach_role(w, c);
    if let Some(u) = rooms::unit(w, c) {
        out.push(format!(
            "he has been the {role} here {}",
            match u.years {
                1 => "one season".to_string(),
                n => format!("{n} seasons"),
            }
        ));
        if let Some(r) = u.record_words() {
            out.push(format!("the team is {r} since he took the job"));
        }
        if let Some(y) = u.year_words() {
            out.push(lower_first(y.trim_end_matches('.')));
        }
    }
    if holdover(w, c).is_some() {
        out.push("he knows these players and the recruits".into());
    }
    if let Some(d) = w.folk.staff.deal(c) {
        out.push(format!("his deal runs through the {} season", d.through));
    }
    out
}

/// What a coach of yours can point to: his years here, the team's record
/// since he took the job, his unit's year, and his deal.
fn his_case(w: &World, c: usize, on_trial: bool) -> Vec<String> {
    let mut out = Vec::new();
    let role = facts::coach_role(w, c);
    if let Some(u) = rooms::unit(w, c) {
        let mut case = format!(
            "What you can point to: you've been the {role} here {}",
            match u.years {
                1 => "one season".to_string(),
                n => format!("{n} seasons"),
            }
        );
        if let Some(r) = u.record_words() {
            case += &format!("; the team is {r} since you took the job");
        }
        if let Some(y) = u.year_words() {
            case += &format!("; {}", lower_first(y.trim_end_matches('.')));
        }
        if on_trial {
            case += "; you know these players and the recruits";
        }
        out.push(case);
    }
    if let Some(d) = w.folk.staff.deal(c) {
        let share = if d.buyout > 0.0 { d.buyout } else { 0.25 };
        let owed = match share {
            x if x >= 0.75 => "most of what's left on it",
            x if x >= 0.45 => "about half of what's left on it",
            _ => "a part of what's left on it",
        };
        out.push(format!("Your deal runs through the {} season; if he lets you go, he owes you {owed}", d.through));
    }
    out
}

/// How he takes it going against him, from his makeup.
fn takes_a_no(m: &Makeup) -> String {
    if m.temper >= 14.0 {
        "If it goes against you, you take it hard, and he hears it, though you stay a pro about it".into()
    } else if m.ego >= 14.0 {
        "You won't beg: you make your case on your record, and if he lets you go you tell him he's making a mistake"
            .into()
    } else if m.loyalty >= 13.0 {
        "If it goes against you, you ask about your players and your recruits before yourself".into()
    } else {
        "If it goes against you, you keep your dignity and ask what comes next: your deal, your players".into()
    }
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

/// What the talk is about from your side, for the talk page: only what a
/// coach in your seat would know of his situation (his standing with you,
/// the meeting, his deal, the portal, the draft), never a want by name or
/// a hidden number. Empty when there is nothing past an ordinary talk.
pub fn situation(w: &World, s: &Scene, him: Them) -> Vec<String> {
    let mut out = Vec::new();
    match him {
        Them::Coach(c) if makeup::kind(w, him) == Kind::Staff => {
            match (holdover(w, c), s.kind) {
                (Some(Holdover::OnTheLine), _) => out.push(
                    "He's a holdover from the last staff, and his job is on the line: you haven't said yet whether he stays."
                        .to_string(),
                ),
                (Some(Holdover::Kept), _) => {
                    out.push("You kept him from the last staff for now; he's coaching for his job.".to_string())
                }
                (None, SceneKind::Firing) => out.push("You called him in about his job.".to_string()),
                _ => {}
            }
            if let Some(d) = w.folk.staff.deal(c)
                && d.through <= w.year
            {
                out.push("His deal ends after this season.".to_string());
            }
        }
        Them::Player(id) => {
            let Some(i) = folk::person(w, id) else { return out };
            let p = &w.people[i];
            if p.cal.portal.is_some() {
                out.push("He's in the transfer portal.".to_string());
            }
            match p.stage {
                crate::world::Stage::Declared => out.push("He's declared for the draft.".to_string()),
                crate::world::Stage::College if w.year + 1 - p.class_of >= 3 => {
                    out.push("He's eligible for the next draft, so where he'd go is likely on his mind.".to_string())
                }
                _ => {}
            }
        }
        _ => {}
    }
    out
}
