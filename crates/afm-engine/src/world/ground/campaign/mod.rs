//! The offseason campaign (lane G6: GR30, GR31, GR33-GR41, GR59;
//! `docs/design/ground.md`, part H; the design in `ground/campaign.md` and
//! `ground/goals.md`).
//!
//! - **Goals in February, a verdict in August** (GR30). The day after
//!   signing day your AD sets one or two goals he holds you to in his
//!   office (`talk`, a boss meeting: live talks v3 with a key, chips
//!   without one); his other motives stay wishes (GR35, `motives`). The
//!   Monday after, at the February meeting, your coordinator lays out the
//!   needs from the Needs map and a ground goal; you keep them or pick your
//!   own from the list of 63 (GR36, GR37, `goals`), and the hunt is an
//!   optional fourth (GR33). A goal that runs two or three classes holds
//!   its slot for each of them (GR38).
//! - **The race through spring:** each Monday letter has a line per goal,
//!   and a warning comes when one slips.
//! - **Changing goals:** drop one of your own at any Monday meeting, and
//!   it counts as a miss with whoever you told (GR40); pick a new one at
//!   any Monday meeting, and it pays by how likely it looked when you
//!   picked it (GR41). A bolder goal than your coordinator's pays more and
//!   costs your staff a little when missed; your AD reads it by your
//!   program's level (GR34).
//! - **Media days** (GR59): a goal you name there is public and judged by
//!   the press and the fans too.
//! - **The verdict** the week before your first game: every goal on the
//!   scale (GR39) with why, from the world; it moves your AD's trust, your
//!   staff's warmth and your name with the coaches on your ground, a little
//!   each and never a firing on its own. The class itself is judged when it
//!   signs; later goals when the class has played.
//! - **Your first offseason** (GR31): your coordinator at your door the
//!   morning after the January window closes (the Jan 1 start; a later
//!   start opens Ground on day one), three schools to visit yourself, and
//!   the tools as you first need them. He suggests and explains, never
//!   decides; you can send him off.
//! - **The ground report's data** (GR77, `report`): the verdict, your finds,
//!   your ground February to August by area, who outworked whom, your men,
//!   a setback, your AD's word and the paper.
//!
//! Only your college seat as head coach runs a campaign; no other program
//! reads it, so the world plays the same on every seed. Every size here is
//! the build's pick (proposed, not confirmed).

pub mod goals;
pub mod motives;
pub mod report;
pub mod talk;

use goals::{Aim, Read, Verdict};

use crate::calendar::{Date, Weekday, YearDates};
use crate::world::World;
use crate::world::folk::Team;
use crate::world::ground::{areas, standing, week};

/// A goal's slot (GR30, GR33).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Slot {
    /// Your coordinator's needs.
    Needs,
    /// Your AD's goal.
    Ad,
    /// Your own goal.
    Yours,
    /// The hunt (optional).
    Hunt,
}

impl Slot {
    pub fn key(self) -> &'static str {
        match self {
            Slot::Needs => "needs",
            Slot::Ad => "ad",
            Slot::Yours => "yours",
            Slot::Hunt => "hunt",
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Slot::Needs => "Your needs",
            Slot::Ad => "Your AD's",
            Slot::Yours => "Yours",
            Slot::Hunt => "The hunt",
        }
    }

    /// One of your own (GR40: these you can drop).
    pub fn own(self) -> bool {
        matches!(self, Slot::Yours | Slot::Hunt)
    }
}

/// One class's part of a goal.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Part {
    pub class: i32,
    /// Its pace at its August verdict.
    pub pace: Option<f32>,
    /// Its final share once the class signed (or played, for a later goal).
    pub result: Option<f32>,
    /// What it read in words then.
    pub words: String,
}

/// Who you told about a goal (GR40, GR59): your staff always.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Told {
    pub ad: bool,
    /// Named at media days: the press and the fans judge it too.
    pub press: bool,
}

/// A goal you hold.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Held {
    pub aim: Aim,
    pub slot: Slot,
    /// The spring it was set (the February meeting's year).
    pub year: i32,
    pub set_on: Date,
    /// How likely it looked when you picked it (GR41).
    pub likely: f32,
    /// How much bolder than your coordinator's suggestion it is (GR34):
    /// his goal's chance less yours, 0 when it isn't bolder.
    pub bold: f32,
    pub told: Told,
    /// Who raised it, and why, in words.
    pub by: String,
    pub why: String,
    /// The day you dropped it (GR40).
    pub dropped: Option<Date>,
    pub parts: Vec<Part>,
    /// Every part has its result.
    pub settled: bool,
    /// Its reading at the last Monday (the slip warning).
    pub last: Option<f32>,
    pub last_words: String,
}

impl Held {
    /// It still holds its slot on a day (GR38): not dropped, and a class
    /// of it still to sign (a later goal frees its slot once its class signs).
    pub fn holds(&self, signing: impl Fn(i32) -> Date, d: Date) -> bool {
        self.dropped.is_none() && !self.settled && self.aim.classes().any(|c| d <= signing(c))
    }

    /// Open: not dropped and not settled.
    pub fn open(&self) -> bool {
        self.dropped.is_none() && !self.settled
    }
}

/// The first offseason's guide (GR31).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Guide {
    /// Your coordinator is guiding you.
    pub on: bool,
    /// You sent him off.
    pub off: bool,
    /// The morning he came to your door.
    pub door: Option<Date>,
    /// The three schools he picked for your first visits (one where they
    /// know you, one where they know the program, one he's heard about),
    /// and the area.
    pub schools: Vec<u32>,
    pub area: Option<u16>,
    /// The tools open so far: "map", "leads", "rivals", "week".
    pub tools: Vec<String>,
    /// You went on his road day.
    pub went: bool,
}

/// The campaign store, on `Ground.campaign`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Campaign {
    /// The spring of the last February meeting (0 before the first).
    pub year: i32,
    /// The class it recruits (it signs the next December and February).
    pub class: i32,
    /// Every goal held, running and past (the last six springs).
    pub goals: Vec<Held>,
    /// Your AD's other motives this spring: wishes that count only when met.
    pub wishes: Vec<motives::Motive>,
    /// His goals in the talk now (before it ends).
    pub draft: Vec<motives::Motive>,
    /// The day of his talk this spring.
    pub talked: Option<Date>,
    /// The guide (GR31).
    pub guide: Guide,
    /// Flips into your class (class, kid, from program) and commitments
    /// lost (class, kid, to program or none), tracked every day.
    pub flipped_in: Vec<(i32, u32, u32)>,
    pub lost: Vec<(i32, u32, Option<u32>)>,
    /// The kids you offered and where each was committed yesterday.
    pub(crate) commits: Vec<(u32, u32)>,
    /// Your area levels at the February meeting (area, points).
    pub feb: Vec<(u16, f32)>,
    /// The verdicts' reports, newest last (the last six).
    pub reports: Vec<report::Report>,
}

