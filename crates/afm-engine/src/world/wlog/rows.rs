//! The yearly rows: people, coaches, programs and schools at the year-end
//! book (before leavers are dropped), the exits, and at the year's close the
//! events, news, clubs and the world's own row.

use rayon::prelude::*;
use serde::Serialize;

use super::why::Alumni;
use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::world::{Person, Stage, World};

/// A person in football at the book (see `Lean`), with the reads the
/// screens never show: his true overall and potential.
#[derive(Serialize)]
struct PlayerRow<'a> {
    season: i32,
    date: Date,
    id: u32,
    level: Option<crate::player::Level>,
    overall: f64,
    potential: f64,
    person: Lean<'a>,
}

/// What never changes about a person, written once, at the first book he
/// is in (`people`): his name, heritage, birth, home, first school and
/// class, family, the team he grew up cheering for and his face.
#[derive(Serialize)]
struct Static<'a> {
    season: i32,
    id: u32,
    name: &'a str,
    ethnicity: &'a crate::world::ethnicity::Ethnicity,
    born: Date,
    home_state: usize,
    hs: usize,
    class_of: i32,
    family: &'a crate::world::background::Family,
    childhood_team: Option<usize>,
    /// Talented enough to be recruited as a regular: the camps never make
    /// him a snapper (`Person::regular_talent`).
    regular_talent: bool,
    looks: &'a crate::world::looks::Looks,
}

impl<'a> Static<'a> {
    fn of(season: i32, p: &'a Person) -> Static<'a> {
        Static {
            season,
            id: p.player.id,
            name: &p.name,
            ethnicity: &p.ethnicity,
            born: p.born,
            home_state: p.home_state,
            hs: p.hs,
            class_of: p.class_of,
            family: &p.family,
            childhood_team: p.childhood_team,
            regular_talent: p.regular_talent,
            looks: &p.looks,
        }
    }
}

/// A person as he stands, without what `Static` holds and without his
/// random stream. Ratings, ceilings and tendencies are kept in tenths
/// (`_x10`: 734 is 73.4). The lists that only grow are cut to the year (his
/// latest season row, high school line, college staff and pro season: the
/// years before are in his earlier rows) and his notes are left for his
/// exit, which keeps them all.
#[derive(Serialize)]
struct Lean<'a> {
    stage: Stage,
    hs: usize,
    college: Option<usize>,
    class_of: i32,
    followers: u32,
    brand: f32,
    stars: u8,
    camp_stars: u8,
    visibility: f64,
    college_seasons: u32,
    redshirt_used: bool,
    sit_out: f64,
    steered: bool,
    nil_income: f64,
    pending_dip: Option<f64>,
    last_role: Option<PlayingTime>,
    hs_lines: &'a [crate::world::hs_year::HsLine],
    last_snaps: u32,
    college_snaps: u32,
    last_performance: Option<f64>,
    transfers: u32,
    juco: bool,
    path: &'a [usize],
    on_notice: bool,
    gpa_trend: f64,
    gpa_at_signing: Option<f64>,
    academic_redshirt: bool,
    walk_on: bool,
    transfer_from: Option<PlayingTime>,
    college_staff: &'a [(i32, usize, Option<usize>, Option<usize>)],
    incident: bool,
    notes: &'a [String],
    stay_home: f64,
    bg: &'a crate::world::background::Background,
    cal: &'a crate::world::PersonCal,
    pro: Option<std::borrow::Cow<'a, crate::world::pro::ProCareer>>,
    mood: f32,
    mood_why: u8,
    trust: f32,
    trust_in: u32,
    roads: &'a crate::world::eligibility::Roads,
    player: LeanPlayer<'a>,
}

#[derive(Serialize)]
struct LeanPlayer<'a> {
    position: crate::positions::Position,
    age: u32,
    attributes_x10: Vec<i16>,
    ceilings_x10: Vec<i16>,
    original_potential: f64,
    bloom: f64,
    longevity: f64,
    injury_proneness: f64,
    wear: f64,
    grow: f32,
    personality: &'a crate::player::Personality,
    body: &'a crate::body::Body,
    position_history: &'a [(u32, crate::positions::Position)],
    spot: &'a crate::spots::Spot,
    spots_played: u32,
    history: &'a [crate::player::SeasonRecord],
    game: &'a crate::player::GameTraits,
    tend: Tend,
    train: &'a crate::training::Train,
}

/// Tendencies in tenths (`tendencies::Tendencies`).
#[derive(Serialize)]
struct Tend {
    v_x10: Vec<i16>,
    was_x10: Vec<i16>,
    contact_dip: f32,
}

fn tenths(v: impl Iterator<Item = f64>) -> Vec<i16> {
    v.map(|x| (x * 10.0).round() as i16).collect()
}

