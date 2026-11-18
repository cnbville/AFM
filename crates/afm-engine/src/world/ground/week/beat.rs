//! Every week from January to August has its beat (GR56): what is on the
//! year's real calendar that week, your beat and what is at stake, read from
//! each year's dates (`calendar::anchors`) and recruiting periods
//! (`calendar::recruiting_period`), never from a table of fixed dates. The
//! 35-week sheet in `docs/design/ground/months.md` is the 2026 reading of
//! these rules. From September the season runs and Ground goes on under it.

use crate::calendar::{Date, Period, Weekday, YearDates, recruiting_period};
use crate::world::World;

/// One week's beat.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Beat {
    /// Week 1 is the week of January 1; weeks past 35 are the season's.
    pub week: u8,
    pub monday: Date,
    /// "Quiet", "Contact from Apr 15", "Dead to Feb 4, then quiet".
    pub period: String,
    /// What is on the calendar, dated, in order.
    pub on: Vec<(Date, String)>,
    /// Your beat ("The road opens Wednesday; the spring game is Saturday").
    pub beat: String,
    /// What is at stake.
    pub stake: String,
}

impl Beat {
    /// The Monday letter's first line: "Week 16: the road opens Wednesday,
    /// the spring game is Saturday."
    pub fn line(&self) -> String {
        let mut b = self.beat.clone();
        if let Some(c) = b.get(0..1) {
            b = c.to_lowercase() + &b[1..];
        }
        format!("Week {}: {b}.", self.week)
    }
}

/// The season's dates a day belongs to (the football year runs from the
/// day after the Super Bowl).
pub fn dates_for(w: &World, d: Date) -> YearDates {
    if d >= w.dates.start && d <= w.dates.super_bowl {
        return w.dates.clone();
    }
    let y = YearDates::new(d.year());
    if d >= y.start { y } else { YearDates::new(d.year() - 1) }
}

/// The week number of a Monday: week 1 holds January 1.
pub fn week_number(monday: Date) -> u8 {
    let jan1 = Date::ymd(monday.add(6).year(), 1, 1);
    let first = jan1.on_or_before(Weekday::Mon);
    (1 + first.days_to(monday).div_euclid(7)).clamp(1, 53) as u8
}

const DAY: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// "Apr 15".
pub fn short(d: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (_, m, day) = d.parts();
    format!("{} {day}", M[m as usize - 1])
}

/// Memorial Day: the last Monday of May.
pub fn memorial_day(year: i32) -> Date {
    Date::last_weekday(year, 5, Weekday::Mon)
}

/// The day the May list comes in (GR57): the day after Memorial Day.
pub fn may_list_day(year: i32) -> Date {
    memorial_day(year).add(1)
}

/// The recruiting period on a day, read with the right season's dates.
fn period_on(w: &World, d: Date) -> Period {
    recruiting_period(&dates_for(w, d), d)
}

/// The week's periods in words.
fn period_words(w: &World, monday: Date) -> String {
    let ps: Vec<Period> = (0..7).map(|k| period_on(w, monday.add(k))).collect();
    let name = |p: Period| match p {
        Period::Contact => "contact",
        Period::Evaluation => "evaluation",
        Period::Quiet => "quiet",
        Period::Dead => "dead",
    };
    let mut runs: Vec<(Period, i32)> = Vec::new();
    for (k, &p) in ps.iter().enumerate() {
        if runs.last().is_none_or(|r| r.0 != p) {
            runs.push((p, k as i32));
        }
    }
    let cap = |s: &str| s[..1].to_uppercase() + &s[1..];
    match runs.len() {
        1 => cap(name(runs[0].0)),
        2 => {
            let (a, b) = (runs[0], runs[1]);
            if b.0 == Period::Dead || a.0 == Period::Dead && b.1 >= 4 {
                format!("{}, {} from {}", cap(name(a.0)), name(b.0), short(monday.add(b.1)))
            } else if a.0 == Period::Dead {
                format!("Dead to {}, then {}", short(monday.add(b.1 - 1)), name(b.0))
            } else {
                format!("{} from {}", cap(name(b.0)), short(monday.add(b.1)))
            }
        }
        _ => cap(&runs
            .iter()
            .map(|r| format!("{} from {}", name(r.0), short(monday.add(r.1))))
            .collect::<Vec<_>>()
            .join(", ")),
    }
}