/// Your college program as its head coach: the campaign's seat.
pub fn seat(w: &World) -> Option<usize> {
    let prog = week::seat_program(w)?;
    let you = crate::world::scouting::readers::you(w)?;
    (w.programs[prog].head_coach == Some(you)).then_some(prog)
}

/// A spring's dates: signing day (in that February) and the season's.
pub struct Spring {
    pub year: i32,
    pub signing: Date,
    pub meeting: Date,
    pub talk: Date,
    pub door: Date,
    pub media: (Date, Date),
    pub fall_camp: Date,
}

impl Spring {
    pub fn of(year: i32) -> Spring {
        let last = YearDates::new(year - 1);
        let this = YearDates::new(year);
        let signing = last.signing_day;
        let meeting = signing.add(1).on_or_after(Weekday::Mon);
        let (_, close) = crate::calendar::anchors::january_window(year - 1);
        Spring {
            year,
            signing,
            meeting,
            talk: signing.add(1),
            door: close.add(1),
            media: this.media_days,
            fall_camp: this.fall_camp,
        }
    }
}

/// The class being recruited on a day: this spring's until signing day,
/// the next one after it.
pub fn class_now(w: &World) -> i32 {
    let y = w.today.year();
    if w.today > Spring::of(y).signing { y + 1 } else { y }
}

/// The signing day of a class (its February).
pub fn signing_of(class: i32) -> Date {
    YearDates::new(class - 1).signing_day
}

/// The verdict day of a spring (GR30): the Monday of the week before your
/// first game, else of the week before the season's Week 1.
pub fn verdict_day(w: &World, prog: usize, year: i32) -> Date {
    let first = crate::world::folk::facts::team_games(w, Team::College(prog))
        .into_iter()
        .map(|g| g.0)
        .filter(|d| d.year() == year && d.month() >= 8)
        .min();
    let first = first.unwrap_or_else(|| {
        let y = YearDates::new(year);
        y.cfb_weeks.get(1).copied().unwrap_or(y.fall_camp.add(28))
    });
    first.add(-7).on_or_before(Weekday::Mon)
}

/// How far the spring has gone, February meeting 0 to the verdict 1.
pub fn elapsed(w: &World, prog: usize) -> f32 {
    let y = w.today.year();
    let s = Spring::of(y);
    let v = verdict_day(w, prog, y);
    if w.today <= s.meeting {
        return 0.0;
    }
    (s.meeting.days_to(w.today) as f32 / s.meeting.days_to(v).max(1) as f32).clamp(0.0, 1.0)
}

/// Your first season, a Jan 1 start: Ground waits until the January portal
/// window has closed (GR31), so the takeover and the portal come first.
pub fn waits(w: &World, d: Date) -> bool {
    if w.ground.campaign.year != 0 || !w.folk.takeover.as_ref().is_some_and(|t| t.rush) {
        return false;
    }
    let Some(since) = w.folk.mine.since else { return false };
    let s = Spring::of(d.year());
    since.days_to(d) < 120 && d < s.door
}

/// The memo from your coordinator (his name and role on it).
fn memo(w: &mut World, prog: usize, d: Date, minute: u16, subject: String, body: Vec<String>) {
    let from =
        week::coordinator(w, prog).map_or_else(|| "Recruiting office".to_string(), |c| w.coaches[c].name.clone());
    week::plan::memo(w, d, minute, &from, subject, body);
}

/// Your seat's campaign each day: flips tracked, the door, the AD's talk,
/// the February meeting, Monday's lines and warnings, media days, the
/// verdict, the classes as they sign, later goals as they're judged.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(prog) = seat(w) else { return };
    track(w, prog, d);
    let s = Spring::of(d.year());
    first_offseason(w, prog, d, &s);
    if d == s.signing && w.ground.campaign.talked.is_none_or(|t| t.year() != s.year) {
        talk::schedule(w, prog, s.talk);
    }
    let since = w.folk.mine.since.unwrap_or(d);
    let late_start = since > s.meeting && since.month() <= 3 && w.ground.campaign.year != s.year;
    if (d == s.meeting || (late_start && d >= since)) && w.ground.campaign.year != s.year {
        meeting(w, prog, d, &s);
    }
    if d.weekday() == Weekday::Mon && w.ground.campaign.year == s.year && d > s.meeting {
        monday(w, prog, d);
    }
    if w.ground.campaign.year == s.year && d == verdict_day(w, prog, s.year) {
        verdict(w, prog, d);
    }
    settle(w, prog, d);
    tools(w, prog);
}

/// Every day: where the kids you offered are committed, so a flip into
/// your class and a commitment lost show (goals 8, 39, 40, 42).
fn track(w: &mut World, prog: usize, d: Date) {
    let y = d.year();
    let mut now: Vec<(u32, u32)> = Vec::new();
    for p in w.people.iter().filter(|p| p.class_of > y && p.class_of <= y + 3) {
        let offered = p.cal.offers.contains(&prog) || p.cal.early_offers.iter().any(|x| x.0 == prog);
        let to = goals::went(p);
        if offered || to == Some(prog) {
            now.push((p.player.id, to.map_or(u32::MAX, |j| j as u32)));
        }
    }
    now.sort_unstable();
    let old = std::mem::take(&mut w.ground.campaign.commits);
    let class_of = |w: &World, id: u32| crate::world::folk::person(w, id).map(|i| w.people[i].class_of);
    for &(id, to) in &now {
        let before = old.binary_search_by_key(&id, |x| x.0).ok().map(|k| old[k].1);
        let Some(class) = class_of(w, id) else { continue };
        match before {
            Some(b) if b != to && to == prog as u32 && b != u32::MAX => {
                w.ground.campaign.flipped_in.push((class, id, b));
            }
            Some(b) if b == prog as u32 && to != prog as u32 => {
                w.ground.campaign.lost.push((class, id, (to != u32::MAX).then_some(to)));
            }
            _ => {}
        }
    }
    // A commit of yours gone from the list (he left the world) isn't lost.
    w.ground.campaign.commits = now;
    let cut = y - 1;
    w.ground.campaign.flipped_in.retain(|x| x.0 >= cut);
    w.ground.campaign.lost.retain(|x| x.0 >= cut);
}

// --- The first offseason (GR31) -----------------------------------------------------------------

