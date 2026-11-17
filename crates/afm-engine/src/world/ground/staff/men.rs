//! Each man's ground (GR68): his areas and his standing there, his people,
//! his ties, his record and the kids credited to him, his style, his week
//! and his mood; and a candidate's ground in signs (hiring for ground).

use super::{beats, count, last};
use crate::calendar::Date;
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::ground::areas;
use crate::world::ground::people::{self, Holder, Who};
use crate::world::ground::standing::{self, Level};

/// A man's mood (GR68), 0-1, in words with what moves it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Mood {
    pub value: f32,
    pub words: &'static str,
    /// What weighs on him or lifts him, the biggest first.
    pub reasons: Vec<String>,
    pub lifts: Vec<String>,
}

/// Mood in words.
pub fn mood_words(v: f32) -> &'static str {
    match v {
        x if x >= 0.7 => "settled",
        x if x >= 0.5 => "content",
        x if x >= 0.35 => "restless",
        x if x >= 0.2 => "listening to other jobs",
        _ => "unhappy",
    }
}

/// The spring road past which a man feels it (days), proposed, not
/// confirmed: a staff's 140 days spread over ten men is 14 each.
pub const LONG_SPRING: u16 = 30;

/// A man's mood: overruled again and again, an area he didn't want, a long
/// spring away from home, passed over for coordinator, a promise you broke
/// weigh on him; finds that bloomed, a title and a raise lift him (staff.md,
/// "His mood"). Sizes proposed, not confirmed.
pub fn mood(w: &World, prog: usize, man: usize) -> Mood {
    let mut v = 0.6f32;
    let mut down: Vec<(f32, String)> = Vec::new();
    let mut up: Vec<(f32, String)> = Vec::new();
    let over = crate::world::front_office::chain::overruled_recent(w, man);
    if over > 2 {
        down.push((0.07 * (over - 2).min(4) as f32, format!("overruled {over} times in two years")));
    }
    let Some(coach) = w.coaches.get(man) else {
        return Mood { value: v, words: mood_words(v), reasons: Vec::new(), lifts: Vec::new() };
    };
    let ambition = coach.personality.ambition as f32;
    // An area he didn't want: all new ground while his home state is worked
    // by someone else.
    if let Some(mine) = beats::his_areas(w, prog, man)
        && !mine.is_empty()
        && let Some(home) = coach.home_state
    {
        let all_new = mine.iter().all(|&a| {
            let schools: Vec<usize> =
                (0..w.high_schools.len()).filter(|&h| w.high_schools[h].ground_area == a).take(400).collect();
            let (k, t, h) = beats::holder_levels(w, Holder::Coach(man as u32), &schools);
            k + t + h == 0 && areas::area(a).state != home
        });
        let others_home = w.ground.staff.beats.iter().any(|b| b.man != man as u32 && areas::area(b.area).state == home);
        if all_new && others_home {
            down.push((
                0.08,
                format!(
                    "an area he didn't want, with {} worked by someone else",
                    w.states.get(home).map_or("his home state", |s| crate::world::folk::facts::state_name(&s.code))
                ),
            ));
        }
    }
    let (spring, _) = beats::days_out(w, man);
    if spring > LONG_SPRING {
        down.push((0.08, format!("a long spring away from home ({spring} days on the road)")));
    }
    let since = w.today.add(-730);
    if ambition >= 12.0 && w.ground.staff.passed_over.iter().any(|p| p.0 == man as u32 && p.1 >= since) {
        down.push((0.12, "passed over for coordinator".into()));
    }
    let broke = w.ground.staff.promises.iter().any(|p| p.man == man as u32 && p.kept == Some(false))
        || w.folk
            .hiring
            .promises
            .iter()
            .any(|p| p.coach == man && p.kept == Some(false) && p.made >= w.today.add(-1095));
    if broke {
        down.push((0.15, "a promise you broke".into()));
    }
    // Finds that bloomed: kids credited to him rated 3 stars or less who
    // started for you or were drafted.
    let mut blooms = 0;
    for c in w.ground.staff.credits.iter().filter(|c| c.man == man as u32 && c.stars <= 3) {
        let Some(i) = crate::world::scouting::place(w, c.id) else { continue };
        let q = &w.people[i];
        let started = q.college == Some(prog) && q.last_role == Some(crate::player::PlayingTime::Starter);
        if started || q.pro.as_ref().is_some_and(|p| p.draft.is_some()) {
            blooms += 1;
            if blooms <= 2 {
                up.push((0.05, format!("his find {} {}", q.name, if started { "started" } else { "was drafted" })));
            }
        }
    }
    if blooms > 2 {
        up.push((0.05 * (blooms - 2).min(2) as f32, format!("{} more finds of his came good", blooms - 2)));
    }
    if crate::world::folk::hiring::title_of(w, man).is_some() {
        up.push((0.04, format!("his title, {}", crate::world::folk::hiring::title_words(2))));
    }
    if w.folk.staff.raised_since(man, w.today.add(-365)) {
        up.push((0.04, "a raise this year".into()));
    }
    for (x, _) in &down {
        v -= x;
    }
    for (x, _) in &up {
        v += x;
    }
    down.sort_by(|a, b| b.0.total_cmp(&a.0));
    up.sort_by(|a, b| b.0.total_cmp(&a.0));
    let v = v.clamp(0.0, 1.0);
    Mood {
        value: v,
        words: mood_words(v),
        reasons: down.into_iter().map(|x| x.1).collect(),
        lifts: up.into_iter().map(|x| x.1).collect(),
    }
}