/// The last item of a list (or none), for a yearly row.
fn last<T>(v: &[T]) -> &[T] {
    &v[v.len().saturating_sub(1)..]
}

impl<'a> Lean<'a> {
    /// The person at the book; an exit keeps his notes.
    fn of(p: &'a Person, exit: bool) -> Lean<'a> {
        let q = &p.player;
        let pro = p.pro.as_deref().map(|c| {
            if c.seasons.len() > 1 || c.other.len() > 1 {
                let mut c = c.clone();
                c.seasons.drain(..c.seasons.len().saturating_sub(1));
                c.other.drain(..c.other.len().saturating_sub(1));
                std::borrow::Cow::Owned(c)
            } else {
                std::borrow::Cow::Borrowed(c)
            }
        });
        Lean {
            stage: p.stage,
            hs: p.hs,
            college: p.college,
            class_of: p.class_of,
            followers: p.followers,
            brand: p.brand,
            stars: p.stars,
            camp_stars: p.camp_stars,
            visibility: p.visibility,
            college_seasons: p.college_seasons,
            redshirt_used: p.redshirt_used,
            sit_out: p.sit_out,
            steered: p.steered,
            nil_income: p.nil_income,
            pending_dip: p.pending_dip,
            last_role: p.last_role,
            hs_lines: last(&p.hs_lines),
            last_snaps: p.last_snaps,
            college_snaps: p.college_snaps,
            last_performance: p.last_performance,
            transfers: p.transfers,
            juco: p.juco,
            path: &p.path,
            on_notice: p.on_notice,
            gpa_trend: p.gpa_trend,
            gpa_at_signing: p.gpa_at_signing,
            academic_redshirt: p.academic_redshirt,
            walk_on: p.walk_on,
            transfer_from: p.transfer_from,
            college_staff: last(&p.college_staff),
            incident: p.incident,
            notes: if exit { &p.notes } else { &[] },
            stay_home: p.stay_home,
            bg: &p.bg,
            cal: &p.cal,
            pro,
            mood: p.mood,
            mood_why: p.mood_why,
            trust: p.trust,
            trust_in: p.trust_in,
            roads: &p.roads,
            player: LeanPlayer {
                position: q.position,
                age: q.age,
                attributes_x10: tenths(q.attributes.values()),
                ceilings_x10: tenths(q.ceilings.values()),
                original_potential: q.original_potential,
                bloom: q.bloom,
                longevity: q.longevity,
                injury_proneness: q.injury_proneness,
                wear: q.wear,
                grow: q.grow,
                personality: &q.personality,
                body: &q.body,
                position_history: &q.position_history,
                spot: &q.spot,
                spots_played: q.spots_played,
                history: last(&q.history),
                game: &q.game,
                tend: Tend {
                    v_x10: tenths(q.tend.v.iter().map(|&x| x as f64)),
                    was_x10: tenths(q.tend.was.iter().map(|&x| x as f64)),
                    contact_dip: q.tend.contact_dip,
                },
                train: &q.train,
            },
        }
    }
}

#[derive(Serialize)]
struct Exit<'a> {
    season: i32,
    date: Date,
    id: u32,
    name: &'a str,
    stage: Stage,
    why: &'static str,
    /// The note the world wrote when he left, if any.
    said: Option<&'a str>,
    class_of: i32,
    stars: u8,
    overall: f64,
    potential: f64,
    original_potential: f64,
    /// Kept as an alumnus (a pro, or a 4- or 5-star), so the world file has him too.
    kept: bool,
    person: Lean<'a>,
}

#[derive(Serialize)]
struct CoachRow<'a> {
    season: i32,
    date: Date,
    id: usize,
    coach: &'a crate::world::coach::Coach,
}

#[derive(Serialize)]
struct ProgramRow<'a> {
    season: i32,
    date: Date,
    id: usize,
    result: Option<&'a crate::world::season::TeamResult>,
    program: &'a crate::world::program::Program,
}

#[derive(Serialize)]
struct SchoolRow<'a> {
    season: i32,
    date: Date,
    id: usize,
    result: Option<&'a crate::world::season::TeamResult>,
    school: &'a crate::world::high_school::HighSchool,
}

#[derive(Serialize)]
struct ClubRow<'a> {
    season: i32,
    date: Date,
    team: usize,
    name: String,
    record: Option<&'a crate::world::nfl_year::NflTeamSeason>,
    league: Option<&'a crate::league::Team>,
    club: &'a crate::world::pro::Club,
}

#[derive(Serialize)]
struct EventRow<'a> {
    season: i32,
    event: &'a crate::calendar::Event,
}

#[derive(Serialize)]
struct Resolved<'a> {
    season: i32,
    at: crate::calendar::Moment,
    pending: &'a crate::calendar::events::Pending,
    outcome: &'a crate::calendar::events::Outcome,
}