/// Your first offseason: the door after the January window closes (a Jan 1
/// start), or day one for a later start, with his three schools.
fn first_offseason(w: &mut World, prog: usize, d: Date, s: &Spring) {
    let c = &w.ground.campaign;
    if c.year != 0 || c.guide.door.is_some() || c.guide.off {
        return;
    }
    let Some(since) = w.folk.mine.since else { return };
    if since.days_to(d) > 200 || d.month() > 8 {
        return;
    }
    let door = if since < s.door { s.door } else { since };
    // A day's grace: the clock may first run his day after the start.
    if d < door || d > door.add(7) {
        return;
    }
    let g = &mut w.ground.campaign.guide;
    g.on = true;
    g.door = Some(d);
    g.tools.push("map".into());
    let (area, schools) = guide_schools(w, prog);
    w.ground.campaign.guide.area = area;
    w.ground.campaign.guide.schools = schools.iter().map(|&h| h as u32).collect();
    let mut body = vec![if d > s.meeting {
        "Let me show you our ground before anything else.".to_string()
    } else if since < s.door {
        "Portal's done. Let me show you our ground.".to_string()
    } else {
        "Let me show you our ground.".to_string()
    }];
    body.push(ground_words(w, prog));
    if !schools.is_empty() && d < s.signing {
        let names: Vec<String> = schools.iter().map(|&h| school_why(w, prog, h)).collect();
        body.push(format!(
            "Your job for January: meet your ground before signing day. It's a contact period, so you can go yourself. Three schools: {}.",
            names.join("; ")
        ));
        body.push("Say the word and I'll put you on the road. The coach's list is yours when you walk in.".into());
    } else if !schools.is_empty() {
        let names: Vec<String> = schools.iter().map(|&h| school_why(w, prog, h)).collect();
        body.push(format!("Where I'd start with the staff on the road: {}.", names.join("; ")));
    }
    body.push("The February meeting is where we set the spring's goals. I'll walk you through it, and you can send me off any time.".into());
    memo(w, prog, d, 5 * 60, "Our ground".into(), body);
}

/// The guide's three schools (GR31): one where they know you, one where
/// they know the program, one he's heard about (a three-star or better on
/// your board where nobody knows you), in the area where your standing is
/// best when it has all three.
fn guide_schools(w: &World, prog: usize) -> (Option<u16>, Vec<usize>) {
    let you = standing::head_holder(w, prog);
    let board = crate::world::scouting::dept::watch(w, prog);
    let st = w.programs[prog].state;
    let mut best: (i32, Option<u16>, Vec<usize>) = (-1, None, Vec::new());
    let mut by_area: std::collections::BTreeMap<u16, Vec<usize>> = Default::default();
    for (h, s) in w.high_schools.iter().enumerate() {
        if s.state == st && s.coach.is_some() {
            by_area.entry(s.ground_area).or_default().push(h);
        }
    }
    let board_schools: Vec<usize> = board.iter().map(|&i| w.people[i].hs).collect();
    for (&a, schools) in &by_area {
        let coach = |h: usize| crate::world::ground::people::head_coach_of(w, h);
        let pick_you = you.and_then(|hold| {
            schools
                .iter()
                .copied()
                .filter(|&h| coach(h).is_some_and(|c| standing::points(w, c, hold) >= 25.0))
                .max_by(|&x, &y| w.high_schools[x].prestige.total_cmp(&w.high_schools[y].prestige))
        });
        let pick_prog = schools
            .iter()
            .copied()
            .filter(|&h| Some(h) != pick_you)
            .filter(|&h| {
                coach(h)
                    .is_some_and(|c| standing::points(w, c, crate::world::ground::Holder::Program(prog as u32)) >= 25.0)
            })
            .max_by(|&x, &y| w.high_schools[x].prestige.total_cmp(&w.high_schools[y].prestige));
        let pick_heard = schools
            .iter()
            .copied()
            .filter(|&h| Some(h) != pick_you && Some(h) != pick_prog)
            .filter(|&h| board_schools.contains(&h))
            .filter(|&h| coach(h).is_some_and(|c| standing::level(w, prog, c) == standing::Level::Stranger))
            .max_by(|&x, &y| w.high_schools[x].prestige.total_cmp(&w.high_schools[y].prestige))
            .or_else(|| {
                schools
                    .iter()
                    .copied()
                    .filter(|&h| Some(h) != pick_you && Some(h) != pick_prog)
                    .max_by(|&x, &y| w.high_schools[x].prestige.total_cmp(&w.high_schools[y].prestige))
            });
        let v: Vec<usize> = [pick_you, pick_prog, pick_heard].into_iter().flatten().collect();
        let score = 2 * pick_you.is_some() as i32 + 2 * pick_prog.is_some() as i32 + v.len() as i32;
        if score > best.0 {
            best = (score, Some(a), v);
        }
    }
    (best.1, best.2)
}

fn school_why(w: &World, prog: usize, h: usize) -> String {
    let name = w.school_name(h).to_string();
    let Some(c) = crate::world::ground::people::head_coach_of(w, h) else { return name };
    let coach = crate::world::ground::people::name(w, c);
    let you = standing::head_holder(w, prog).map_or(0.0, |hold| standing::points(w, c, hold));
    let them = standing::points(w, c, crate::world::ground::Holder::Program(prog as u32));
    if you >= 25.0 {
        format!("{name}, where {coach} knows you")
    } else if them >= 25.0 {
        format!("{name}, where {coach} knows the program")
    } else {
        format!("{name}, {coach}'s school: I've heard about a kid there")
    }
}