/// His style on the road, read from his makeup (GR65, GR68): a salesman
/// oversells, a straight shooter doesn't, a closer gets the yes, a grinder
/// outworks. Cut points proposed, not confirmed.
pub fn style(w: &World, man: usize) -> (&'static str, String) {
    let Some(c) = w.coaches.get(man) else { return ("grinder", String::new()) };
    let ab = c.ability();
    let mk = people::makeup(w, Who::Coach(man as u32));
    if mk.straight <= 8.0 && ab.relationships >= 11.0 {
        ("salesman", "he sells hard, and his claims to families are his to keep or break".into())
    } else if mk.straight >= 14.0 {
        ("straight shooter", "families believe what he says, and he says what he sees".into())
    } else if ab.recruiting >= 14.0 && c.personality.ambition >= 12.0 {
        ("closer", "he gets the yes in the living room".into())
    } else {
        ("grinder", "he wins on miles and visits".into())
    }
}

/// One of a man's areas, as his page shows it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AreaGround {
    pub area: u16,
    pub code: String,
    pub name: String,
    pub nickname: String,
    /// His own level there (his people, weighted by talent).
    pub level: &'static str,
    pub known: usize,
    pub trusted: usize,
    pub home: usize,
    pub heads: usize,
    pub why: String,
    pub since: Date,
    pub yours: bool,
    /// Another man works it with him.
    pub with: Vec<String>,
}

/// One person on a man's ground.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Person {
    pub who: Who,
    pub name: String,
    pub role: String,
    pub level: &'static str,
}

/// A kid credited to him.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Credited {
    pub id: u32,
    pub name: String,
    pub stars: u8,
    pub year: i32,
    pub now: String,
}

