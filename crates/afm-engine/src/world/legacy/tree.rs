//! G2, your coaching tree: the coaches who learned under you, their own
//! branches, and how you grow the men on your staff now. The tree is read
//! from the world's own records: a coach's mentor (the head coach he
//! learned under longest), the stints he worked on your staff, and the
//! jobs and records since. A high school assistant of yours can become a
//! head coach in your state when a school's search reads his résumé.
//! Rewards come when a branch earns them (his first head job, his titles),
//! and in the NFL the league's own compensatory picks for a minority
//! coach hired away are counted on the tree. Sizes are proposed, not
//! confirmed.

use super::{Bank, key, memo, pay};
use crate::world::World;
use crate::world::coach::{Coach, Employer, Group, Role};
use crate::world::storylines::catalogue::Rating;

/// How you grow an assistant (board 34's "grow" choices).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Grow {
    /// He calls his side's plays (your play calling setting, read here).
    Calls,
    /// A recruiting area of his own.
    Recruiting,
    /// Clinics and visits to other staffs in the off-season.
    Clinics,
}

impl Grow {
    pub fn key(self) -> &'static str {
        match self {
            Grow::Calls => "calls",
            Grow::Recruiting => "recruiting",
            Grow::Clinics => "clinics",
        }
    }
    pub fn from_key(s: &str) -> Option<Grow> {
        Some(match s {
            "calls" => Grow::Calls,
            "recruiting" => Grow::Recruiting,
            "clinics" => Grow::Clinics,
            _ => return None,
        })
    }
}

/// One coach on the tree, as the board draws him.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Node {
    pub coach: Option<u32>,
    pub name: String,
    /// "HC Westfield, 31-18", "Your OC, 4 seasons".
    pub line: String,
    pub generation: u8,
    /// The gen-1 coach a gen-2 coach learned under.
    pub parent: Option<u32>,
    pub head: bool,
    pub rival: bool,
    /// On your staff now.
    pub yours: bool,
}

/// The whole tree and its tiles.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Tree {
    pub mentors: Vec<Node>,
    pub you: Node,
    pub gen1: Vec<Node>,
    pub gen2: Vec<Node>,
    pub head_coaches: u32,
    pub coordinators: u32,
    pub wins: u32,
    pub titles: u32,
    /// The NFL's compensatory picks for minority coaches hired away from
    /// your staff (Resolution JC-2A).
    pub comp_picks: u32,
    pub rival: Option<String>,
}

/// Your head coaching stints: (employer, from, to).
fn your_stints(w: &World, me: usize) -> Vec<(Employer, i32, i32)> {
    let c = &w.coaches[me];
    let mut v: Vec<(Employer, i32, i32)> =
        c.history.iter().filter(|s| s.role == Role::HeadCoach).map(|s| (s.employer, s.from, s.to)).collect();
    if let Some(j) = c.job
        && j.role == Role::HeadCoach
    {
        v.push((j.employer, j.since, w.year));
    }
    v
}

/// Whether a coach worked as an assistant on your staff.
fn worked_under(w: &World, c: &Coach, mine: &[(Employer, i32, i32)]) -> bool {
    let job = c.job.map(|j| (j.employer, j.role, j.since, w.year));
    c.history
        .iter()
        .map(|s| (s.employer, s.role, s.from, s.to))
        .chain(job)
        .any(|(e, r, from, to)| r != Role::HeadCoach && mine.iter().any(|&(me, a, b)| me == e && from <= b && to >= a))
}

/// A coach's head coaching wins and titles across his career.
fn head_record(c: &Coach) -> (u32, u32, u32) {
    let mut r = (0, 0, 0);
    for s in c.history.iter().filter(|s| s.role == Role::HeadCoach) {
        r.0 += s.wins;
        r.1 += s.losses;
        r.2 += s.titles;
    }
    if c.job.is_some_and(|j| j.role == Role::HeadCoach) {
        r.0 += c.wins;
        r.1 += c.losses;
        r.2 += c.titles;
    }
    r
}

fn short_role(r: Role) -> String {
    match r {
        Role::HeadCoach => "HC".into(),
        Role::OffensiveCoordinator => "OC".into(),
        Role::DefensiveCoordinator => "DC".into(),
        Role::PositionCoach(g) => format!("{} coach", g.name()),
    }
}