/// Your ground in a line: where you stand best and who leads your state.
fn ground_words(w: &World, prog: usize) -> String {
    let talent = standing::talents(w);
    let st = w.programs[prog].state;
    let mut rows: Vec<(f32, &'static str)> = areas::table()
        .iter()
        .enumerate()
        .filter(|(_, a)| a.state == st)
        .map(|(k, a)| (standing::area_level(w, prog, k as u16, &talent), a.name))
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    match rows.first() {
        Some(&(p, name)) if p >= 25.0 => format!(
            "We're {} in {name}; everywhere else in the state we're starting from less.",
            standing::Level::of(p).name()
        ),
        Some(_) => "Nobody on the ground knows us well yet. That's what this spring is for.".into(),
        None => String::new(),
    }
}

/// You go on your coordinator's road day (GR31): you to his area on the
/// next open day this week, his three schools first.
pub fn guide_go(w: &mut World) -> Result<String, String> {
    let prog = seat(w).ok_or("The campaign is for a college head coach.")?;
    let g = w.ground.campaign.guide.clone();
    let area = g.area.ok_or("Your coordinator hasn't picked your schools.")?;
    let you = crate::world::scouting::readers::you(w).ok_or("No career.")? as u32;
    week::plan::ensure_now(w, prog);
    let monday = week::monday_of(w.today);
    let mut last = Err("No open road day this week.".to_string());
    let today = w.today;
    for k in (0..7u32).filter(|&k| monday.add(k as i32) >= today) {
        last = week::plan::set_road(w, you, area, &[k]);
        if last.is_ok() {
            break;
        }
    }
    let text = last?;
    if let Some(r) = w.ground.week.roads.iter_mut().find(|r| r.man == you) {
        r.focus = g.schools.clone();
        r.why = "Your coordinator's three schools".into();
    }
    w.ground.campaign.guide.went = true;
    Ok(text)
}

/// You send your coordinator off (GR31): the guide stops; every tool opens.
pub fn guide_off(w: &mut World) -> Result<String, String> {
    seat(w).ok_or("The campaign is for a college head coach.")?;
    let g = &mut w.ground.campaign.guide;
    g.on = false;
    g.off = true;
    for t in TOOLS {
        if !g.tools.iter().any(|x| x == t) {
            g.tools.push(t.to_string());
        }
    }
    Ok("He nods and goes back to his office. The calls are yours.".into())
}

/// Ground's tools in the order they open.
pub const TOOLS: [&str; 4] = ["map", "leads", "rivals", "week"];

/// Tools open as you first need them, in your first offseason only: the
/// map on day one, the leads board with your first lead, the rivals page
/// when a rival first takes a kid of yours or flips one, the week planner
/// at the February meeting. After the first offseason everything is open.
pub fn tool_open(w: &World, tool: &str) -> bool {
    let c = &w.ground.campaign;
    !c.guide.on || c.guide.tools.iter().any(|t| t == tool)
}

fn tools(w: &mut World, _prog: usize) {
    if !w.ground.campaign.guide.on {
        return;
    }
    let lead = !w.ground.leads.is_empty();
    let rival = !w.ground.campaign.lost.is_empty() || !w.ground.campaign.flipped_in.is_empty();
    let g = &mut w.ground.campaign.guide;
    for (t, now) in [("leads", lead), ("rivals", rival)] {
        if now && !g.tools.iter().any(|x| x == t) {
            g.tools.push(t.into());
        }
    }
}

// --- The February meeting (GR30) ---------------------------------------------------------------

/// Your coordinator's needs goal from the Needs map (the position goals,
/// 28-37): the biggest hole in next year's class by your staff's plan.
pub fn needs_goal(w: &World, prog: usize, class: i32) -> (Aim, String) {
    use crate::positions::Position as P;
    let o = crate::world::front_office::college::outlook(w, prog);
    let need = |p: P| o.need.get(p as usize).copied().unwrap_or(0);
    let mut by: Vec<(usize, P)> = P::ALL.iter().map(|&p| (need(p), p)).collect();
    by.sort_by(|a, b| b.0.cmp(&a.0).then((a.1 as usize).cmp(&(b.1 as usize))));
    let (n, top) = by.first().copied().unwrap_or((0, P::OT));
    let a = |goal: u8, n: u32, of: u32| Aim { goal, n, of, class };
    let ol = need(P::OT) + need(P::IOL);
    let db = need(P::CB) + need(P::S);
    let (aim, why) = if top == P::QB {
        (a(29, 0, 0), "We need a quarterback in every class.".to_string())
    } else if matches!(top, P::OT | P::IOL) && ol >= 3 {
        (a(31, 0, 0), format!("{ol} linemen on the plan: a tackle, a guard and a center, over two classes."))
    } else if matches!(top, P::CB | P::S) && db >= 2 {
        (a(32, 0, 0), format!("{db} spots in the secondary on the plan."))
    } else if top == P::DT {
        (a(37, n.clamp(1, 3) as u32, 0), format!("{n} defensive tackles on the plan, and we need size."))
    } else {
        let board = crate::world::scouting::dept::watch(w, prog);
        let kid = board
            .iter()
            .copied()
            .filter(|&i| w.people[i].class_of == class && w.people[i].player.position == top)
            .max_by(|&x, &y| w.people[x].stars.cmp(&w.people[y].stars).then(y.cmp(&x)));
        match kid {
            Some(i) => (
                a(33, 0, w.people[i].player.id),
                format!("{} is our biggest need, and {} is the best one on the board.", top.name(), w.people[i].name),
            ),
            None => (a(32, 0, 0), "The secondary is where the plan is thinnest.".to_string()),
        }
    };
    (aim, why)
}

/// Your coordinator's ground goal (GR30): hold an area you lead that a
/// rival is closing on, take one you're close to leading, or open the best
/// area where nobody knows you, in your state.
pub fn ground_goal(w: &World, prog: usize, class: i32) -> (Aim, String) {
    let talent = standing::talents(w);
    let st = w.programs[prog].state;
    let mut best: Option<(f32, Aim, String)> = None;
    for (k, ar) in areas::table().iter().enumerate().filter(|(_, a)| a.state == st) {
        let id = k as u16;
        let schools: Vec<usize> = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].ground_area == id).collect();
        let tal: f32 = schools.iter().map(|&h| talent.get(h).copied().unwrap_or(1.0)).sum();
        let me = standing::area_points(w, prog, &schools, &talent);
        let lead = standing::leader(w, &schools, &talent);
        let (score, goal, why) = match lead {
            Some((l, _)) if l == prog => {
                let second = (0..w.programs.len())
                    .filter(|&j| j != prog)
                    .map(|j| standing::area_points(w, j, &schools, &talent))
                    .fold(0.0f32, f32::max);
                let close = (10.0 - (me - second)).max(0.0);
                (
                    tal * 0.05 * close,
                    60,
                    format!("We lead {}, and the next staff is {:.0} points back.", ar.name, me - second),
                )
            }
            Some((l, pts)) if pts - me <= 12.0 && me >= 25.0 => (
                tal * 0.08 * (12.0 - (pts - me)),
                59,
                format!("{} leads {} by a few calls. We can take it.", w.program_name(l), ar.name),
            ),
            _ if me < 25.0 => {
                (tal * 0.04 * (1.0 + me / 25.0), 61, format!("{} has the kids and nobody there knows us yet.", ar.name))
            }
            _ => continue,
        };
        if best.as_ref().is_none_or(|b| score > b.0) {
            best = Some((score, Aim { goal, n: 0, of: id as u32, class }, why));
        }
    }
    best.map(|b| (b.1, b.2)).unwrap_or((Aim { goal: 44, n: 0, of: 0, class }, "Find one the world missed.".into()))
}

/// The day's reading of a held goal for a class.
pub fn reading(w: &World, prog: usize, h: &Held, class: i32) -> Read {
    goals::read(w, prog, &h.aim, class)
}

/// Starts a goal with its chance and boldness.
fn hold(w: &World, prog: usize, aim: Aim, slot: Slot, by: String, why: String, bold_over: Option<f32>) -> Held {
    let t = elapsed(w, prog);
    let likely = goals::likely(w, prog, &aim, t);
    let bold = bold_over.map_or(0.0, |s| (s - likely).max(0.0));
    let bold = if bold >= 0.15 { bold.min(0.6) } else { 0.0 };
    let parts = aim.classes().map(|c| Part { class: c, pace: None, result: None, words: String::new() }).collect();
    Held {
        aim,
        slot,
        year: w.today.year(),
        set_on: w.today,
        likely,
        bold,
        told: Told { ad: slot == Slot::Ad, press: false },
        by,
        why,
        dropped: None,
        parts,
        settled: false,
        last: None,
        last_words: String::new(),
    }
}

/// The slot is taken by a running goal (GR38).
fn taken(w: &World, slot: Slot, d: Date) -> bool {
    w.ground.campaign.goals.iter().any(|h| h.slot == slot && h.holds(signing_of, d) && h.year < d.year())
}