/// A man's ground, for his page and the staff rail (GR68, GR76).
#[derive(Clone, Debug, serde::Serialize)]
pub struct ManGround {
    pub man: usize,
    pub name: String,
    pub role: String,
    pub age: u32,
    pub home: Option<String>,
    pub areas: Vec<AreaGround>,
    /// His people, the best first (up to 10), and how many at each level.
    pub people: Vec<Person>,
    pub levels: (usize, usize, usize),
    /// His ties in words (his own, which go with him).
    pub ties: Vec<String>,
    /// His calls on the offer chain: (wanted offered, passed, settled right,
    /// overruled).
    pub record: (usize, usize, usize, usize),
    pub credited: Vec<Credited>,
    pub style: &'static str,
    pub style_why: String,
    /// Road days this season (spring, fall) and this week.
    pub days: (u16, u16, u32),
    /// Riding along with: (the other man, he's the young one).
    pub ride: Option<(String, bool)>,
    pub mood: Mood,
    /// Another school wants him (the warning's line).
    pub courted: Option<String>,
}

/// A man's level over his areas in words.
fn level_word(p: f32) -> &'static str {
    Level::of(p).name()
}

/// A man's ground (GR68).
pub fn ground(w: &World, prog: usize, man: usize) -> Option<ManGround> {
    let c = w.coaches.get(man)?;
    let holder = Holder::Coach(man as u32);
    let talent = standing::talents(w);
    let by_area = areas::schools_by_area(w);
    let mut out_areas = Vec::new();
    for b in w.ground.staff.beats.iter().filter(|b| b.man == man as u32) {
        let schools = &by_area[b.area as usize];
        let (k, t, h) = beats::holder_levels(w, holder, schools);
        let ar = areas::area(b.area);
        let with = beats::on_area(w, prog, b.area).into_iter().filter(|&m| m != man).map(|m| last(w, m)).collect();
        out_areas.push(AreaGround {
            area: b.area,
            code: areas::code(b.area),
            name: ar.name.to_string(),
            nickname: ar.nickname.to_string(),
            level: level_word(beats::holder_points(w, holder, schools, &talent)),
            known: k,
            trusted: t,
            home: h,
            heads: schools.iter().filter(|&&s| people::head_coach_of(w, s).is_some()).count(),
            why: b.why.clone(),
            since: b.since,
            yours: b.yours,
            with,
        });
    }
    // His people: what he keeps himself, then his own ties' head coaches.
    let mut list: Vec<(f32, Who)> = w
        .ground
        .memories
        .iter()
        .filter(|m| m.holder == holder && m.points >= 25.0)
        .map(|m| (m.points, m.who))
        .collect();
    let mut ties: Vec<(usize, f64)> = c.ties.iter().map(|(&h, &t)| (h, t)).collect();
    ties.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    for &(h, _) in &ties {
        if let Some(who) = people::head_coach_of(w, h)
            && !list.iter().any(|x| x.1 == who)
        {
            let p = standing::points(w, who, holder);
            if p >= 25.0 {
                list.push((p, who));
            }
        }
    }
    list.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut levels = (0, 0, 0);
    for (p, _) in &list {
        match Level::of(*p) {
            Level::Home => levels.2 += 1,
            Level::Trusted => levels.1 += 1,
            Level::Known => levels.0 += 1,
            Level::Stranger => {}
        }
    }
    let people_out = list
        .iter()
        .take(10)
        .map(|&(p, who)| Person {
            who,
            name: people::name(w, who),
            role: people::role(w, who),
            level: Level::of(p).name(),
        })
        .collect();
    let tie_words: Vec<String> = ties
        .iter()
        .take(5)
        .filter_map(|&(h, t)| {
            let s = w.high_schools.get(h)?;
            let strength = if t >= 0.6 {
                "a strong tie"
            } else if t >= 0.3 {
                "a tie"
            } else {
                "a light tie"
            };
            Some(format!("{} ({}), {strength}", w.school_name(h), areas::area(s.ground_area).nickname))
        })
        .collect();
    let calls = crate::world::front_office::chain::record(w, man);
    let record = (
        calls.iter().filter(|x| x.wanted).count(),
        calls.iter().filter(|x| !x.wanted).count(),
        calls.iter().filter(|x| x.settled == Some(true)).count(),
        calls.iter().filter(|x| x.overruled).count(),
    );
    let credited = w
        .ground
        .staff
        .credits
        .iter()
        .rev()
        .filter(|x| x.man == man as u32)
        .take(8)
        .filter_map(|x| {
            let i = crate::world::scouting::place(w, x.id)?;
            let q = &w.people[i];
            let now = match (q.college, q.last_role) {
                (Some(p), Some(crate::player::PlayingTime::Starter)) if p == prog => "a starter for you".to_string(),
                (Some(p), _) if p == prog => "on your roster".to_string(),
                (Some(p), _) => format!("at {}", w.program_name(p)),
                _ => "signed".to_string(),
            };
            Some(Credited { id: x.id, name: q.name.clone(), stars: x.stars, year: x.date.year(), now })
        })
        .collect();
    let (style, style_why) = style(w, man);
    let (sp, fa) = beats::days_out(w, man);
    let week = w.ground.week.roads.iter().filter(|r| r.man == man as u32).map(|r| r.days.count_ones()).sum();
    let ride = w.ground.staff.rides.iter().find(|r| r.young == man as u32 || r.veteran == man as u32).map(|r| {
        if r.young == man as u32 { (last(w, r.veteran as usize), true) } else { (last(w, r.young as usize), false) }
    });
    let courted = w
        .ground
        .staff
        .courted
        .iter()
        .find(|x| x.man == man as u32 && x.state == 0)
        .map(|x| format!("{} wants him as {}", employer_name(w, x.employer), x.role.name()));
    Some(ManGround {
        man,
        name: c.name.clone(),
        role: crate::world::folk::facts::coach_role(w, man),
        age: c.age_on(w.today),
        home: c.home_state.and_then(|s| w.states.get(s)).map(|s| s.code.clone()),
        areas: out_areas,
        people: people_out,
        levels,
        ties: tie_words,
        record,
        credited,
        style,
        style_why,
        days: (sp, fa, week),
        ride,
        mood: mood(w, prog, man),
        courted,
    })
}