/// Whether two employers play each other in the same conference, division
/// or district.
fn same_league(w: &World, a: Employer, b: Employer) -> bool {
    match (a, b) {
        (Employer::College(p), Employer::College(q)) => {
            w.programs[p].conference == w.programs[q].conference && w.programs[p].division == w.programs[q].division
        }
        (Employer::Nfl(s), Employer::Nfl(t)) => {
            let div = |n: usize| crate::world::franchise::era_in(w, n, w.year).map(|e| e.div);
            div(s).is_some() && div(s) == div(t)
        }
        (Employer::HighSchool(s), Employer::HighSchool(t)) => w.hs_season.as_ref().is_some_and(|x| {
            let d = |h: usize| x.district_of(h);
            d(s).is_some() && d(s) == d(t)
        }),
        _ => false,
    }
}

fn node(w: &World, c: usize, generation: u8, parent: Option<u32>, mine: Option<Employer>) -> Node {
    let co = &w.coaches[c];
    let (wins, losses, _) = head_record(co);
    let (line, head, rival) = match co.job {
        Some(j) => {
            let place = crate::world::front_office::market::employer_name(w, j.employer);
            let head = j.role == Role::HeadCoach;
            let rival = head && mine.is_some_and(|m| m != j.employer && same_league(w, m, j.employer));
            let line = if head {
                format!("HC {place}, {}-{}", co.wins, co.losses)
            } else if mine == Some(j.employer) {
                format!("Your {}, {}", short_role(j.role), super::seasons_words(w.year - j.since + 1))
            } else {
                format!("{}, {place}", short_role(j.role))
            };
            (line, head, rival)
        }
        None if co.retired => (format!("Retired, {wins}-{losses} as head coach"), false, false),
        None => ("Between jobs".into(), false, false),
    };
    let yours = mine.is_some_and(|m| co.job.is_some_and(|j| j.employer == m));
    Node { coach: Some(c as u32), name: co.name.clone(), line, generation, parent, head, rival, yours }
}

/// Your coaching tree now.
pub fn tree(w: &World) -> Option<Tree> {
    let (me, _) = super::you(w)?;
    let mine = your_stints(w, me);
    let job = w.coaches[me].job.map(|j| j.employer);
    let you_c = &w.coaches[me];
    // Who taught you: your mentor and the first head coach who knew you.
    let mut mentors = Vec::new();
    if let Some(m) = you_c.mentor {
        let co = &w.coaches[m];
        let (wins, losses, titles) = head_record(co);
        let place = co.job.or(co.history.last().map(|s| crate::world::coach::Job {
            employer: s.employer,
            role: s.role,
            since: s.from,
            first: 0,
        }));
        let at = place.map_or(String::new(), |j| crate::world::front_office::market::employer_name(w, j.employer));
        let line = if titles > 0 {
            format!("{at}, {}", crate::world::news::count(titles, "title"))
        } else {
            format!("{at}, {wins}-{losses}")
        };
        mentors.push(Node {
            coach: Some(m as u32),
            name: co.name.clone(),
            line,
            generation: 0,
            parent: None,
            head: true,
            rival: false,
            yours: false,
        });
    }
    if let Some(c) = w.career.as_ref()
        && let Some(k) = c
            .known
            .iter()
            .filter(|k| Some(k.coach) != you_c.mentor && k.coach < w.coaches.len())
            .min_by_key(|k| (k.from, k.coach))
    {
        let co = &w.coaches[k.coach];
        mentors.push(Node {
            coach: Some(k.coach as u32),
            name: co.name.clone(),
            line: format!("{}, {}", k.how, k.from),
            generation: 0,
            parent: None,
            head: false,
            rival: false,
            yours: false,
        });
    }
    let (yw, yl, _) = head_record(you_c);
    let here = job.map_or(String::new(), |e| crate::world::front_office::market::employer_name(w, e));
    let you = Node {
        coach: Some(me as u32),
        name: "You".into(),
        line: format!("{here}, {yw}-{yl}"),
        generation: 0,
        parent: None,
        head: true,
        rival: false,
        yours: true,
    };
    // Generation one: learned under you or worked on your staff.
    let mut gen1: Vec<Node> = Vec::new();
    for (c, co) in w.coaches.iter().enumerate() {
        if c == me {
            continue;
        }
        if co.mentor == Some(me) || (!mine.is_empty() && worked_under(w, co, &mine)) {
            gen1.push(node(w, c, 1, None, job));
        }
    }
    // Your high school staff now: they are on the tree from their first day.
    if let Some(k) = w.career.as_ref() {
        for a in &k.hs_staff {
            let role = match a.coordinator {
                Some(true) => "OC".to_string(),
                Some(false) => "DC".to_string(),
                None => format!("{} coach", Group::ALL[a.group as usize % Group::ALL.len()].name()),
            };
            gen1.push(Node {
                coach: None,
                name: a.name.clone(),
                line: format!("Your {role}, {}", super::seasons_words(w.year - a.since + 1)),
                generation: 1,
                parent: None,
                head: false,
                rival: false,
                yours: true,
            });
        }
    }
    let ids: Vec<usize> = gen1.iter().filter_map(|n| n.coach.map(|c| c as usize)).collect();
    let mut gen2: Vec<Node> = Vec::new();
    for (c, co) in w.coaches.iter().enumerate() {
        if c == me || ids.contains(&c) {
            continue;
        }
        if let Some(m) = co.mentor
            && ids.contains(&m)
        {
            gen2.push(node(w, c, 2, Some(m as u32), job));
        }
    }
    // Head coaches first, then the most wins.
    let order = |v: &mut Vec<Node>| {
        v.sort_by(|a, b| {
            let wins = |n: &Node| n.coach.map_or(0, |c| head_record(&w.coaches[c as usize]).0);
            b.head.cmp(&a.head).then(b.yours.cmp(&a.yours)).then(wins(b).cmp(&wins(a))).then(a.name.cmp(&b.name))
        })
    };
    order(&mut gen1);
    order(&mut gen2);
    let all: Vec<&Node> = gen1.iter().chain(gen2.iter()).collect();
    let head_coaches = all.iter().filter(|n| n.head).count() as u32;
    let coordinators = all
        .iter()
        .filter(|n| {
            n.coach
                .and_then(|c| w.coaches[c as usize].job)
                .is_some_and(|j| matches!(j.role, Role::OffensiveCoordinator | Role::DefensiveCoordinator))
        })
        .count() as u32;
    let (mut wins, mut titles) = (0, 0);
    for n in &all {
        if let Some(c) = n.coach {
            let r = head_record(&w.coaches[c as usize]);
            wins += r.0;
            titles += r.2;
        }
    }
    let comp_picks = match job {
        Some(Employer::Nfl(t)) => {
            let since = mine.iter().filter(|s| s.0 == Employer::Nfl(t)).map(|s| s.1).min().unwrap_or(w.year);
            w.market.picks.diversity.iter().filter(|d| d.1 == t && d.0 >= since).count() as u32 * 2
        }
        _ => 0,
    };
    let rival = gen1.iter().find(|n| n.rival).map(|n| n.name.split_whitespace().last().unwrap_or("").to_string());
    Some(Tree { mentors, you, gen1, gen2, head_coaches, coordinators, wins, titles, comp_picks, rival })
}