/// The February meeting (GR30): the class you signed, next year's holes,
/// your AD's word, your ground; your coordinator's three goals, which you
/// change at will.
fn meeting(w: &mut World, prog: usize, d: Date, s: &Spring) {
    let class = s.year + 1;
    let c = &mut w.ground.campaign;
    c.year = s.year;
    c.class = class;
    c.goals.retain(|h| h.year >= s.year - 6);
    // The AD's goals from his talk; without one (a late start, a skipped
    // talk) his weightiest motives stand.
    if !w.ground.campaign.goals.iter().any(|h| h.slot == Slot::Ad && h.year == s.year) && !taken(w, Slot::Ad, d) {
        let ms = motives::motives(w, prog, class);
        let k = motives::held(w, prog);
        let (set, rest): (Vec<_>, Vec<_>) = ms.into_iter().enumerate().partition(|(i, _)| *i < k);
        let set: Vec<motives::Motive> = set.into_iter().map(|x| x.1).collect();
        w.ground.campaign.wishes = rest.into_iter().map(|x| x.1).take(5).collect();
        set_ad(w, prog, set);
    }
    let mut lines: Vec<String> = Vec::new();
    // The class you signed, next year's holes.
    let signed = goals::class_of(w, prog, class - 1).len();
    let rank = crate::world::folk::boss::class_rank(w, prog, class - 1);
    lines.push(match rank {
        Some(r) => format!("The class we signed: {signed}, {} in the conference.", goals_ordinal(r)),
        None => format!("The class we signed: {signed}."),
    });
    if !taken(w, Slot::Needs, d) {
        let (aim, why) = needs_goal(w, prog, class);
        let h = hold(w, prog, aim, Slot::Needs, "your coordinator".into(), why.clone(), None);
        lines.push(format!("Needs: {}. {why}", goals::words(w, &aim)));
        w.ground.campaign.goals.push(h);
    }
    for h in
        w.ground.campaign.goals.iter().filter(|h| h.slot == Slot::Ad && h.open() && h.aim.classes().any(|c| c == class))
    {
        lines.push(format!("Your AD's word: {}. {}", goals::words(w, &h.aim), h.why));
    }
    if !taken(w, Slot::Yours, d) {
        let (aim, why) = ground_goal(w, prog, class);
        let h = hold(w, prog, aim, Slot::Yours, "your coordinator".into(), why.clone(), None);
        lines.push(format!("A ground goal: {}. {why}", goals::words(w, &aim)));
        w.ground.campaign.goals.push(h);
    } else if let Some(h) = w.ground.campaign.goals.iter().find(|h| h.slot == Slot::Yours && h.open()) {
        lines.push(format!("Your goal still running: {}.", goals::words(w, &h.aim)));
    }
    let finds = w.ground.finds.iter().filter(|f| f.prog as usize == prog && f.class_of == class).count();
    lines.push(if finds > 0 {
        format!("The hunt is yours if you want it: we have {finds} finds in this class to chase.")
    } else {
        "The hunt is yours if you want it: find one the world missed.".to_string()
    });
    lines.push("Change any of them. Each Monday you can drop one of yours or pick a new one.".into());
    // Your ground as it stands, for the verdict's February to August.
    let talent = standing::talents(w);
    let feb: Vec<(u16, f32)> =
        report_areas(w, prog).into_iter().map(|a| (a, standing::area_level(w, prog, a, &talent))).collect();
    w.ground.campaign.feb = feb;
    if w.ground.campaign.guide.on && !w.ground.campaign.guide.tools.iter().any(|t| t == "week") {
        w.ground.campaign.guide.tools.push("week".into());
        lines.push(
            "From today the week is yours to plan: I draft it every Monday, and you change what you like.".into(),
        );
    }
    memo(w, prog, d, 7 * 60, format!("The February meeting: our goals for the class of {class}"), lines);
}

fn goals_ordinal(n: u32) -> String {
    let suf = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}

/// Writes your AD's goals: the campaign's slot and his boss goals for the
/// season, held as bars you agreed (`talk`), so the summer meeting keeps
/// them and his review reads them.
pub(crate) fn set_ad(w: &mut World, prog: usize, set: Vec<motives::Motive>) {
    let year = w.today.year();
    let team = Team::College(prog);
    let season = crate::world::folk::boss::goal_season(w, team);
    w.ground.campaign.goals.retain(|h| !(h.slot == Slot::Ad && h.year == year && h.dropped.is_none()));
    for m in set {
        let mut h = hold(w, prog, m.aim, Slot::Ad, m.who.clone(), m.why.clone(), None);
        h.told.ad = true;
        w.ground.campaign.goals.push(h);
        let kind = crate::world::folk::boss::GoalKind::Campaign(m.aim);
        if !w.folk.goals.iter().any(|g| g.season == season && g.kind == kind) {
            w.folk.goals.push(crate::world::folk::boss::Goal {
                kind,
                season,
                deal: 0,
                met: None,
                set_on: w.today,
                talk: true,
            });
        }
    }
}

/// Your AD set his goals for this season in the spring talk: the summer
/// meeting leaves his class goal to them. A class goal you took on
/// yourself (`folk::commit`, a bar you put up for an ask) is yours, on top
/// of his.
pub fn ad_set(w: &World, season: i32) -> bool {
    w.folk.goals.iter().any(|g| match g.kind {
        crate::world::folk::boss::GoalKind::Campaign(a) if g.season == season => {
            w.ground.campaign.goals.iter().any(|h| h.slot == Slot::Ad && h.aim == a && h.dropped.is_none())
        }
        _ => false,
    })
}

/// Whether one of your AD's campaign goals is met now: its classes so far
/// read Mostly or better (the boss's review reads it, `boss::goal_met`).
pub fn met(w: &World, team: Team, a: &Aim) -> bool {
    let Team::College(prog) = team else { return false };
    let mut parts = Vec::new();
    for c in a.classes() {
        if c > w.ground.campaign.class.max(a.class) {
            break;
        }
        let r = goals::read(w, prog, a, c);
        if r.waiting {
            return false;
        }
        parts.push(r.share);
    }
    Verdict::of(goals::share_over(&parts, a.def().classes)) >= Verdict::Mostly
}

// --- Your goals at a Monday meeting (GR37, GR40, GR41) ------------------------------------------

fn monday_only(w: &World) -> Result<(), String> {
    let s = Spring::of(w.today.year());
    if w.ground.campaign.year != s.year {
        return Err("The campaign starts at the February meeting.".into());
    }
    if w.today.weekday() != Weekday::Mon && w.today != s.meeting {
        return Err("Goals change at Monday's meeting.".into());
    }
    Ok(())
}