/// An employer's short name.
pub fn employer_name(w: &World, e: Employer) -> String {
    match e {
        Employer::College(p) => w.program_name(p).to_string(),
        Employer::HighSchool(h) => w.school_name(h).to_string(),
        Employer::Nfl(t) => format!("the {}", w.nfl[t].name),
        Employer::Other(f) => crate::world::pro::other::name(f, w.year).to_string(),
    }
}

/// A candidate's ground in signs (GR68, "Hiring for ground"), fogged by
/// your network (GR2): his old jobs (public), where his ties are (named
/// areas only where your staff knows someone, else just the state), what
/// the people you're Trusted with say about him, and for a high school
/// coach what his school's trust is worth and the hiring rule (C17). Never
/// his true numbers.
pub fn signs(w: &World, prog: usize, c: usize) -> Vec<String> {
    let Some(co) = w.coaches.get(c) else { return Vec::new() };
    let mut out = Vec::new();
    // His old jobs, public.
    let mut stops: Vec<String> = co
        .history
        .iter()
        .rev()
        .filter(|s| matches!(s.employer, Employer::College(_) | Employer::HighSchool(_)))
        .take(3)
        .map(|s| format!("{} {}-{} ({})", employer_name(w, s.employer), s.from, s.to, s.role.name()))
        .collect();
    if let Some(j) = co.job {
        stops.insert(0, format!("{} now ({})", employer_name(w, j.employer), j.role.name()));
    }
    if !stops.is_empty() {
        out.push(format!("Where he's coached: {}.", stops.join("; ")));
    }
    // Where his ties are.
    let mut ties: Vec<(usize, f64)> = co.ties.iter().filter(|x| *x.1 >= 0.2).map(|(&h, &t)| (h, t)).collect();
    ties.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    ties.truncate(8);
    let mut open: Vec<(u16, Vec<String>)> = Vec::new();
    let mut fogged: Vec<usize> = Vec::new();
    let mut said: Vec<String> = Vec::new();
    for &(h, t) in &ties {
        let Some(s) = w.high_schools.get(h) else { continue };
        let a = s.ground_area;
        let known = (0..w.high_schools.len())
            .filter(|&x| w.high_schools[x].ground_area == a)
            .filter_map(|x| people::head_coach_of(w, x))
            .any(|who| standing::level(w, prog, who) >= Level::Known);
        if known {
            match open.iter_mut().find(|x| x.0 == a) {
                Some(x) => x.1.push(s.name.clone()),
                None => open.push((a, vec![s.name.clone()])),
            }
        } else if !fogged.contains(&s.state) {
            fogged.push(s.state);
        }
        if said.len() < 2
            && t >= 0.5
            && let Some(who) = people::head_coach_of(w, h)
            && standing::level(w, prog, who) >= Level::Trusted
        {
            said.push(format!(
                "{} at {}, whom you're Trusted with: \"he's the man in {}.\"",
                people::name(w, who),
                w.school_name(h),
                areas::area(a).nickname
            ));
        }
    }
    for (a, schools) in open {
        out.push(format!("Ties around {}: {}.", areas::area(a).nickname, schools.join(", ")));
    }
    if !fogged.is_empty() {
        let names: Vec<&str> = fogged
            .iter()
            .filter_map(|&s| w.states.get(s))
            .map(|s| crate::world::folk::facts::state_name(&s.code))
            .collect();
        out.push(format!("Ties in {} you can't place: your staff knows nobody there yet.", names.join(" and ")));
    }
    out.extend(said);
    // A high school coach: his school comes with him, within the rules.
    if let Some(Employer::HighSchool(h)) = co.job.map(|j| j.employer) {
        match crate::world::folk::backchannel::hs_hire(w, c) {
            Err(e) => out.push(format!("The rules bar it now: {e}")),
            Ok(_) => out.push(format!(
                "He'd bring {}'s trust with him: its coaches and its families know him.",
                w.school_name(h)
            )),
        }
    }
    if out.is_empty() {
        out.push("Nothing your network can tell you about where he recruits.".into());
    }
    out
}