/// The first day of the spring road (the contact or evaluation period in
/// April) and its last day, for a season.
pub fn spring_road(y: &YearDates) -> Option<(Date, Date)> {
    let s = y.season;
    let open = |d: Date| matches!(recruiting_period(y, d), Period::Contact | Period::Evaluation);
    let first = (0..61).map(|k| Date::ymd(s, 4, 1).add(k)).find(|&d| open(d))?;
    let mut last = first;
    while open(last.add(1)) && last.add(1).month() <= 6 {
        last = last.add(1);
    }
    Some((first, last))
}

/// The first day of June's summer dead period.
fn summer_dead(y: &YearDates) -> Option<Date> {
    (0..40).map(|k| Date::ymd(y.season, 6, 3).add(k)).find(|&d| recruiting_period(y, d) == Period::Dead)
}

/// Your June camps: the second and third Saturdays (built,
/// `scouting::dept`, `college_day`).
pub fn camp_days(season: i32) -> [Date; 2] {
    [Date::nth_weekday(season, 6, Weekday::Sat, 2), Date::nth_weekday(season, 6, Weekday::Sat, 3)]
}

/// The week's beat.
pub fn beat(w: &World, monday: Date) -> Beat {
    let monday = monday.on_or_before(Weekday::Mon);
    let sun = monday.add(6);
    let week = week_number(monday);
    let y = dates_for(w, monday.add(3));
    // The January before a season's dates is the last season's, and the
    // spring's are this one's.
    let prev = if y.start > Date::ymd(sun.year(), 1, 1) { YearDates::new(y.season - 1) } else { y.clone() };
    let s = sun.year();
    let inside = |d: Date| d >= monday && d <= sun;
    let mut on: Vec<(Date, String)> = Vec::new();
    let mut add = |d: Date, t: &str| {
        if inside(d) {
            on.push((d, t.to_string()));
        }
    };
    // January: the portal and the takeover, the coaching changes, the
    // services' winter update, signing day.
    let (po, pc) = crate::calendar::anchors::january_window(s - 1);
    add(po, "The portal opens");
    add(pc, "The portal closes");
    add(Date::ymd(s, 1, 20), "High school coaching changes");
    for (d, k) in crate::world::scouting::services::update_days(s) {
        if k != "fall" {
            add(d, &format!("The services' {k} update"));
        }
    }
    let nsd = if prev.signing_day.year() == s { prev.signing_day } else { y.signing_day };
    add(nsd, "Signing day, 7 AM");
    // Lane G6: your AD's talk the day after, the February meeting the
    // Monday after, the coordinator at your door in your first season, and
    // the verdict the week before your first game.
    add(nsd.add(1), "Your AD's office: the class and what's next");
    add(nsd.add(1).on_or_after(Weekday::Mon), "The February meeting");
    if let Some(door) = w.ground.campaign.guide.door {
        add(door, "Your coordinator at your door");
    }
    if let Some(prog) = crate::world::ground::campaign::seat(w) {
        add(crate::world::ground::campaign::verdict_day(w, prog, y.season), "The verdict at the map");
    }
    let jd = y.junior_days;
    for k in 0..9 {
        let sat = jd.0.on_or_after(Weekday::Sat).add(7 * k);
        if sat <= jd.1 && sat > nsd {
            add(sat, "Junior day");
        }
    }
    add(y.combine_start, "The NFL combine");
    add(y.spring_practice.0, "Spring practice opens");
    add(y.official_visits_from, "Official visits open for juniors");
    let road = spring_road(&y);
    if let Some((a, b)) = road {
        add(a, "The spring road opens");
        add(b, "The road closes");
    }
    add(y.spring_game, "The spring game");
    add(y.draft[0].date, "The NFL draft");
    add(y.spring_term_end, "Your spring term ends");
    add(memorial_day(s), "Memorial Day");
    add(may_list_day(s), "The May list");
    add(y.summer_enrollment, "Your signees arrive");
    for (k, d) in camp_days(y.season).into_iter().enumerate() {
        add(d, if k == 0 { "Your first camp" } else { "Your second camp" });
    }
    add(crate::world::hs_year::transfer_day(y.season), "High school transfers and reclasses");
    if let Some(d) = summer_dead(&y) {
        add(d, "The summer dead period opens");
    }
    add(Date::ymd(s, 7, 1), "NIL payday");
    add(Date::ymd(s, 7, 4), "The Fourth");
    add(y.media_days.0, "Media days open");
    add(y.media_days.1, "Media days close");
    add(y.coaches_preseason_poll, "The coaches' poll");
    add(y.fall_camp, "Fall camp opens");
    add(y.ap_preseason_poll, "The AP poll");
    add(y.hs_school_start, "High schools open");
    if let Some(&w0) = y.cfb_weeks.first() {
        add(w0, "Week 0");
    }
    on.sort_by_key(|x| x.0);
    on.dedup();
    let has = |t: &str| on.iter().find(|x| x.1.starts_with(t)).map(|x| x.0);
    let day = |d: Date| DAY[d.weekday().index() as usize];
    let period = period_words(w, monday);
    let open = |d: Date| matches!(period_on(w, d), Period::Contact | Period::Evaluation);
    let (beat, stake): (String, &str) = if let Some(d) = has("Signing day") {
        (format!("signing day is {}, then your AD's office", day(d)), "The class you start from")
    } else if has("The February meeting").is_some() {
        ("the February meeting: the spring's goals, then the road plan".into(), "Your goals")
    } else if has("The verdict at the map").is_some() {
        ("the verdict before the season".into(), "The verdict")
    } else if sun.month() == 1 && monday <= pc {
        ("Ground waits: the takeover and the portal come first".into(), "Your roster")
    } else if monday.month() == 1 && nsd > sun {
        if nsd <= sun.add(7) {
            ("the last calls and home visits before signing day".into(), "The class")
        } else {
            (
                "your staff on the road: home visits with your commits and next year's schools".into(),
                "Saving the class; the first find",
            )
        }
    } else if has("Spring practice opens").is_some() {
        ("spring practice opens: your first open practice for coaches".into(), "Warmth with many coaches")
    } else if has("Official visits open").is_some() {
        ("official visit invitations go out: the kids book their weekends for late May and June".into(), "Who visits")
    } else if let Some(d) = has("The spring road opens") {
        let mut b = format!("the road opens {}", day(d));
        if let Some(g) = has("The spring game") {
            b.push_str(&format!(", the spring game is {}", day(g)));
        }
        (b, "First leads")
    } else if road.is_some_and(|(a, _)| a > sun && a.days_to(sun) > -8) {
        ("the road plan for the opening: who goes where first".into(), "Being first")
    } else if let Some(d) = has("The road closes") {
        (format!("the road's last days: it closes {}", day(d)), "The days left")
    } else if has("The NFL draft").is_some() && open(monday.add(1)) {
        ("the road, with your draft picks in every pitch".into(), "Proof")
    } else if has("The services' spring update").is_some() && open(monday.add(1)) {
        ("the road, and your finds against the services' new lists".into(), "The hunt")
    } else if open(monday.add(1)) && monday.month() <= 6 {
        ("the road: your men at high school spring practices".into(), "Names")
    } else if has("The May list").is_some() {
        ("the May list, then your first official visit weekend after the road".into(), "Who gets your officials")
    } else if has("Your signees arrive").is_some() {
        ("your first real look at the class you signed; an official visit weekend".into(), "Commitments")
    } else if has("Your first camp").is_some() || has("Your second camp").is_some() {
        ("your camp, and an official visit weekend".into(), "Names, times, offers")
    } else if has("The summer dead period").is_some() {
        ("the phones: the kids decide after their officials".into(), "Commitments")
    } else if has("NIL payday").is_some() {
        ("the announcements and the flips".into(), "Holding what you have")
    } else if has("Media days open").is_some() {
        ("media days".into(), "Your name")
    } else if sun.month() == 7 && monday.day() >= 18 {
        ("your state's coaches' convention and your AD's season goals".into(), "His season goals")
    } else if sun.month() == 7 {
        ("a quiet July: a week off, if you take it".into(), "Rest, your family")
    } else if has("Fall camp opens").is_some() || monday.month() == 8 && monday.day() < 17 {
        ("recruits at camp practice; the phones".into(), "The race in public")
    } else if monday.month() == 8 {
        ("the verdict before the season".into(), "The verdict")
    } else if has("Junior day").is_some() {
        ("junior day: the families on campus, and who you offer".into(), "The juniors' first look at you")
    } else {
        match period_on(w, monday.add(2)) {
            Period::Quiet => ("your campus: visits and the phones".into(), "Interest"),
            Period::Dead => ("the phones".into(), "Holding what you have"),
            Period::Evaluation => ("Friday nights and your evaluation days".into(), "Names"),
            Period::Contact => ("the road and home visits".into(), "Commitments"),
        }
    };
    Beat { week, monday, period, on, beat: beat_cap(beat), stake: stake.to_string() }
}

fn beat_cap(s: String) -> String {
    match s.get(0..1) {
        Some(c) => c.to_uppercase() + &s[1..],
        None => s,
    }
}
