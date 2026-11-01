//! Section 4: the daily clock (T1). The world runs one day at a time, from
//! the day after one Super Bowl to the day after the next (a football year),
//! and every system acts on its own real day under that day's rule book:
//! the NFL offseason and season, the college year (camp, Saturdays, polls,
//! the postseason by era, the portal and the draft deadlines), each state's
//! high school season, recruiting stages and the coaching carousels.
//!
//! The 30 hidden years, the 2026 start and every later year run through
//! this same clock (one code path, build plan rule 1).

use super::college::{self, PollKind};
use super::data::Division;
use super::{CalendarLog, World, hs_year, nfl_year, recruiting, staff};
use crate::calendar::data::MEDIA_DAYS;
use crate::calendar::rules::TransferRule;
use crate::calendar::{Date, Event, Kind, Level, Loudness, Moment, Weekday, YearDates};

/// Time spent per step, printed each year with AFM_PROFILE set (tooling).
static PROFILE: std::sync::Mutex<Vec<(&'static str, f64)>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn timed<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    if std::env::var_os("AFM_PROFILE").is_none() {
        return f();
    }
    let t = std::time::Instant::now();
    let out = f();
    let dt = t.elapsed().as_secs_f64();
    let mut v = PROFILE.lock().unwrap();
    match v.iter_mut().find(|x| x.0 == name) {
        Some(x) => x.1 += dt,
        None => v.push((name, dt)),
    }
    out
}

fn print_profile(year: i32) {
    if std::env::var_os("AFM_PROFILE").is_none() {
        return;
    }
    let mut v = std::mem::take(&mut *PROFILE.lock().unwrap());
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    let parts: Vec<String> = v.iter().take(24).map(|(n, t)| format!("{n} {t:.2}s")).collect();
    eprintln!("PROFILE {year}: {}", parts.join(", "));
}

/// One football year: days until the next one starts.
pub(crate) fn advance_year(w: &mut World) {
    let year = w.year;
    while w.year == year {
        advance_day(w);
    }
}

/// Which day of a portal window it is: the windows by era (T3, T11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum PortalWindow {
    Winter,
    Spring,
    /// After a head coach change (T3): 30 days from five days after the
    /// carousel (seasons 2022-2024), then 15 days from five days after the
    /// new coach's hire (2025 on).
    CoachChange,
    /// After May commencement (part B 36): graduates with a season left,
    /// outside the windows until 2024 (graduate transfers were exempt).
    Graduate,
}

/// The transfer windows of the offseason after `season`: (opens, closes,
/// which). Before 2018 there was no portal: a transfer asked his school for
/// a release after the season (modelled as a January "window"). 2018-2021
/// the portal was open all year; most entered after the season. From the
/// 2022 season: a winter window and a spring window; from the 2025 season
/// one January window (Jan 2-16).
pub(crate) fn portal_windows(season: i32, rule: TransferRule) -> Vec<(Date, Date, PortalWindow)> {
    let s = season;
    match rule {
        TransferRule::Permission | TransferRule::OpenPortal => {
            // No windows: players left after their season (each entrant's
            // day follows his team's last game, part D 21) and again after
            // spring practice, to the end of May (part D 22).
            let spring = Date::nth_weekday(s + 1, 4, Weekday::Sat, 3).add(2);
            vec![
                (Date::ymd(s, 12, 1).on_or_after(Weekday::Mon), Date::ymd(s + 1, 1, 31), PortalWindow::Winter),
                (spring, Date::ymd(s + 1, 5, 31), PortalWindow::Spring),
            ]
        }
        TransferRule::Windows => {
            let (open, close) = match s {
                2022 => (Date::ymd(2022, 12, 5), Date::ymd(2023, 1, 18)),
                2023 => (Date::ymd(2023, 12, 4), Date::ymd(2024, 1, 2)),
                _ => (Date::ymd(s, 12, 9), Date::ymd(s, 12, 28)),
            };
            let spring = if s == 2024 {
                (Date::ymd(2025, 4, 16), Date::ymd(2025, 4, 25))
            } else {
                (Date::ymd(s + 1, 4, 15), Date::ymd(s + 1, 4, 30))
            };
            vec![(open, close, PortalWindow::Winter), (spring.0, spring.1, PortalWindow::Spring)]
        }
        TransferRule::January => vec![(Date::ymd(s + 1, 1, 2), Date::ymd(s + 1, 1, 16), PortalWindow::Winter)],
    }
}

/// The commitment waves of the class signing next (T5.2): after junior
/// days (part D 12), after the spring evaluation period, in mid-June after
/// the camps and junior officials (part D 13), at the end of summer and at
/// the end of October.
fn commit_wave(y: &YearDates, d: Date) -> Option<u8> {
    [(3, 20), (4, 30), (6, 15), (8, 31), (10, 31)]
        .iter()
        .position(|&(m, day)| Date::ymd(y.season, m, day) == d)
        .map(|k| k as u8)
}