/// You pick your own goal at a Monday meeting (GR37, GR41): any goal on
/// the list, in place of the one you hold (which counts as dropped with
/// whoever you told). It pays by how likely it looked today; a bolder one
/// than your coordinator's ground goal pays more (GR34).
pub fn pick(w: &mut World, aim: Aim) -> Result<String, String> {
    let prog = seat(w).ok_or("The campaign is for a college head coach.")?;
    monday_only(w)?;
    check_aim(w, prog, &aim)?;
    let (slot, others) = if aim.goal == 44 { (Slot::Hunt, false) } else { (Slot::Yours, true) };
    let _ = others;
    let class = w.ground.campaign.class;
    let aim = Aim { class: aim.class.max(class), ..aim };
    let (sugg, _) = ground_goal(w, prog, class);
    let t = elapsed(w, prog);
    let sugg_likely = goals::likely(w, prog, &sugg, t);
    let mut out = Vec::new();
    if let Some(k) = w.ground.campaign.goals.iter().position(|h| h.slot == slot && h.open()) {
        if w.ground.campaign.goals[k].aim == aim {
            return Err("That's the goal you hold.".into());
        }
        out.push(drop_at(w, prog, k));
    }
    let h = hold(w, prog, aim, slot, "you".into(), String::new(), Some(sugg_likely));
    let mut s = format!("{}: {}. It looks {}", slot.words(), goals::words(w, &aim), chance_words(h.likely));
    if h.bold > 0.0 {
        s.push_str("; bolder than your coordinator's, so it pays more and costs your staff a little if you miss");
    }
    s.push('.');
    out.push(s);
    w.ground.campaign.goals.push(h);
    Ok(out.join(" "))
}

/// A goal you can name: its number on the list and what it's about.
fn check_aim(w: &World, prog: usize, a: &Aim) -> Result<(), String> {
    if !(1..=63).contains(&a.goal) {
        return Err("No such goal.".into());
    }
    let d = a.def();
    let ok = match d.subj {
        goals::Subj::None => true,
        goals::Subj::Kid => crate::world::folk::person(w, a.of).is_some(),
        goals::Subj::Area => (a.of as usize) < areas::table().len(),
        goals::Subj::Rival => {
            (a.of as usize) < w.programs.len() && a.of as usize != prog || goals::rival(w, prog).is_some()
        }
        goals::Subj::Position => (a.of as usize) < crate::positions::POSITION_COUNT,
        goals::Subj::School => (a.of as usize) < w.high_schools.len(),
        goals::Subj::State => (a.of as usize) < 51,
    };
    if !ok {
        return Err("Name what the goal is about.".into());
    }
    if d.n > 0 && a.n == 0 {
        return Err("Name its number.".into());
    }
    Ok(())
}

/// Takes the hunt (GR33) as an optional fourth goal.
pub fn hunt(w: &mut World, on: bool) -> Result<String, String> {
    let prog = seat(w).ok_or("The campaign is for a college head coach.")?;
    monday_only(w)?;
    let k = w.ground.campaign.goals.iter().position(|h| h.slot == Slot::Hunt && h.open());
    match (on, k) {
        (true, Some(_)) => Err("You're on the hunt already.".into()),
        (true, None) => {
            let class = w.ground.campaign.class;
            pick(w, Aim { goal: 44, n: 0, of: 0, class }).map(|_| "The hunt is on: find one the world missed.".into())
        }
        (false, Some(k)) => Ok(drop_at(w, prog, k)),
        (false, None) => Err("You aren't on the hunt.".into()),
    }
}

/// You drop one of your own goals at a Monday meeting (GR40): it counts
/// as a miss with whoever you told.
pub fn drop(w: &mut World, slot: Slot) -> Result<String, String> {
    let prog = seat(w).ok_or("The campaign is for a college head coach.")?;
    monday_only(w)?;
    if !slot.own() {
        return Err("Only your own goals can be dropped; your AD's change only by talking to him.".into());
    }
    let k = w.ground.campaign.goals.iter().position(|h| h.slot == slot && h.open()).ok_or("No such goal.")?;
    Ok(drop_at(w, prog, k))
}

fn drop_at(w: &mut World, prog: usize, k: usize) -> String {
    let d = w.today;
    let h = w.ground.campaign.goals[k].clone();
    pay(w, prog, &h, -1.0, 1.0, false);
    let g = &mut w.ground.campaign.goals[k];
    g.dropped = Some(d);
    for p in g.parts.iter_mut().filter(|p| p.result.is_none()) {
        p.result = Some(0.0);
        p.words = "dropped".into();
    }
    let who = told_words(&h.told);
    format!("You dropped {}: a miss with {who}.", goals::words(w, &h.aim))
}

fn told_words(t: &Told) -> String {
    let mut v = vec!["your staff"];
    if t.ad {
        v.push("your AD");
    }
    if t.press {
        v.push("the press");
    }
    match v.len() {
        1 => v[0].to_string(),
        2 => format!("{} and {}", v[0], v[1]),
        _ => format!("{}, {} and {}", v[0], v[1], v[2]),
    }
}

/// You tell your AD about one of your own goals: he judges it too, by the
/// program's level (GR34).
pub fn tell_ad(w: &mut World, slot: Slot) -> Result<String, String> {
    seat(w).ok_or("The campaign is for a college head coach.")?;
    let h = w.ground.campaign.goals.iter_mut().find(|h| h.slot == slot && h.open()).ok_or("No such goal.")?;
    if h.told.ad {
        return Err("He knows.".into());
    }
    h.told.ad = true;
    Ok("Your AD has it. He'll judge it with his own.".into())
}

/// You name a goal at media days (GR59): public, judged by the press and
/// the fans too, on the same scale.
pub fn name_at_media_days(w: &mut World, slot: Slot) -> Result<String, String> {
    seat(w).ok_or("The campaign is for a college head coach.")?;
    let s = Spring::of(w.today.year());
    if w.today < s.media.0 || w.today > s.media.1 {
        return Err("Only at media days.".into());
    }
    let k = w.ground.campaign.goals.iter().position(|h| h.slot == slot && h.open()).ok_or("No such goal.")?;
    if w.ground.campaign.goals[k].told.press {
        return Err("You said it already.".into());
    }
    w.ground.campaign.goals[k].told.press = true;
    let aim = w.ground.campaign.goals[k].aim;
    let text = format!("At media days you named a goal: {}.", goals::words(w, &aim));
    crate::world::folk::post_mine(w, w.today, crate::calendar::Loudness::Small, false, "ground: media days", &text);
    Ok(text)
}

pub fn chance_words(p: f32) -> &'static str {
    match p {
        p if p >= 0.8 => "close to sure",
        p if p >= 0.6 => "likely",
        p if p >= 0.4 => "a coin flip",
        p if p >= 0.2 => "a reach",
        _ => "a long shot",
    }
}

/// A kid would count toward one of your open goals for this class: the
/// May list ranks him ahead of the rest at the same offer (lane G3's list).
pub fn serves(w: &World, prog: usize, i: usize) -> bool {
    use crate::positions::Position as P;
    let p = &w.people[i];
    let c = &w.ground.campaign;
    let st = w.programs[prog].state;
    c.goals.iter().filter(|h| h.open() && h.aim.classes().any(|x| x == p.class_of)).any(|h| {
        let a = &h.aim;
        let pos = p.player.position;
        match a.goal {
            _ if a.def().subj == goals::Subj::Kid => a.of == p.player.id,
            5 | 6 => p.stars >= 4,
            21 | 26 => p.stars >= 5,
            11..=20 => p.home_state == st,
            28 | 29 => pos == P::QB,
            30 => pos == P::OT,
            31 => matches!(pos, P::OT | P::IOL),
            32 => matches!(pos, P::CB | P::S),
            34 => matches!(pos, P::K | P::P),
            35 => p.home_state == st && P::ALL.get(a.of as usize) == Some(&pos),
            36 => matches!(pos, P::RB | P::WR | P::CB | P::S),
            37 => pos == P::DT,
            41 => w.programs.get(a.of as usize).is_some_and(|r| r.state == p.home_state),
            44 | 49 => w.ground.finds.iter().any(|f| f.kid == p.player.id),
            57 => p.hs == a.of as usize,
            58 => p.home_state == a.of as usize,
            59..=61 => w.high_schools.get(p.hs).is_some_and(|s| s.ground_area as u32 == a.of),
            _ => false,
        }
    })
}