/// What a man takes with him if he goes (GR76, "what leaves with him on
/// the map"): each of his areas' level for your staff today and with him
/// gone (the program's own ties and your other men stay).
pub fn what_leaves(w: &World, prog: usize, man: usize) -> Vec<(u16, &'static str, &'static str)> {
    let talent = standing::talents(w);
    let by_area = areas::schools_by_area(w);
    let mut holders = standing::holders(w, prog);
    let rest: Vec<Holder> = holders.iter().copied().filter(|h| *h != Holder::Coach(man as u32)).collect();
    if !holders.contains(&Holder::Coach(man as u32)) {
        holders.push(Holder::Coach(man as u32));
    }
    let level_with = |hs: &[Holder], schools: &[usize]| {
        let (mut sum, mut weight) = (0.0f32, 0.0f32);
        for &s in schools {
            let t = talent.get(s).copied().unwrap_or(1.0);
            let p = people::head_coach_of(w, s)
                .map_or(0.0, |who| hs.iter().map(|&h| standing::points(w, who, h)).fold(0.0, f32::max));
            sum += t * p;
            weight += t;
        }
        Level::of(if weight > 0.0 { sum / weight } else { 0.0 }).name()
    };
    w.ground
        .staff
        .beats
        .iter()
        .filter(|b| b.man == man as u32)
        .map(|b| {
            let schools = &by_area[b.area as usize];
            (b.area, level_with(&holders, schools), level_with(&rest, schools))
        })
        .collect()
}

/// A coach's role words for lists ("defensive line coach").
pub fn role_words(r: Role) -> &'static str {
    r.name()
}

/// "4 head coaches".
pub fn heads_words(n: usize) -> String {
    count(n, "head coach", "head coaches")
}