/// How you grow a man on your staff now (None takes the choice back).
pub fn set_grow(w: &mut World, coach: usize, grow: Option<Grow>) {
    w.legacy.grow.retain(|x| x.0 != coach);
    if let Some(g) = grow {
        w.legacy.grow.push((coach, g));
        // Letting your coordinator call his side is your play calling
        // setting: it's set here too.
        if g == Grow::Calls
            && let Some((me, _)) = super::you(w)
            && let Some(j) = w.coaches[coach].job
            && let Some(y) = w.coaches[me].yours.as_mut()
        {
            match j.role {
                Role::OffensiveCoordinator => y.calls_offense = false,
                Role::DefensiveCoordinator => y.calls_defense = false,
                _ => {}
            }
        }
    }
}

/// After your season: your staff grows toward you, each by the way you
/// chose; your branches' new head jobs and titles pay.
pub(crate) fn season(w: &mut World) {
    let Some((me, _)) = super::you(w) else { return };
    let job = w.coaches[me].job.map(|j| j.employer);
    let mine = *w.coaches[me].ability();
    let teach = mine.teaching.iter().sum::<f64>() / mine.teaching.len() as f64;
    // Your assistants on world records (college and NFL staffs).
    let staff: Vec<usize> = (0..w.coaches.len())
        .filter(|&c| c != me && job.is_some() && w.coaches[c].job.map(|j| j.employer) == job)
        .collect();
    for &c in &staff {
        let grow = w.legacy.grow.iter().find(|x| x.0 == c).map(|x| x.1);
        let co = &mut w.coaches[c];
        // Toward your strengths, faster with a better teacher (you) and a
        // younger learner, never past his ceiling.
        let pace = 0.05 * (teach / 20.0) * if co.age < 40 { 1.2 } else { 0.8 };
        let cap = co.learning_ceiling;
        let pull = |x: &mut f64, to: f64, extra: f64| {
            let gain = pace * (to - *x).max(0.0) + extra;
            *x = (*x + gain).min(cap.max(*x)).clamp(1.0, 20.0);
        };
        let calls = grow == Some(Grow::Calls);
        let clinics = grow == Some(Grow::Clinics);
        pull(
            &mut co.ability.scheme,
            mine.scheme,
            if calls {
                0.25
            } else if clinics {
                0.1
            } else {
                0.0
            },
        );
        pull(&mut co.ability.game_management, mine.game_management, if calls { 0.2 } else { 0.0 });
        pull(&mut co.ability.motivation, mine.motivation, 0.0);
        pull(&mut co.ability.network, mine.network, if clinics { 0.3 } else { 0.0 });
        pull(&mut co.ability.recruiting, mine.recruiting, if grow == Some(Grow::Recruiting) { 0.3 } else { 0.0 });
        if let Some(j) = co.job
            && let Role::PositionCoach(g) = j.role
        {
            let gi = g as usize;
            let extra = if clinics { 0.15 } else { 0.0 };
            pull(&mut co.ability.teaching[gi], mine.teaching[gi], extra);
        }
        // The résumé the world reads: calling plays and an area of your
        // own put a name on lists.
        if calls {
            co.stat_buzz += 0.5;
        }
        if grow == Some(Grow::Recruiting) {
            co.reputation = (co.reputation + 0.2).clamp(1.0, 20.0);
        }
    }
    // The branches: a first head job and every title pay you once.
    let Some(t) = tree(w) else { return };
    for n in t.gen1.iter().filter(|n| n.coach.is_some()) {
        let c = n.coach.unwrap_or(0) as usize;
        if n.head {
            let job = w.coaches[c].job.map_or(0, |j| j.since as u64);
            if pay(
                w,
                key("G2", c as u64, job),
                "G2",
                format!("{} got a head job ({})", n.name, n.line),
                0.5,
                Bank::Rating(Rating::Network),
                0.5,
            ) {
                memo(
                    w,
                    "legacy-tree",
                    format!("{} gets a head job", n.name),
                    vec![format!("{} is a head coach now: {}. Another branch on your tree.", n.name, n.line)],
                );
            }
        }
        let (_, _, titles) = head_record(&w.coaches[c]);
        for k in 1..=titles.min(6) {
            pay(
                w,
                key("G2", c as u64, 0x7171 + k as u64),
                "G2",
                format!("{} won a title as a head coach", n.name),
                0.25,
                Bank::Rating(Rating::Network),
                0.5,
            );
        }
    }
}