#[derive(Serialize)]
struct NewsRow<'a> {
    season: i32,
    index: usize,
    news: &'a crate::world::news::NewsItem,
}

/// The world's own row for a year.
#[derive(Serialize)]
struct WorldRow<'a> {
    season: i32,
    date: Date,
    people: usize,
    alumni: usize,
    coaches: usize,
    counters: &'a crate::world::CalendarLog,
    rules: &'a crate::calendar::RuleBook,
    changes: Vec<&'a crate::calendar::rules::AppliedChange>,
    signing_class: Option<&'a (i32, [u32; 6])>,
    nfl_champion: Option<&'a (i32, usize)>,
    college_spread: f64,
    draft_bars: Option<(f64, f64)>,
    system_norms: &'a crate::world::systems::SystemNorms,
    novelty: Vec<Vec<f64>>,
    trends: Vec<&'a crate::world::trends::Trend>,
    level_scales: &'a [crate::level_rating::ScaleHistory],
    fbs_signees_by_state: Vec<u32>,
    /// A fingerprint of each store at the year's close: two runs that
    /// match have the same; the first year they differ is where they split.
    fingerprints: Vec<(&'static str, String)>,
}

fn last_only<T>(v: &mut Vec<T>) {
    if v.len() > 1 {
        v.drain(..v.len() - 1);
    }
}

/// The year-end book (`clock::signing_day_and_book`, before `compact`).
pub(super) fn book(w: &World, season: i32) {
    let Some(log) = w.wlog.as_deref() else { return };
    let date = w.today;
    // Who is new since the last book: what never changes about him, once.
    let from = log.marks.lock().unwrap().people;
    for p in w.people.iter().filter(|p| p.player.id >= from) {
        log.put("people", &Static::of(season, p));
    }
    // The rows, on all cores (readers key them by id, not order).
    w.people.par_iter().filter(|p| in_football(p.stage)).for_each(|p| {
        log.put(
            "players",
            &PlayerRow {
                season,
                date,
                id: p.player.id,
                level: level_of(w, p),
                overall: p.player.overall(),
                potential: p.player.potential(),
                person: Lean::of(p, false),
            },
        );
    });
    // The exits: what `compact` drops (or keeps as an alumnus), whole.
    w.people.par_iter().filter(|p| !in_football(p.stage)).for_each(|p| {
        let (why, said) = exit_why(p);
        log.put(
            "exits",
            &Exit {
                season,
                date,
                id: p.player.id,
                name: &p.name,
                stage: p.stage,
                why,
                said,
                class_of: p.class_of,
                stars: p.stars,
                overall: p.player.overall(),
                potential: p.player.potential(),
                original_potential: p.player.original_potential,
                kept: p.pro.is_some() || p.stars >= 4,
                person: Lean::of(p, true),
            },
        );
    });
    log.set_people_mark(w.next_person);
    for c in &w.coaches {
        log.put("coaches", &CoachRow { season, date, id: c.id, coach: c });
    }
    let results = match (&w.college, &w.college_prev) {
        (Some(c), _) if c.year == season => Some(&c.results),
        (_, Some(c)) if c.year == season => Some(&c.results),
        _ => None,
    };
    for (i, p) in w.programs.iter().enumerate() {
        log.put("programs", &ProgramRow { season, date, id: i, result: results.and_then(|r| r.get(i)), program: p });
    }
    let hs = w.hs_season.as_ref().map(|s| &s.results);
    for (i, s) in w.high_schools.iter().enumerate() {
        let mut school = s.clone();
        last_only(&mut school.seasons);
        log.put("schools", &SchoolRow { season, date, id: i, result: hs.and_then(|r| r.get(i)), school: &school });
    }
    super::why::alumni_since(w, log, Alumni::Book);
}

/// Whether a stage is in football (a row) or has left it (an exit).
fn in_football(s: Stage) -> bool {
    matches!(s, Stage::HighSchool | Stage::College | Stage::PrepSchool | Stage::Declared | Stage::Pro | Stage::Away)
}

fn level_of(w: &World, p: &Person) -> Option<crate::player::Level> {
    use crate::player::Level;
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => Some(Level::HS),
        Stage::College => p.college.and_then(|c| w.programs.get(c)).map(|g| crate::world::division_level(g.division)),
        Stage::Pro => Some(Level::NFL),
        _ => None,
    }
}