// --- The race through spring ---------------------------------------------------------------------

/// Each Monday: every goal's reading kept, and a warning when one slips a
/// step on the scale (in the coordinator's voice, from the world).
fn monday(w: &mut World, prog: usize, d: Date) {
    let class = w.ground.campaign.class;
    let mut warn: Vec<String> = Vec::new();
    for k in 0..w.ground.campaign.goals.len() {
        let h = w.ground.campaign.goals[k].clone();
        if !h.open() || !h.aim.classes().any(|c| c == class) || h.aim.def().later {
            continue;
        }
        let r = reading(w, prog, &h, class);
        if let Some(last) = h.last
            && Verdict::of(r.share) < Verdict::of(last)
            && Verdict::of(last) >= Verdict::Partly
        {
            warn.push(format!(
                "{}: {} (was {}, now {}).",
                goals::words(w, &h.aim),
                r.words,
                Verdict::of(last).words(),
                Verdict::of(r.share).words()
            ));
        }
        let g = &mut w.ground.campaign.goals[k];
        g.last = Some(r.share);
        g.last_words = r.words;
    }
    if !warn.is_empty() {
        memo(w, prog, d, 7 * 60 + 30, "A goal is slipping".into(), warn);
    }
}

/// The Monday letter's lines (G3's letter reads them): one per goal for
/// this class, then your coordinator's read while he guides you.
pub fn letter_lines(w: &World, prog: usize) -> Vec<String> {
    let c = &w.ground.campaign;
    let s = Spring::of(w.today.year());
    if c.year != s.year || w.today < s.meeting {
        return Vec::new();
    }
    let mut v = Vec::new();
    for h in c.goals.iter().filter(|h| h.open() && h.aim.classes().any(|x| x == c.class)) {
        if h.slot == Slot::Hunt && !w.ground.finds.iter().any(|f| f.prog as usize == prog && f.class_of == c.class) {
            continue;
        }
        let r = reading(w, prog, h, c.class);
        v.push(format!("{}: {}.", goals::words(w, &h.aim), r.words));
    }
    if c.guide.on {
        v.push(guide_line(w, prog));
    }
    v
}

/// Your coordinator's read for the week while he guides you (GR31): he
/// suggests and explains, never decides.
fn guide_line(w: &World, prog: usize) -> String {
    let c = &w.ground.campaign;
    if let Some(h) = c.goals.iter().find(|h| h.slot == Slot::Yours && h.open() && matches!(h.aim.goal, 59..=61)) {
        let r = goals::area_read(w, prog, &h.aim);
        return format!("My read: {}. A visit a week there keeps it moving.", r.words);
    }
    if w.ground.leads.is_empty() {
        return "My read: the coach's list is where leads start. Every school visit gets you his.".into();
    }
    "My read: fill the blanks on your best lead before the services get to him.".into()
}

// --- The verdict (GR30, GR39) and what it moves -------------------------------------------------

/// The verdict the week before your first game: every goal's part for
/// this class read as a pace, with what it moves; then the report.
fn verdict(w: &mut World, prog: usize, d: Date) {
    let class = w.ground.campaign.class;
    let mut lines: Vec<String> = Vec::new();
    for k in 0..w.ground.campaign.goals.len() {
        let h = w.ground.campaign.goals[k].clone();
        if !h.open() || h.parts.iter().all(|p| p.class != class) {
            continue;
        }
        let r = reading(w, prog, &h, class);
        let share = if r.waiting { 0.5 } else { r.share };
        let v = Verdict::of(share);
        if let Some(p) = w.ground.campaign.goals[k].parts.iter_mut().find(|p| p.class == class) {
            p.pace = Some(share);
            p.words = r.words.clone();
        }
        if !r.waiting {
            pay(w, prog, &h, v.score(), 1.0 / h.aim.def().classes as f32, h.told.press);
        }
        lines.push(format!(
            "{}: {}{} ({}).",
            goals::words(w, &h.aim),
            if r.waiting { "the pace says " } else { "" },
            v.words(),
            r.words
        ));
    }
    // Wishes met lift his trust a little; missing one costs nothing (GR35).
    let wishes = w.ground.campaign.wishes.clone();
    let b = crate::world::folk::boss::boss(w, Team::College(prog));
    let mut met_wishes = Vec::new();
    for m in &wishes {
        if goals::read(w, prog, &m.aim, class).share >= 0.999 {
            w.folk.mine.nudge(b.key(), WISH);
            met_wishes.push(goals::words(w, &m.aim));
        }
    }
    if !met_wishes.is_empty() {
        lines.push(format!("You met what your AD only wished for: {}.", met_wishes.join(", ")));
    }
    let rep = report::build(w, prog, d);
    lines.push(rep.ad.clone());
    if !rep.paper.is_empty() {
        lines.push(rep.paper.clone());
    }
    let c = &mut w.ground.campaign;
    c.reports.push(rep);
    if c.reports.len() > 6 {
        c.reports.remove(0);
    }
    // The first offseason ends here: every tool is open from now.
    if c.guide.on {
        c.guide.on = false;
        for t in TOOLS {
            if !c.guide.tools.iter().any(|x| x == t) {
                c.guide.tools.push(t.to_string());
            }
        }
    }
    memo(w, prog, d, 7 * 60, "The verdict at the map".into(), lines);
}

/// Your AD's trust for a goal of his (a full step of the scale).
pub const AD_TRUST: f32 = 0.04;
/// For one of yours he knows about.
pub const TOLD_TRUST: f32 = 0.025;
/// For a public goal he reads in the paper (half his own weight).
pub const PRESS_TRUST: f32 = 0.015;
/// A wish of his you met.
pub const WISH: f32 = 0.015;
/// Your staff's warmth with you, per goal.
pub const STAFF_WARMTH: f32 = 0.02;
/// What a bold goal missed costs your staff on top.
pub const BOLD_MISS: f32 = 0.02;
/// Your name with the coaches on your ground: standing points per goal.
pub const NAME_POINTS: f32 = 0.8;

/// What a goal is worth when it's made, and what it costs when it's missed,
/// by how likely it looked (GR41): a sure thing pays little and costs
/// most; a long shot pays most and costs little.
pub fn worth(likely: f32) -> (f32, f32) {
    ((1.4 - likely).clamp(0.4, 1.3), (likely + 0.3).clamp(0.4, 1.2))
}