/// A high school's search: whether it hires one of your assistants. A
/// keyed draw for each, on his résumé: years with you, a coordinator's
/// job, your record (proposed, not confirmed).
pub(crate) fn assistant_pick(w: &World, school: usize) -> Option<usize> {
    let k = w.career.as_ref()?;
    let (me, team) = super::you(w)?;
    let crate::world::stories::TeamKey::Hs(h) = team else { return None };
    if w.high_schools[h as usize].state != w.high_schools[school].state || school == h as usize {
        return None;
    }
    let (yw, yl, _) = head_record(&w.coaches[me]);
    let pct = yw as f64 / (yw + yl).max(1) as f64;
    for (i, a) in k.hs_staff.iter().enumerate() {
        let years = w.year - a.since;
        if years < 3 || a.age > 58 {
            continue;
        }
        let coord = if a.coordinator.is_some() { 0.06 } else { 0.0 };
        let chance = (0.02 + coord + 0.08 * (pct - 0.5).max(0.0) + 0.005 * (a.teaching - 10.0)).clamp(0.0, 0.15);
        let u = crate::world::season::key_uniform(
            w.options.seed as u64 ^ 0xA551_5747,
            (school as u64) << 16 ^ i as u64 ^ (w.year as u64) << 40,
        );
        if u < chance {
            return Some(i);
        }
    }
    None
}

/// Your assistant becomes a head coach: a coach of the world's from now
/// on, with you as his mentor.
pub(crate) fn promote_assistant(w: &mut World, i: usize) -> Option<usize> {
    let a = w.career.as_mut()?.hs_staff.get(i).cloned()?;
    let me = super::you(w)?.0;
    let c = crate::world::staff::new_hs_coach(w, a.age);
    let co = &mut w.coaches[c];
    co.name = a.name.clone();
    co.born = a.born;
    co.mentor = Some(me);
    let g = a.group as usize % co.ability.teaching.len();
    co.ability.teaching[g] = a.teaching;
    if let Some(k) = w.career.as_mut() {
        k.hs_staff.remove(i);
    }
    memo(
        w,
        "legacy-tree-hs",
        format!("{} takes a head job", a.name),
        vec![format!(
            "{} is leaving your staff: a school in the state hired him as its head coach. You lose a good man and gain a branch.",
            a.name
        )],
    );
    Some(c)
}