/// Why someone left football: read from the note the world wrote when he
/// did (each place that ends a career writes one), else from where he stood.
fn exit_why(p: &Person) -> (&'static str, Option<&str>) {
    const SAID: &[(&str, &str)] = &[
        ("Walked away from football", "walked away"),
        ("Retired at", "retired"),
        ("tried out at rookie minicamp", "undrafted, unsigned after a tryout"),
        ("Went undrafted and unsigned", "undrafted and unsigned"),
        ("junior college years ran out", "JUCO years ran out"),
        ("Never earned a move up from JUCO", "no move up from JUCO"),
        ("prep school year didn't bring an offer", "no offer after prep school"),
        ("no program would take him", "a serious incident"),
        ("found no school and left football", "portal: no new school"),
        ("never signed", "never signed"),
        ("fell through", "commitment fell through"),
    ];
    let last = p.notes.last().map(String::as_str);
    if let Some(n) = last {
        for (key, why) in SAID {
            if n.contains(key) {
                return (why, last);
            }
        }
    }
    let why = match () {
        _ if p.pro.is_some() => "pro career over",
        _ if p.roads.away.is_some() => "did not come back from time away",
        _ if p.college.is_none() && p.cal.signed.is_none() => "finished high school without a college team",
        _ if p.college.is_some() => "college career over",
        _ => "left football",
    };
    (why, last)
}

/// The football year closes (`clock::rollover`, before the calendar drops
/// the year's events).
pub(super) fn year_end(w: &World) {
    let Some(log) = w.wlog.as_deref() else { return };
    let season = w.year;
    let date = w.today;
    for e in w.calendar.all().filter(|e| e.when.date < w.today) {
        log.put("events", &EventRow { season: e.season.unwrap_or(season), event: e });
    }
    for (pending, at, outcome) in w.calendar.resolved.iter().filter(|r| r.1.date >= w.dates.start) {
        log.put("events", &Resolved { season, at: *at, pending, outcome });
    }
    {
        let mut m = log.marks.lock().unwrap();
        for (k, n) in w.news.iter().enumerate().skip(m.news) {
            log.put("news", &NewsRow { season, index: k, news: n });
        }
        m.news = w.news.len();
    }
    for (t, club) in w.clubs.iter().enumerate() {
        log.put(
            "clubs",
            &ClubRow {
                season,
                date,
                team: t,
                name: w.nfl_team_name(t),
                record: w.nfl_seasons.iter().rev().find(|s| s.year == season && s.team == t),
                league: w.nfl.get(t),
                club,
            },
        );
    }
    super::snap::year_end(w, log);
    super::why::alumni_since(w, log, Alumni::YearEnd);
    super::why::recruiting_since(w, log);
    log.put(
        "world",
        &WorldRow {
            season,
            date,
            people: w.people.len(),
            alumni: w.alumni.len(),
            coaches: w.coaches.len(),
            counters: &w.log,
            rules: &w.rules,
            changes: w.changes.applied.iter().filter(|c| c.season == season).collect(),
            signing_class: w.signing_classes.iter().rev().find(|c| c.0 == season + 1 || c.0 == season),
            nfl_champion: w.nfl_champions.iter().rev().find(|c| c.0 == season),
            college_spread: w.college_spread,
            draft_bars: w.draft_bars,
            system_norms: &w.system_norms,
            novelty: w.novelty.iter().map(|r| r.to_vec()).collect(),
            trends: w.trends.iter().filter(|t| t.year >= season - 1).collect(),
            level_scales: &w.level_scales,
            fbs_signees_by_state: w.fbs_signees_by_state.to_vec(),
            fingerprints: super::why::fingerprints(w),
        },
    );
}

/// The fields of a person (and his player) that neither `Static` nor `Lean`
/// writes, by name: a field added to `Person` or `Player` must be added to
/// one of them (`tests/world_log.rs`).
#[cfg(test)]
pub(crate) fn missing_fields(p: &Person) -> Vec<String> {
    use serde_json::Value;
    fn keys<T: Serialize>(v: &T) -> Value {
        let bytes = rmp_serde::to_vec_named(v).unwrap_or_default();
        rmp_serde::from_slice::<super::read::Any>(&bytes).map(|a| a.0).unwrap_or(Value::Null)
    }
    let person = keys(p);
    let lean = keys(&Lean::of(p, true));
    let fixed = keys(&Static::of(0, p));
    let mut out = Vec::new();
    for k in person.as_object().into_iter().flat_map(|m| m.keys()) {
        if k != "rng" && k != "player" && lean.get(k).is_none() && fixed.get(k).is_none() {
            out.push(k.clone());
        }
    }
    let renamed = |k: &str| match k {
        "attributes" => "attributes_x10".to_string(),
        "ceilings" => "ceilings_x10".to_string(),
        _ => k.to_string(),
    };
    for k in person["player"].as_object().into_iter().flat_map(|m| m.keys()) {
        if k != "id" && lean["player"].get(renamed(k)).is_none() {
            out.push(format!("player.{k}"));
        }
    }
    out
}