/// Your AD reads a bold goal by your program's level (GR34): a low-tier
/// AD takes it with salt (a make surprises him more, a miss costs less),
/// LSU's takes it at its word (a miss costs more).
pub fn salt(w: &World, prog: usize) -> (f32, f32) {
    let p = w.programs[prog].prestige as f32;
    if p >= 0.75 {
        (1.0, 1.5)
    } else if p >= 0.5 {
        (1.2, 1.0)
    } else {
        (1.5, 0.5)
    }
}

/// Moves what a goal's verdict moves (`score` from the scale, `part` its
/// share of the goal's classes): your AD's trust (his goals, yours he
/// knows of, public ones at half), your staff's warmth, your name with the
/// coaches on your ground. A little each, never a firing on its own.
fn pay(w: &mut World, prog: usize, h: &Held, score: f32, part: f32, public: bool) {
    let (gain, loss) = worth(h.likely);
    let bold_gain = 1.0 + h.bold;
    let x = if score >= 0.0 { score * gain * bold_gain } else { score * loss } * part;
    let team = Team::College(prog);
    let b = crate::world::folk::boss::boss(w, team);
    let ad = if h.slot == Slot::Ad {
        AD_TRUST * x
    } else if h.told.ad {
        let (sg, sl) = if h.bold > 0.0 { salt(w, prog) } else { (1.0, 1.0) };
        TOLD_TRUST * x * if x >= 0.0 { sg } else { sl }
    } else {
        0.0
    };
    let press = if public || h.told.press { PRESS_TRUST * x } else { 0.0 };
    if ad + press != 0.0 {
        w.folk.mine.nudge(b.key(), ad + press);
    }
    // Your staff's warmth: the goals they chase with you.
    let staff = STAFF_WARMTH * x - if score < 0.0 && h.bold > 0.0 { BOLD_MISS * part } else { 0.0 };
    if staff != 0.0 && h.slot != Slot::Ad {
        for m in week::road::men(w, prog) {
            if Some(m) != w.programs[prog].head_coach {
                crate::world::folk::network::touch(w, m, staff);
            }
        }
    }
    // Your name with the coaches who know your program.
    let pts = NAME_POINTS * x;
    if pts.abs() >= 0.05 {
        let hold = crate::world::ground::Holder::Program(prog as u32);
        for m in w.ground.memories.iter_mut().filter(|m| m.holder == hold && m.points >= 25.0) {
            if matches!(m.who, crate::world::ground::Who::Coach(_)) {
                m.points = (m.points + pts).clamp(0.0, 100.0);
            }
        }
    }
    if public || h.told.press {
        let text = format!(
            "The paper on {}'s goal ({}): {}.",
            w.program_name(prog),
            goals::words(w, &h.aim),
            Verdict::of(if score >= 0.999 {
                1.0
            } else if score >= 0.5 {
                0.7
            } else if score > -0.5 {
                0.4
            } else {
                0.0
            })
            .words()
        );
        crate::world::folk::post_mine(w, w.today, crate::calendar::Loudness::Medium, false, "ground: verdict", &text);
    }
}

/// Classes settle as they sign; later goals when the class has played
/// (checked at each signing day and verdict). A goal is settled when
/// every part has its result.
fn settle(w: &mut World, prog: usize, d: Date) {
    let s = Spring::of(d.year());
    let check_later = d == s.signing.add(1) || d == verdict_day(w, prog, s.year);
    for k in 0..w.ground.campaign.goals.len() {
        let h = w.ground.campaign.goals[k].clone();
        if !h.open() {
            continue;
        }
        let later = h.aim.def().later;
        let mut changed = false;
        let mut parts = h.parts.clone();
        for p in parts.iter_mut().filter(|p| p.result.is_none()) {
            let due = if later { check_later && d > signing_of(p.class) } else { d == signing_of(p.class).add(1) };
            if !due {
                continue;
            }
            let r = goals::read(w, prog, &h.aim, p.class);
            if r.waiting {
                continue;
            }
            p.result = Some(r.share);
            p.words = r.words.clone();
            changed = true;
            if later {
                pay(w, prog, &h, Verdict::of(r.share).score(), 1.0 / h.aim.def().classes as f32, h.told.press);
            }
        }
        if changed {
            let done = parts.iter().all(|p| p.result.is_some());
            let share =
                goals::share_over(&parts.iter().filter_map(|p| p.result).collect::<Vec<_>>(), h.aim.def().classes);
            let text = format!(
                "{}: {} ({}).",
                goals::words(w, &h.aim),
                Verdict::of(share).words(),
                parts.last().map_or(String::new(), |p| p.words.clone())
            );
            let g = &mut w.ground.campaign.goals[k];
            g.parts = parts;
            g.settled = done;
            if done {
                crate::world::folk::note(w, "recruiting", "Your goals".into(), text, None);
            }
        }
    }
}

/// The areas the report follows: your state's, then the states your men
/// recruit (up to 24 areas, the most talent first).
pub fn report_areas(w: &World, prog: usize) -> Vec<u16> {
    let mut states: Vec<usize> = vec![w.programs[prog].state];
    for (_, v) in w.scouting.dept.areas.iter() {
        for &s in v {
            if !states.contains(&(s as usize)) {
                states.push(s as usize);
            }
        }
    }
    states.truncate(3);
    let talent = standing::talents(w);
    let mut by: Vec<(f32, u16)> = Vec::new();
    for (k, a) in areas::table().iter().enumerate() {
        if states.contains(&a.state) {
            let t: f32 = w
                .high_schools
                .iter()
                .filter(|h| h.ground_area == k as u16)
                .map(|h| talent.get(h.id).copied().unwrap_or(1.0))
                .sum();
            by.push((t, k as u16));
        }
    }
    by.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    by.into_iter().take(24).map(|x| x.1).collect()
}

/// The catalogue for the screens: every goal with its words, group, classes,
/// who raises it and how likely it looks for your class today.
pub fn catalogue(w: &World) -> Vec<(goals::Def, f32)> {
    let Some(prog) = seat(w) else { return Vec::new() };
    let class = w.ground.campaign.class.max(class_now(w));
    let t = elapsed(w, prog);
    goals::GOALS
        .iter()
        .map(|d| {
            let a = Aim { goal: d.id, n: d.n, of: default_of(w, prog, d), class };
            (*d, if d.subj == goals::Subj::None || a.of > 0 { goals::likely(w, prog, &a, t) } else { 0.0 })
        })
        .collect()
}

/// A goal's subject when you don't name one: the rival, your campus area.
fn default_of(w: &World, prog: usize, d: &goals::Def) -> u32 {
    match d.subj {
        goals::Subj::Rival => goals::rival(w, prog).map_or(0, |r| r as u32),
        goals::Subj::Area => goals::campus_area(w, prog).map_or(0, |a| a as u32),
        goals::Subj::State => w.programs[prog].state as u32,
        _ => 0,
    }
}