/// The college all-star games of a football year's January, with their dates.
fn all_star_games(y: &YearDates) -> [(Date, &'static str); 4] {
    [
        (y.senior_bowl, "Senior Bowl"),
        (y.shrine_bowl, "East-West Shrine Bowl"),
        (y.hbcu_bowl, "HBCU Legacy Bowl"),
        (y.nflpa_bowl, "NFLPA Collegiate Bowl"),
    ]
}

/// The one day the college all-star games name their rosters (F7): a week
/// before the first game held, but after the FBS and FCS title games, so
/// the finalists' seniors are in the pool, and after the draft withdrawal
/// day when the first game allows it, so the declared underclassmen are
/// all in and none who withdraws holds an invite. A first game before the
/// withdrawal day names them the day before it.
pub(crate) fn all_star_roster_day(y: &YearDates) -> Date {
    let first = all_star_games(y)
        .iter()
        .filter(|g| recruiting::all_star_held(g.1, y.season + 1))
        .map(|g| g.0)
        .min()
        .expect("the Senior Bowl is held every year");
    let fcs_final = y.fcs_rounds.last().copied().unwrap_or(y.title_game);
    first.add(-7).max(y.title_game.add(1)).max(fcs_final.add(1)).max(y.declare_withdraw.min(first.add(-1)))
}

/// The day high school coaching changes are made (January, after the state
/// finals and the college carousel's hires).
fn hs_carousel_day(y: &YearDates) -> Date {
    Date::ymd(y.season + 1, 1, 20)
}

/// The day June transfers happen: each state's transfer season, after the
/// school year ends.
fn hs_transfer_day(y: &YearDates) -> Date {
    Date::ymd(y.season, 6, 15)
}

/// Bodies grow by the date (update 3, A16): each person's years past his
/// whole-year age today, from his birthday.
fn grow_bodies(w: &mut World, d: Date) {
    let unknown = crate::calendar::born::unknown();
    for p in &mut w.people {
        p.player.grow = if p.born == unknown {
            0.0
        } else {
            let years = f64::from(d.0 - p.born.0) / 365.2425;
            (years - f64::from(p.player.age)).clamp(0.0, 2.0) as f32
        };
    }
}

/// Day `d` of football year `y`.
pub(crate) fn advance_day(w: &mut World) {
    // The world log's snapshots of the key dates, before anything acts.
    super::wlog::day(w, w.today);
    // A war room left open (a day passed without finishing the draft): the
    // staff finish it on the board.
    if w.scouting.live.is_some() {
        super::scouting::live::finish(w);
    }
    let d = w.today;
    let y = w.dates.clone();
    grow_bodies(w, d);
    // A live game left from a day gone by (called off, or never booked).
    if w.live.as_ref().is_some_and(|l| l.date < d) {
        w.live = None;
    }
    if d == y.start {
        year_start(w);
    }
    // --- Days of last football year that fall after its Super Bowl ----------------
    if let Some(p) = w.prev_dates.clone()
        && d == p.signing_day
        && d > p.super_bowl
    {
        signing_day_and_book(w, p.season);
    }

    // --- The books: every club's money, projects and the dated money days ------------
    timed("money", || super::finance::day(w, d));

    anniversaries(w, d);
    timed("recruiting days", || recruiting::recruiting_day(w, d));
    recruiting::announce_commitments(w, d);

    // --- The NFL year (T2, A1-A17): its own dated steps -----------------------------
    timed("nfl", || nfl_year::day(w, d));
    // Your front office's day: reminders, answers, calls (lane 10).
    super::front_office::day(w, d);

    // --- Spring: college spring practice and high school spring ball (T4.1) ----------
    if d == y.spring_practice.0 {
        hs_year::spring_notices(w);
        // Staffs install their systems for the year (S3-S5).
        super::systems::spring_install(w);
        super::systems::set_spots(w);
    }
    if d == y.spring_game {
        hs_year::spring_game(w);
    }
    if d == y.junior_days.0 {
        post(
            w,
            d,
            Level::Recruiting,
            Kind::Window,
            Loudness::Small,
            "Junior days open: programs host juniors on campus.",
        );
    }
    if d == y.official_visits_from && w.rules.junior_officials {
        post(w, d, Level::Recruiting, Kind::Window, Loudness::Small, "Juniors may take official visits from today.");
    }
    // The draft: three days in April (T2), on the anchors' hours.
    if d == y.draft[0].date {
        timed("draft", || recruiting::draft(w));
    }
    // The spring portal window (2023-2025).
    for (open, close, which) in portal_windows(w.year - 1, transfer_rule_of(w, w.year - 1)) {
        if which == PortalWindow::Spring && d == open {
            timed("portal open", || recruiting::portal_open(w, which, close));
        }
    }
    if d == y.spring_term_end {
        college::spring_term_check(w);
    }
    // Commencement, the second Saturday of May, and the graduate transfers
    // who move after it (part B 36): exempt from the windows until 2024.
    if d == Date::nth_weekday(w.year, 5, Weekday::Sat, 2) {
        college::may_graduation(w);
        if !matches!(transfer_rule_of(w, w.year - 1), TransferRule::January) {
            timed("portal open", || recruiting::portal_open(w, PortalWindow::Graduate, Date::ymd(w.year, 6, 30)));
        }
    }
    if d == y.summer_enrollment {
        // Walk-on tryouts and finds at the colleges in their build year (SK68).
        super::new_teams::college::tryouts(w);
        super::eligibility::summer_roads(w);
        recruiting::enroll(w, false);
    }
    if d == hs_transfer_day(&y) {
        timed("hs transfers", || recruiting::high_school_transfers(w));
        w.rebuild_rosters();
    }
    if d == y.money_year {
        timed("nil", || recruiting::nil_payouts(w));
        college_changes(w);
    }
    if d >= y.media_days.0 && d <= y.media_days.1 && d.weekday() != Weekday::Sat && d.weekday() != Weekday::Sun {
        let k = y.media_days.0.days_to(d) as usize;
        if let Some(conf) = MEDIA_DAYS.get(k) {
            let text = format!("{conf} media days.");
            post(w, d, Level::College, Kind::Date, Loudness::Small, &text);
        }
    }
    // Dated offers before a class's first wave (part B 34).
    let season = w.year;
    for (date, class_of, pass) in [
        (y.junior_days.0.add(14), season + 1, 4),
        (Date::ymd(season, 6, 15), season + 2, 1),
        (Date::ymd(season, 9, 1), season + 2, 2),
        (Date::ymd(season + 1, 1, 15), season + 3, 0),
        (Date::ymd(season + 1, 1, 15), season + 2, 3),
    ] {
        if d == date {
            timed("early offers", || recruiting::early_offers(w, d, class_of, pass));
        }
    }
    if let Some(wave) = commit_wave(&y, d) {
        timed("sign summer", || recruiting::signing_stage(w, recruiting::SignStage::Summer(wave), w.year));
    }

    // --- Fall: camp, the new high school year, polls -------------------------------
    if d == y.fall_camp {
        // Last offseason's coaching changes are done with.
        w.late_windows.clear();
        w.coach_follows.clear();
        recruiting::camp_walk_ons(w);
        timed("college start", || college::start(w));
        college::lean_checkpoint(w, college::LeanCheck::FallCamp);
    }
    if d == y.coaches_preseason_poll {
        college::release_poll(w, d, PollKind::CoachesPreseason);
    }
    if d == y.ap_preseason_poll {
        college::release_poll(w, d, PollKind::ApPreseason);
    }
    timed("hs", || hs_year::day(w, d));
    super::times::day(w, d);

    // --- College Saturdays and the weeks around them (T3, T6) ------------------------
    timed("college games", || college::play_day(w, d));
    college::bowl_practices(w, d);
    if d.weekday() == Weekday::Sun && d > y.cfb_weeks[0] && d <= y.selection_day {
        let kind = if d == y.selection_day { None } else { Some(PollKind::Ap) };
        if let Some(k) = kind {
            college::release_poll(w, d, PollKind::Coaches);
            college::release_poll(w, d, k);
            // The Harris poll (2005-2013) from the season's fifth week.
            if harris_year(w) && y.cfb_weeks.len() > 4 && d > y.cfb_weeks[4] {
                college::release_poll(w, d, PollKind::Harris);
            }
            if w.rules.cfb_postseason == crate::calendar::CfbPostseason::Bcs && d >= y.cfb_weeks[0].add(7 * 7 + 1) {
                college::release_poll(w, d, PollKind::Bcs);
            }
        }
    }
    // Playoff committee rankings on Tuesdays (2014 on).
    if d.weekday() == Weekday::Tue
        && d >= y.first_cfp_ranking
        && d < y.selection_day
        && matches!(
            w.rules.cfb_postseason,
            crate::calendar::CfbPostseason::FourTeam | crate::calendar::CfbPostseason::TwelveTeam
        )
    {
        college::release_poll(w, d, PollKind::Cfp);
    }
    if y.cfb_weeks.len() > 4 && d == y.cfb_weeks[4].add(1) {
        college::redshirt_trick(w);
    }
    if y.cfb_weeks.len() > 8 && d == y.cfb_weeks[8].add(1) {
        college::lean_checkpoint(w, college::LeanCheck::MidSeason);
    }
    for (div, end) in [
        (Division::JUCO, y.juco_regular_end),
        (Division::D2, y.d2_regular_end),
        (Division::NAIA, y.d2_regular_end),
        (Division::D3, y.d2_regular_end),
        (Division::FCS, y.fcs_regular_end),
    ] {
        if d == end.add(1) {
            college::regular_season_end(w, div);
        }
    }
    // The Sunday after rivalry week: the regular season's book, then the
    // college coaching carousel (firings come before the title games).
    if d == y.fbs_regular_end.add(1) {
        super::program::announce_drops(w);
        college::record_regular_season(w);
        college::regular_season_end(w, Division::FBS);
        // Each head coach who leaves moves his players the day he goes
        // (`college::coach_gone`, minor bug 88).
        timed("carousel college", || staff::carousel(w, staff::Scope::College));
        // A head coach change opens its own transfer window five days later
        // (T3): 30 days long in the windows era (2022-2024), 15 days from 2025 on.
        let days = match w.changes.rules(w.year).transfer {
            TransferRule::Windows => Some(30),
            TransferRule::January => Some(15),
            _ => None,
        };
        if let Some(days) = days {
            w.coach_window = Some((d.add(5), d.add(5 + days)));
        }
        timed("draft reads", || recruiting::draft_reads(w));
    }
    // In-season firings (part B 38): the Sundays from late September to mid-November.
    if d.weekday() == Weekday::Sun && (4..=10).any(|k| y.cfb_weeks.get(k).is_some_and(|s| d == s.add(1))) {
        staff::in_season_firings(w);
    }
    // College coaching searches decide on their own days (part D 18); at
    // fall camp any still open decide at once.
    if !w.searches.is_empty() {
        timed("searches", || staff::college_searches_day(w, d == y.fall_camp));
    }
    if d == Date::ymd(y.season + 1, 1, 31) && w.recording {
        staff::diversity_report(w);
    }
    if d == y.selection_day {
        // The human polls vote after the title games; the BCS standings
        // read them, the committee ranks on its own.
        college::release_poll(w, d, PollKind::Coaches);
        college::release_poll(w, d, PollKind::Ap);
        if harris_year(w) {
            college::release_poll(w, d, PollKind::Harris);
        }
        match w.rules.cfb_postseason {
            crate::calendar::CfbPostseason::Bcs => college::release_poll(w, d, PollKind::Bcs),
            crate::calendar::CfbPostseason::PollsOnly => {}
            _ => college::release_poll(w, d, PollKind::Cfp),
        }
        timed("selection", || college::selection_day(w));
        college::lean_checkpoint(w, college::LeanCheck::Bowls);
    }
    if d == y.heisman_finalists {
        college::awards(w, "finalists");
    }
    if d == y.all_america {
        recruiting::honors(w);
    }
    if d == y.awards_show {
        college::awards(w, "show");
    }
    if d == y.heisman {
        college::awards(w, "heisman");
    }
    if Some(d) == y.early_signing {
        timed("sign early", || recruiting::signing_stage(w, recruiting::SignStage::Early, w.year));
        recruiting::class_rankings_news(w, w.year + 1, "Early signing period");
        college::lean_checkpoint(w, college::LeanCheck::EarlySigning);
    }
    if d == Date::ymd(w.year, 12, 15) {
        recruiting::advisory_grades(w);
    }
    if d == y.finals_week.0 {
        college::finals_week(w);
    }
    if d == y.december_graduation {
        college::december_graduation(w);
    }
    // --- The offseason's windows (T3): portal, draft deadlines, enrollment ----------
    let season = w.year;
    let rule = w.rules.transfer;
    // The season's eligibility settles before a window that opens the same
    // day reads it (lane 10: a burned redshirt counted only after).
    if d == Date::ymd(season + 1, 1, 2) {
        college::settle_eligibility(w);
    }
    for (open, close, which) in portal_windows(season, rule) {
        if which == PortalWindow::Winter && d == open {
            recruiting::portal_open(w, which, close);
        }
    }
    if let Some((open, close)) = w.coach_window
        && d == open
    {
        w.coach_window = None;
        recruiting::portal_open(w, PortalWindow::CoachChange, close);
    }
    // A head coach who left after that window opened: his players' own
    // window, five days after the new hire (minor bug 88), the day's
    // programs together.
    if w.late_windows.iter().any(|x| x.1.is_some_and(|(open, _)| open == d)) {
        let (today, later): (Vec<_>, Vec<_>) =
            w.late_windows.iter().copied().partition(|x| x.1.is_some_and(|(open, _)| open == d));
        w.late_windows = later;
        let close = today.iter().filter_map(|x| x.1.map(|t| t.1)).max().unwrap_or(d);
        w.window_programs = today.iter().map(|x| x.0).collect();
        recruiting::portal_open(w, PortalWindow::CoachChange, close);
        w.window_programs.clear();
    }
    // Your players who talked to you first decide on their day.
    if !w.portal_intents.is_empty() {
        recruiting::intents_day(w);
    }
    // Entrants decide on their own days (T3, T9).
    if !w.portal_pending.is_empty() {
        timed("portal day", || recruiting::portal_day(w, false));
    }
    // Early entrants announce on their own days (part D 17).
    if d > y.fbs_regular_end && d < y.declare_deadline {
        recruiting::declarations(w, recruiting::Declare::Soft);
    }
    if d == y.declare_deadline {
        recruiting::declarations(w, recruiting::Declare::Deadline);
    }
    if d == y.declare_withdraw {
        recruiting::declarations(w, recruiting::Declare::Withdraw);
    }
    // The playoff teams' players declare once their season is over (T3),
    // and have their own 72 hours to take it back.
    if d == y.title_game.add(4) && y.title_game.add(4) > y.declare_deadline {
        recruiting::declarations(w, recruiting::Declare::Deadline);
    }
    if d == y.title_game.add(4 + 3) && y.title_game.add(4) > y.declare_deadline {
        recruiting::declarations(w, recruiting::Declare::Withdraw);
    }
    if d == y.early_enrollment {
        if y.early_signing.is_none() {
            // Before the early signing period, early enrollees signed aid
            // agreements in January and enrolled at once.
            recruiting::signing_stage(w, recruiting::SignStage::JanuaryEnrollees, w.year);
        }
        recruiting::enroll(w, true);
    }
    // The January games name their rosters in the senior fall (part B 35).
    for (date, name) in
        [(Date::ymd(w.year, 10, 15), "All-American Bowl"), (Date::ymd(w.year, 11, 1), "Polynesian Bowl")]
    {
        if d == date {
            hs_year::all_star_rosters(w, name);
        }
    }
    for (date, name) in [(y.all_american_bowl, "All-American Bowl"), (y.polynesian_bowl, "Polynesian Bowl")] {
        if d == date {
            if crate::calendar::anchors::cancelled(name, y.season) {
                post(w, d, Level::HighSchool, Kind::Game, Loudness::Medium, &format!("The {name} is called off."));
            } else {
                hs_year::all_star_game(w, name);
            }
        }
    }
    // The college all-star games name their rosters together
    // (all_star_roster_day), the Senior Bowl first and the others from what
    // is left: the order of the game dates does not decide the tiers.
    if d == all_star_roster_day(&y) {
        for name in recruiting::ALL_STAR_ORDER {
            recruiting::all_star_invites(w, name);
        }
    }
    for (date, name) in all_star_games(&y) {
        if d == date {
            if crate::calendar::anchors::cancelled(name, y.season) {
                post(w, d, Level::College, Kind::Game, Loudness::Medium, &format!("The {name} is called off."));
            } else {
                recruiting::all_star_game(w, name);
            }
        }
    }
    // The college season ends with its last title game.
    if d >= y.title_game && w.college.as_ref().is_some_and(|c| c.started && !c.finished) && college::all_played(w) {
        timed("college finish", || college::finish(w));
    }
    if d == hs_carousel_day(&y) {
        timed("carousel hs", || staff::carousel(w, staff::Scope::HighSchool));
        staff::hs_staff_year(w);
        staff::age_coaches(w);
    }
    if d == y.signing_day && d <= y.super_bowl {
        timed("signing day", || signing_day_and_book(w, y.season));
    }
    // The week's rhythm, for the record and the future screens (T6), and
    // the training blocks of every level (T7).
    timed("rhythm", || rhythm(w, d, &y));
    nfl_year::training_blocks(w, d, &y);
    // Section 6's people (lane 6): ties, lives, courts, staffs and your agenda.
    timed("people", || super::folk::day(w, d));
    // The stories around the day's football (final build lane 8).
    timed("stories", || super::stories::day(w, d));
    // The storyline engine (update 2): stories that last.
    timed("storylines", || super::storylines::day(w, d));
    // The systems to chase (update 2, lane 2) and the world's halls.
    timed("legacy", || super::legacy::day(w, d));
    timed("scouting", || super::scouting::day(w, d));
    // The hunts (SK67): yours, the AI's, the generational prospect.
    timed("hunt", || super::hunt::day(w, d));
    // Update 2, lane 5: the call sheet drafted on plan day, the week's
    // keys, and the call words.
    timed("plans", || super::plans::day(w, d));
    // Section 6b: every team's training week, and the growth it earned.
    if d.weekday() == Weekday::Mon {
        timed("training", || crate::training::week::monday(w, d));
    }

    // News items of the day get the day's date.
    for n in w.news.iter_mut().rev() {
        if n.date.is_some() {
            break;
        }
        n.date = Some(d);
    }
    w.today = d.add(1);
    if w.today > y.super_bowl {
        rollover(w);
    }
}

fn transfer_rule_of(w: &World, season: i32) -> TransferRule {
    w.changes.rules(season).transfer
}

/// A new football year begins (the day after the Super Bowl).
fn year_start(w: &mut World) {
    let year = w.year;
    super::scouting::dept::prune(w);
    w.rules = w.changes.rules(year);
    // Your team's season as its schedule page showed it (update 2, lane 6),
    // kept before any conference moves.
    super::season_page::keep_seat_season(w);
    // Conference moves announced last year take effect.
    super::finance::college::switch(w);
    // The real moves of the season (college history, through 2026).
    super::program::apply_history(w);
    // Colleges start football and districts open schools (SK68).
    super::new_teams::year_start(w);
    w.college_prev = w.college.take();
    w.college = Some(college::new_season(w));
    if let Some(prev) = w.college_prev.as_mut() {
        w.prev_polls = std::mem::take(&mut prev.polls);
        college::keep_for_next_year(prev);
    }
    // The athletic directors fill their open dates for the seasons ahead.
    super::deals::top_up(w);
    // States realign for a two-year cycle (update 2, lane 6).
    if year == super::hs_year::cycle_start(year) || w.hs_season.is_none() {
        super::hs_playoffs::reclassify(w);
    }
    super::voters::year_start(w);
    w.hs_season = Some(hs_year::new_season(w));
    super::marks::hs_new_season(w);
    let nfl = nfl_year::new_season(w);
    super::units::book_nfl(w);
    w.nfl_prev = w.nfl_season.replace(nfl);
    super::franchise::year_start(w);
    let y = w.dates.clone();
    // The year's fixed dates on the calendar (T1.3).
    let sb = format!("Super Bowl ({}).", y.super_bowl.pretty());
    let draft = format!("The NFL draft opens {}.", y.draft[0].date.pretty());
    let signing = format!("National signing day, {}.", y.signing_day.pretty());
    for (date, level, kind, loud, text) in [
        (y.start, Level::Nfl, Kind::Date, Loudness::Small, "The football year begins.".to_string()),
        (y.draft[0].date, Level::Nfl, Kind::Date, Loudness::Huge, draft),
        (y.super_bowl, Level::Nfl, Kind::Game, Loudness::Huge, sb),
    ] {
        w.calendar.post(Event::new(date, level, kind, loud, text));
    }
    // A signing day after the Super Bowl falls in the next football year but
    // stays on the calendar and belongs to this class's season (part C 1).
    let mut e = Event::new(y.signing_day, Level::Recruiting, Kind::Date, Loudness::Big, signing);
    if y.signing_day > y.super_bowl {
        e.season = Some(y.season);
    }
    w.calendar.post(e);
    if let Some(esp) = y.early_signing {
        let text = format!("The early signing period opens {}.", esp.pretty());
        w.calendar
            .post(Event::new(esp, Level::Recruiting, Kind::Window, Loudness::Big, text).rule("NCAA: early signing"));
    }
    let _ = hs_transfer_day(&y);
    // The new schedules are out: your film room keeps your coming
    // opponents, last season's tapes of them included.
    w.prune_tapes();
}

/// Recorded years whose small events the game log keeps (part D 40).
const SMALL_EVENT_YEARS: i32 = 5;

/// The last day of a football year has passed: the next one starts.
fn rollover(w: &mut World) {
    // The world log takes the year before the calendar drops it.
    super::wlog::year_end(w);
    if w.options.keep_game_log {
        // News that belongs to last season (its signing day fell after the
        // Super Bowl) joins that season's record.
        // Small events too (part D 40: visits, 4-star news, weekly polls,
        // the Pro Bowl), for the last five recorded years only, to bound
        // the memory a 30-year world keeps (proposed, not confirmed).
        let mut kept: Vec<Event> = Vec::new();
        let small = w.recording;
        for e in w.calendar.all().filter(|e| (small || e.loud >= Loudness::Medium) && e.when.date < w.today) {
            let past = e.season.filter(|&s| s != w.year).and_then(|s| w.event_history.iter().rposition(|h| h.0 == s));
            match past {
                Some(k) => w.event_history[k].1.push(e.clone()),
                None => kept.push(e.clone()),
            }
        }
        w.event_history.push((w.year, kept));
        for (y, events) in w.event_history.iter_mut() {
            if *y <= w.year - SMALL_EVENT_YEARS {
                events.retain(|e| e.loud >= Loudness::Medium);
            }
        }
    }
    w.calendar.close_year(w.today);
    let next = w.year + 1;
    let log = std::mem::replace(&mut w.log, CalendarLog { year: next, ..CalendarLog::default() });
    w.log_history.push(log);
    let weeks = |y: i32| w.changes.rules(y).nfl_weeks as usize;
    let dates = YearDates::with_weeks(next, weeks(next), weeks(next - 1));
    w.prev_dates = Some(std::mem::replace(&mut w.dates, dates));
    print_profile(w.year);
    w.year = next;
}

/// February signing day, then the year-end book for `season`: program
/// prestige, grades and NIL, life events, and everyone who left is archived.
fn signing_day_and_book(w: &mut World, season: i32) {
    // Anyone still weighing his portal options decides now.
    recruiting::portal_day(w, true);
    recruiting::signing_stage(w, recruiting::SignStage::Final, season);
    recruiting::class_rankings_news(w, season + 1, "National signing day");
    let keep = w.year;
    w.year = season;
    let results = match (&w.college, &w.college_prev) {
        (Some(c), _) if c.year == season => c.results.clone(),
        (_, Some(c)) if c.year == season => c.results.clone(),
        _ => vec![Default::default(); w.programs.len()],
    };
    staff::update_programs(w, &results);
    w.life_events();
    super::people::year_book(w);
    super::fame::year_book(w);
    // The world log's yearly rows and exits, before leavers are dropped.
    super::wlog::book(w, season);
    w.compact();
    w.rebuild_rosters();
    w.year = keep;
}

/// College off-field changes from the agreed list (T11): rolled each summer
/// from 2027, with the state-by-state high school rules.
fn college_changes(w: &mut World) {
    let season = w.year;
    let mut rng = w.rng.fork();
    for (_, text) in w.changes.college_year(season, &mut rng) {
        w.log.rule_changes.push(text.clone());
        w.calendar.post(Event::new(w.today, Level::College, Kind::RuleChange, Loudness::Big, text));
    }
    if season >= 2027 {
        let odds = crate::calendar::rules::CHANGE_ODDS;
        for s in 0..w.states.len() {
            if !w.states[s].hs_nil && rng.chance(odds.hs_nil_per_state) {
                w.states[s].hs_nil = true;
                let text = format!("{} lets high school athletes earn NIL money.", w.states[s].code);
                w.log.rule_changes.push(text.clone());
                w.changes.applied.push(crate::calendar::rules::AppliedChange {
                    change: crate::calendar::Change::HsNil,
                    season,
                    detail: w.states[s].code.clone(),
                });
                w.calendar.post(Event::new(w.today, Level::HighSchool, Kind::RuleChange, Loudness::Small, text));
            }
            if rng.chance(odds.hs_transfer_per_state) {
                let loosen = rng.chance(0.5);
                let rule = &mut w.states[s].transfer;
                rule.sit_out = if loosen { (rule.sit_out - 0.5).max(0.0) } else { (rule.sit_out + 0.5).min(1.0) };
                let dir = if loosen { "loosens" } else { "tightens" };
                let text = format!("{} {dir} its high school transfer rule.", w.states[s].code);
                w.log.rule_changes.push(text.clone());
                w.changes.applied.push(crate::calendar::rules::AppliedChange {
                    change: crate::calendar::Change::HsTransferRules,
                    season,
                    detail: format!("{} {dir}", w.states[s].code),
                });
                w.calendar.post(Event::new(w.today, Level::HighSchool, Kind::RuleChange, Loudness::Small, text));
            }
        }
    }
}

fn post(w: &mut World, d: Date, level: Level, kind: Kind, loud: Loudness, text: &str) {
    w.calendar.post(Event::new(d, level, kind, loud, text));
}

/// The week's rhythm at each level (T6), posted as quiet steps for the record
/// and the future screens: the NFL week in season, the college game week, the
/// high school week. Each decision step is a hook the screens and staff
/// hand-offs (T10) will fill.
fn rhythm(w: &mut World, d: Date, y: &YearDates) {
    let in_nfl = d > y.nfl_kickoff.add(-4) && d <= *y.nfl_weeks.last().unwrap_or(&y.nfl_kickoff);
    if in_nfl {
        let step: Option<(u16, &str)> = match d.weekday() {
            Weekday::Mon => {
                Some((9, "NFL Monday: film of Sunday's game, injury exams, the head coach's press conference."))
            }
            Weekday::Tue => Some((16, "NFL Tuesday: players off; the game plan is built; waiver claims at 4pm ET.")),
            Weekday::Wed => Some((15, "NFL Wednesday: the plan is installed; first injury report.")),
            Weekday::Thu => Some((15, "NFL Thursday: third downs and the red zone.")),
            Weekday::Fri => Some((15, "NFL Friday: situations; final injury statuses.")),
            Weekday::Sat => Some((16, "NFL Saturday: walkthrough and travel; practice squad call-ups by 4pm ET.")),
            Weekday::Sun => Some((11, "NFL Sunday: inactives 90 minutes before kickoff.")),
        };
        if let Some((h, text)) = step {
            w.calendar.post(
                Event::new(d, Level::Nfl, Kind::Rhythm, Loudness::Small, text)
                    .at(Moment::at(d, h, 0))
                    .rule("CBA: in-season week"),
            );
        }
    }
    let in_college = d >= y.cfb_weeks[0].add(-5) && d <= y.fbs_regular_end;
    if in_college && d.weekday() == Weekday::Mon {
        w.calendar.post(
            Event::new(d, Level::College, Kind::Rhythm, Loudness::Small, "College game week: 20 countable hours, one day off; Tuesday to Thursday practice, Friday travel, Saturday games; staffs at high school games on Fridays.")
                .rule("NCAA: 20-hour rule"),
        );
    }
    let hs_fall = d >= y.hs_school_start && d <= y.thanksgiving;
    if hs_fall && d.weekday() == Weekday::Mon {
        w.calendar.post(Event::new(
            d,
            Level::HighSchool,
            Kind::Rhythm,
            Loudness::Small,
            "High school week: film Monday, practice Tuesday to Thursday, JV and freshman games Thursday, varsity Friday, film Saturday.",
        ));
    }
}

/// "On this day" (A42): title games, Heismans and first picks from five,
/// ten, fifteen... years ago, as news lines only (never a stakes factor).
fn anniversaries(w: &mut World, d: Date) {
    let (y, m, day) = d.parts();
    let found: Vec<(i32, Level, String, Vec<crate::calendar::Ref>)> = w
        .calendar
        .history
        .iter()
        .filter(|e| e.memorable)
        .filter_map(|e| {
            let (ey, em, ed) = e.when.date.parts();
            let ago = y - ey;
            (em == m && ed == day && ago > 0 && ago % 5 == 0).then(|| (ago, e.level, e.text.clone(), e.refs.clone()))
        })
        .collect();
    for (ago, level, text, refs) in found {
        let words = match ago {
            5 => "Five".to_string(),
            10 => "Ten".to_string(),
            15 => "Fifteen".to_string(),
            20 => "Twenty".to_string(),
            25 => "Twenty-five".to_string(),
            30 => "Thirty".to_string(),
            35 => "Thirty-five".to_string(),
            n => n.to_string(),
        };
        let line = format!("{words} years ago today: {text}");
        // The old line's links move with its words (L2).
        let shift = (line.len() - text.len()) as u16;
        let mut e = Event::new(d, level, Kind::Anniversary, Loudness::Small, line);
        e.refs = refs.into_iter().map(|r| crate::calendar::Ref { at: r.at + shift, ..r }).collect();
        w.calendar.post(e);
    }
}

/// T10: Continue skips quiet days and stops on the first day with an event
/// that needs a decision, a big moment that matters to the manager (his
/// team or his level) or a huge moment anywhere (T8: a big moment stops the
/// game and shows its own screen, such as free agency opening). Returns the
/// day it stopped on: a day played (today is the morning after), or the
/// morning of your game or a decision day, not yet played (today).
pub fn continue_until_needed(w: &mut World, max_days: u32) -> Date {
    // N58 (Cain): Big moments that matter to the manager, Huge ones anywhere.
    let seat = w.seat;
    let stops = |e: &Event| e.needs_decision || huge_for(seat, e) || (e.loud == Loudness::Big && cares(seat, e));
    for i in 0..max_days {
        let d = w.today;
        // The draft is live in your war room: nothing moves until it's done.
        if w.scouting.live.is_some() {
            return d;
        }
        // The morning of your game, or of a day you decide things on: stop
        // before it plays, so you get there first. Pressed that morning,
        // Continue plays it.
        if i > 0 && w.morning_stop(d).is_some() {
            return d;
        }
        let kept = w.seat_games.last().map(|g| (g.date, g.teams));
        advance_day(w);
        // Your own game day always stops, as a manager's match day does.
        if w.calendar.on(d).iter().any(stops) || w.seat_games.last().map(|g| (g.date, g.teams)) != kept {
            return d;
        }
    }
    w.today
}

fn cares(seat: Option<super::Seat>, e: &Event) -> bool {
    seat.is_none_or(|s| s.cares(e))
}

/// A huge moment anywhere (N58): the draft, cutdown day, the deadlines, the
/// playoffs and the title games. A regular-season game is huge only for its
/// own teams (topic 8), so another level's rivalry game or season-deciding
/// game is not one for you.
fn huge_for(seat: Option<super::Seat>, e: &Event) -> bool {
    e.loud == Loudness::Huge && (!e.regular || cares(seat, e))
}

/// T10: holiday mode to a chosen date. Staff run things; it always stops for
/// a huge moment. Returns the day it stopped on.
pub fn holiday_to(w: &mut World, until: Date) -> Date {
    let seat = w.seat;
    while w.today < until {
        let d = w.today;
        advance_day(w);
        if w.calendar.on(d).iter().any(|e| huge_for(seat, e)) {
            return d;
        }
    }
    w.today
}

/// The BCS read the Harris poll as its human poll from 2005 to 2013.
fn harris_year(w: &World) -> bool {
    w.rules.cfb_postseason == crate::calendar::CfbPostseason::Bcs && (2005..=2013).contains(&w.year)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T8, T10: Continue stops on a big moment too (the franchise tag
    /// deadline, long before the draft), and every day it skips is quiet.
    #[test]
    fn continue_stops_on_big_moments() {
        let options = super::super::WorldOptions {
            scale: 0.002,
            history_years: 0,
            names: super::super::names::NameMode::Fictional,
            ..Default::default()
        };
        let mut w = World::empty(options);
        let start = w.today;
        let stop = continue_until_needed(&mut w, 120);
        assert!(stop < w.dates.draft[0].date, "stopped on {}", stop.pretty());
        assert!(w.calendar.on(stop).iter().any(|e| e.loud == Loudness::Big), "stopped on {}", stop.pretty());
        let mut d = start;
        while d < stop {
            assert!(w.calendar.on(d).iter().all(|e| e.loud < Loudness::Big && !e.needs_decision));
            d = d.add(1);
        }
    }

    /// Continue stops the morning of each day an NFL seat acts on (Cain
    /// 2026-10-29: it played twelve days past the combine): the tag window,
    /// the combine, the tag deadline, the negotiating window, free agency,
    /// the offseason program and the draft.
    #[test]
    fn continue_stops_on_your_days() {
        let options = super::super::WorldOptions {
            scale: 0.002,
            history_years: 0,
            names: super::super::names::NameMode::Fictional,
            ..Default::default()
        };
        let mut w = World::empty(options);
        w.seat = Some(super::super::Seat { level: Level::Nfl, nfl_team: Some(3), program: None, other: None });
        let y = w.dates.clone();
        let mut mornings = Vec::new();
        while w.today <= y.draft[0].date {
            // The war room would wait for you: the staff finish the draft.
            if w.scouting.live.is_some() {
                super::super::scouting::live::finish(&mut w);
            }
            let from = w.today;
            let stop = continue_until_needed(&mut w, 400);
            if stop == w.today && stop > from {
                mornings.push(stop);
            }
        }
        let program = crate::training::rules::Frame::pro(&y, false).phase1;
        let mut want = vec![y.tag_window_open, y.combine_start, y.tag_deadline.date, y.league_year.date, program];
        want.extend(y.tampering_open.map(|m| m.date));
        want.push(y.draft[0].date);
        for d in want {
            assert!(mornings.contains(&d), "Continue played through {} without stopping that morning", d.pretty());
        }
    }

    /// Each seat's days, read without playing them: the college and high
    /// school calendars' training blocks and signing days.
    #[test]
    fn each_seat_has_its_days() {
        let options = super::super::WorldOptions {
            scale: 0.002,
            history_years: 0,
            names: super::super::names::NameMode::Fictional,
            ..Default::default()
        };
        let mut w = World::empty(options);
        let y = w.dates.clone();
        let fbs = w.programs.iter().position(|p| p.division == super::super::data::Division::FBS).unwrap();
        w.seat = Some(super::super::Seat { level: Level::College, nfl_team: None, program: Some(fbs), other: None });
        // Signing day can fall after the Super Bowl.
        let end = y.super_bowl.max(y.signing_day);
        let days = w.your_days(y.start, end);
        let names: Vec<&str> = days.iter().map(|x| x.1.as_str()).collect();
        for name in ["Spring practice", "Summer program", "Fall camp", "The portal opens", "National signing day"] {
            assert!(names.contains(&name), "college: no {name} in {names:?}");
        }
        assert!(days.iter().any(|x| x.0 == y.fall_camp && x.1 == "Fall camp"), "college: fall camp not on its day");
        let h = w.high_schools.iter().position(|h| h.state < w.states.len()).unwrap();
        w.seat = Some(super::super::Seat { level: Level::HighSchool, nfl_team: None, program: Some(h), other: None });
        let names: Vec<String> = w.your_days(y.start, end).into_iter().map(|x| x.1).collect();
        for name in ["Summer workouts", "Preseason practice"] {
            assert!(names.iter().any(|n| n == name), "high school: no {name} in {names:?}");
        }
    }

    /// N58: a manager's Continue passes Big moments at other levels, but a
    /// college moment about his own program still matters to him.
    #[test]
    fn a_seat_cares_about_its_level_and_team() {
        let seat = super::super::Seat { level: Level::Nfl, nfl_team: Some(3), program: None, other: None };
        let d = Date::ymd(2026, 9, 5);
        assert!(!seat.cares(&Event::new(d, Level::College, Kind::Game, Loudness::Big, "A college game.")));
        assert!(seat.cares(&Event::new(d, Level::Nfl, Kind::Game, Loudness::Big, "An NFL game.")));
        let coach = super::super::Seat { level: Level::College, nfl_team: None, program: Some(7), other: None };
        let nfl = Event::new(d, Level::Nfl, Kind::Transfer, Loudness::Big, "His player is drafted.").program(7);
        assert!(coach.cares(&nfl));
    }

    /// F4: the news of a signing day that falls after the Super Bowl shows
    /// on its day, in the new football year, but is filed under the class's
    /// season, like the day's counters.
    #[test]
    fn late_signing_news_is_filed_under_its_season() {
        let options = super::super::WorldOptions {
            scale: 0.002,
            history_years: 0,
            names: super::super::names::NameMode::Fictional,
            keep_game_log: true,
            ..Default::default()
        };
        let mut w = World::empty(options);
        let season = w.year;
        let news = |d: Date, text: &str| Event::new(d, Level::Recruiting, Kind::Signing, Loudness::Medium, text);
        w.calendar.post(news(w.today, "a summer commitment"));
        // The year turns over the day after its last day: news dated today
        // or later belongs to the new year.
        w.today = w.today.add(1);
        rollover(&mut w);
        let day = w.prev_dates.as_ref().unwrap().signing_day;
        w.calendar.filing = Some(season);
        w.calendar.post(news(day, "a February signing"));
        w.calendar.filing = None;
        w.calendar.post(news(day, "next year's news"));
        assert_eq!(w.calendar.on(day).len(), 2, "both show on the day");
        w.today = day.add(1);
        rollover(&mut w);
        let texts = |k: usize| w.event_history[k].1.iter().map(|e| e.text.as_str()).collect::<Vec<_>>();
        assert_eq!(w.event_history[0].0, season);
        assert_eq!(texts(0), ["a summer commitment", "a February signing"]);
        assert_eq!(w.event_history[1].0, season + 1);
        assert_eq!(texts(1), ["next year's news"]);
    }
}
