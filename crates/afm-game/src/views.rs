//! The screens' data (section 8, topic 5): each function builds one screen
//! as JSON from the world, showing only what your staff could know.

use afm_engine::attributes::Category;
use afm_engine::calendar::{Date, Level as CalLevel, Link, Loudness, Ref};
use afm_engine::player::{Level, PlayingTime};
use afm_engine::positions::Position;
use afm_engine::world::coach::{Employer, Group, Role};
use afm_engine::world::{Person, Stage, World};
use serde_json::{Value, json};

use crate::colors;
use crate::reads;

/// A team the screens are about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamRef {
    Nfl(usize),
    College(usize),
    HighSchool(usize),
}

impl TeamRef {
    /// From `{kind, id}`, or your own team when none is given.
    pub fn from_args(w: &World, args: &Value) -> Result<TeamRef, String> {
        let id = args.get("id").and_then(Value::as_u64).map(|v| v as usize);
        match (args.get("kind").and_then(Value::as_str), id) {
            (Some("nfl"), Some(i)) if i < w.nfl.len() => Ok(TeamRef::Nfl(i)),
            (Some("college"), Some(i)) if i < w.programs.len() => Ok(TeamRef::College(i)),
            (Some("hs"), Some(i)) if i < w.high_schools.len() => Ok(TeamRef::HighSchool(i)),
            (Some(k), _) => Err(format!("No such team: {k} {id:?}")),
            (None, _) => mine(w).ok_or_else(|| "You have no team yet.".to_string()),
        }
    }

    pub fn json(self) -> Value {
        match self {
            TeamRef::Nfl(i) => json!({ "kind": "nfl", "id": i }),
            TeamRef::College(i) => json!({ "kind": "college", "id": i }),
            TeamRef::HighSchool(i) => json!({ "kind": "hs", "id": i }),
        }
    }
}

/// Your team.
pub fn mine(w: &World) -> Option<TeamRef> {
    let s = w.seat?;
    Some(match s.level {
        CalLevel::Nfl => TeamRef::Nfl(s.nfl_team?),
        CalLevel::HighSchool => TeamRef::HighSchool(s.program?),
        _ => TeamRef::College(s.program?),
    })
}

pub fn date_text(d: Date) -> String {
    let (y, m, day) = d.parts();
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {} {} {}", d.weekday().short(), day, M[(m - 1) as usize], y)
}

pub fn team_name(w: &World, t: TeamRef) -> String {
    match t {
        TeamRef::Nfl(i) => w.nfl_team_name(i),
        TeamRef::College(i) => w.program_name(i).to_string(),
        TeamRef::HighSchool(i) => w.school_name(i).to_string(),
    }
}

pub fn team_colors(w: &World, t: TeamRef) -> [String; 2] {
    match t {
        TeamRef::Nfl(i) => colors::nfl(&w.nfl_team_name(i), i),
        TeamRef::College(i) => colors::college(&w.programs[i].real_name, i),
        TeamRef::HighSchool(i) => colors::high_school(i),
    }
}

pub(crate) fn team_brief(w: &World, t: TeamRef) -> Value {
    let [c1, c2] = team_colors(w, t);
    json!({ "ref": t.json(), "name": team_name(w, t), "colors": [c1, c2], "mine": is_mine(w, t) })
}

/// The screens' team for an employer: none for a club in another league.
pub(crate) fn employer_ref(e: Employer) -> Option<TeamRef> {
    match e {
        Employer::HighSchool(s) => Some(TeamRef::HighSchool(s)),
        Employer::College(p) => Some(TeamRef::College(p)),
        Employer::Nfl(t) => Some(TeamRef::Nfl(t)),
        Employer::Other(_) => None,
    }
}

/// A club in another league, as a team link: its name this year and
/// colours. The team screen opens its league page.
pub(crate) fn other_brief(w: &World, fr: usize) -> Value {
    let name = afm_engine::world::pro::other::name(fr, w.year);
    json!({ "ref": { "kind": "other", "id": fr }, "name": name, "colors": colors::other(name, fr), "mine": false })
}

/// Any employer as a team link.
pub(crate) fn employer_brief(w: &World, e: Employer) -> Value {
    match (employer_ref(e), e) {
        (Some(t), _) => team_brief(w, t),
        (None, Employer::Other(f)) => other_brief(w, f),
        (None, _) => Value::Null,
    }
}

/// An employer's name: the program, the club or the school.
pub(crate) fn employer_name(w: &World, e: Employer) -> String {
    match e {
        Employer::College(p) => w.program_name(p).to_string(),
        Employer::Nfl(t) => w.nfl_team_name(t),
        Employer::HighSchool(h) => w.school_name(h).to_string(),
        Employer::Other(f) => afm_engine::world::pro::other::name(f, w.year).to_string(),
    }
}

/// An employer's line under its name (conference, division, state, league).
pub(crate) fn employer_line(w: &World, e: Employer) -> String {
    match (employer_ref(e), e) {
        (Some(t), _) => team_line(w, t),
        (None, Employer::Other(f)) => {
            let k = afm_engine::world::pro::other::league_of(f);
            afm_engine::world::pro::other::league_name(k, w.year).to_string()
        }
        (None, _) => String::new(),
    }
}

/// A team's record and which season it is: this season's once it has
/// played, before that last season's ("9-8 in 2025"). On the new football
/// year's first day, before it plays, the season held is still last year's.
pub(crate) fn season_record(w: &World, t: TeamRef) -> (String, String) {
    let now = "this season".to_string();
    match t {
        TeamRef::Nfl(i) => {
            let (wi, l, ti) = w.nfl_record(i);
            let text = |wi: u32, l: u32, ti: u32| if ti > 0 { format!("{wi}-{l}-{ti}") } else { format!("{wi}-{l}") };
            match w.nfl_season_year() {
                Some(y) if y < w.year => (text(wi, l, ti), format!("in {y}")),
                _ if wi + l + ti == 0 => match w.nfl_seasons.iter().rev().find(|s| s.team == i) {
                    Some(s) => (text(s.wins, s.losses, s.ties), format!("in {}", s.year)),
                    None => (text(0, 0, 0), now),
                },
                _ => (text(wi, l, ti), now),
            }
        }
        TeamRef::College(i) => {
            let r = w.college_record(i);
            let text = format!("{}-{} ({}-{} conf)", r.wins, r.losses, r.conf_wins, r.conf_losses);
            match w.college_year() {
                Some(y) if y < w.year => (text, format!("in {y}")),
                _ if r.wins + r.losses == 0 => match w.programs[i].seasons.last() {
                    Some(s) => (format!("{}-{}", s.wins, s.losses), format!("in {}", s.year)),
                    None => (text, now),
                },
                _ => (text, now),
            }
        }
        TeamRef::HighSchool(i) => match w.hs_record(i) {
            // A high school season ends in December: in January and February
            // the record is last fall's.
            Some((wn, l)) if wn + l > 0 => {
                (format!("{wn}-{l}"), if w.today.year() > w.year { format!("in {}", w.year) } else { now })
            }
            _ => match w.high_schools[i].seasons.last() {
                Some(h) => (format!("{}-{}", h.wins, h.losses), format!("in {}", h.year)),
                None => ("0-0".to_string(), now),
            },
        },
    }
}

/// A high school's class in its state's own words, from the state
/// association's 2026 classes (`hs_states.csv`, biggest first): Texas's
/// twelve are 6A Division I down to 1A Division II (TAPPS for a private
/// school), Ohio plays Divisions I-VII, California Open and 1-AA to 7-A,
/// Nebraska A to D-6, the 8-man and 6-man classes are the smallest. Where
/// a state splits public and private schools (Alabama, Georgia, Louisiana,
/// New Jersey, Tennessee) a private school takes its own class's name.
/// The rest number their classes with A's, the biggest highest.
pub(crate) fn hs_class(w: &World, s: usize) -> String {
    let h = &w.high_schools[s];
    let st = &w.states[h.state];
    let n = st.schools.iter().map(|&k| w.high_schools[k].classification).max().unwrap_or(0) as usize + 1;
    let k = (h.classification as usize).min(n - 1);
    // Where the school's class falls in a list of `len` names, biggest first.
    let at = |len: usize| (k * len / n).min(len - 1);
    let pick = |names: &[&str]| names[at(names.len())].to_string();
    const ROMAN: [&str; 8] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII"];
    let a = |len: usize| format!("{}A", len - at(len));
    match st.code.as_str() {
        "AK" | "VT" => format!("Division {}", ROMAN[at(3)]),
        "AL" if h.private => pick(&["Private AA", "Private A"]),
        "AL" | "FL" => a(6),
        "AZ" => pick(&["Open", "6A", "5A", "4A", "3A", "2A", "1A", "8-player"]),
        "AR" => pick(&["7A", "6A", "5A", "4A", "3A", "2A", "8-man"]),
        "CA" => pick(&[
            "Open Division",
            "Division 1-AA",
            "Division 1-A",
            "Division 2-AA",
            "Division 2-A",
            "Division 3-AA",
            "Division 3-A",
            "Division 4-AA",
            "Division 4-A",
            "Division 5-AA",
            "Division 5-A",
            "Division 6-AA",
            "Division 6-A",
            "Division 7-AA",
            "Division 7-A",
        ]),
        "CO" => pick(&["5A", "4A", "3A", "2A", "1A", "8-man", "6-man"]),
        "CT" => pick(&["Class LL", "Class L", "Class MM", "Class M", "Class SS", "Class S"]),
        "GA" if h.private => "A Private".into(),
        "GA" => pick(&["7A", "6A", "5A", "4A", "3A", "2A", "A"]),
        "HI" => pick(&["Open", "Division I", "Division II"]),
        "IA" => pick(&["5A", "4A", "3A", "2A", "1A", "A", "8-player"]),
        "KS" => pick(&["6A", "5A", "4A", "3A", "2A", "1A", "8-player Division I", "8-player Division II", "6-player"]),
        "LA" => format!("{} Division {}", if h.private { "Select" } else { "Non-Select" }, ROMAN[at(4)]),
        "ME" => pick(&["Class A", "Class B", "Class C", "Class D", "8-man Large", "8-man Small"]),
        "MD" => pick(&["4A", "4A/3A", "3A", "2A", "2A/1A", "1A"]),
        "NH" => format!("Division {}", ROMAN[at(4)]),
        "MA" => format!("Division {}", at(8) + 1),
        "RI" => format!("Division {}", at(4) + 1),
        "MI" => pick(&[
            "Division 1",
            "Division 2",
            "Division 3",
            "Division 4",
            "Division 5",
            "Division 6",
            "Division 7",
            "Division 8",
            "8-player Division 1",
            "8-player Division 2",
        ]),
        "MN" => pick(&["6A", "5A", "4A", "3A", "2A", "1A", "9-player"]),
        "MO" => pick(&["Class 6", "Class 5", "Class 4", "Class 3", "Class 2", "Class 1", "8-man"]),
        "MT" => pick(&["Class AA", "Class A", "Class B", "Class C 8-player", "Class C 6-player"]),
        "NE" => pick(&["Class A", "Class B", "Class C-1", "Class C-2", "Class D-1", "Class D-2", "Class D-6"]),
        "NV" => pick(&["Open", "5A", "4A", "3A", "2A", "1A"]),
        "NJ" if h.private => pick(&["Non-Public A", "Non-Public B"]),
        "NJ" => format!("Group {}", 5 - at(5)),
        "NM" => pick(&["6A", "5A", "4A", "3A", "2A", "8-man", "6-man"]),
        "NY" => pick(&["Class AA", "Class A", "Class B", "Class C", "Class D"]),
        "ND" => pick(&["Class AAA", "Class AA", "Class A", "9-man"]),
        "OH" => format!("Division {}", ROMAN[at(7)]),
        "OK" => pick(&["6A-I", "6A-II", "5A", "4A", "3A", "2A-I", "2A-II", "A-I", "A-II", "B-I", "B-II", "C"]),
        "OR" => pick(&["6A", "5A", "4A", "3A", "2A", "1A 8-player", "1A 6-player"]),
        "SC" => pick(&["4A", "3A", "2A", "1A Division I", "1A Division II"]),
        "SD" => pick(&["11AAA", "11AA", "11A", "11B", "9AA", "9A", "9B"]),
        "TN" if h.private => pick(&["Division II-AAA", "Division II-AA", "Division II-A"]),
        "TN" => a(6),
        "TX" if h.private => format!("TAPPS Division {}", ROMAN[at(4)]),
        "TX" => pick(&[
            "6A Division I",
            "6A Division II",
            "5A Division I",
            "5A Division II",
            "4A Division I",
            "4A Division II",
            "3A Division I",
            "3A Division II",
            "2A Division I",
            "2A Division II",
            "1A Division I",
            "1A Division II",
        ]),
        "UT" => pick(&["6A", "5A", "4A", "3A", "2A", "1A", "8-player"]),
        "VA" => format!("Class {}", 6 - at(6)),
        "WA" => pick(&["4A", "3A", "2A", "1A", "2B", "1B"]),
        "WV" => pick(&["Class AAAA", "Class AAA", "Class AA", "Class A"]),
        "WI" => pick(&[
            "Division 1",
            "Division 2",
            "Division 3",
            "Division 4",
            "Division 5",
            "Division 6",
            "Division 7",
            "8-player",
        ]),
        "WY" => pick(&["4A", "3A", "2A", "1A 9-man", "1A 6-man"]),
        _ => a(n),
    }
}

/// The NFL division a club plays in.
pub(crate) fn nfl_division(w: &World, t: usize) -> Option<&'static str> {
    const NAMES: [&str; 8] =
        ["AFC East", "AFC North", "AFC South", "AFC West", "NFC East", "NFC North", "NFC South", "NFC West"];
    let (cf, dv) = w.nfl_alignment()?;
    let d = if *dv.get(t)? >= 4 { dv[t] } else { cf.get(t)? * 4 + dv[t] };
    NAMES.get(d).copied()
}

/// Where a team plays, in a line: "Big 12 · FBS", "NFC North", "Texas 5A".
pub(crate) fn team_line(w: &World, t: TeamRef) -> String {
    match t {
        TeamRef::College(i) => {
            format!("{} · {}", w.conference_name(&w.programs[i].conference), w.programs[i].division.name())
        }
        TeamRef::Nfl(i) => nfl_division(w, i).unwrap_or("NFL").to_string(),
        TeamRef::HighSchool(i) => {
            let st = w.high_schools[i].state;
            format!("{} {}", state_name(&w.states[st].code), hs_class(w, i))
        }
    }
}

/// A state's name from its postal code.
pub(crate) fn state_name(code: &str) -> &str {
    const S: [(&str, &str); 51] = [
        ("AL", "Alabama"),
        ("AK", "Alaska"),
        ("AZ", "Arizona"),
        ("AR", "Arkansas"),
        ("CA", "California"),
        ("CO", "Colorado"),
        ("CT", "Connecticut"),
        ("DE", "Delaware"),
        ("DC", "District of Columbia"),
        ("FL", "Florida"),
        ("GA", "Georgia"),
        ("HI", "Hawaii"),
        ("ID", "Idaho"),
        ("IL", "Illinois"),
        ("IN", "Indiana"),
        ("IA", "Iowa"),
        ("KS", "Kansas"),
        ("KY", "Kentucky"),
        ("LA", "Louisiana"),
        ("ME", "Maine"),
        ("MD", "Maryland"),
        ("MA", "Massachusetts"),
        ("MI", "Michigan"),
        ("MN", "Minnesota"),
        ("MS", "Mississippi"),
        ("MO", "Missouri"),
        ("MT", "Montana"),
        ("NE", "Nebraska"),
        ("NV", "Nevada"),
        ("NH", "New Hampshire"),
        ("NJ", "New Jersey"),
        ("NM", "New Mexico"),
        ("NY", "New York"),
        ("NC", "North Carolina"),
        ("ND", "North Dakota"),
        ("OH", "Ohio"),
        ("OK", "Oklahoma"),
        ("OR", "Oregon"),
        ("PA", "Pennsylvania"),
        ("RI", "Rhode Island"),
        ("SC", "South Carolina"),
        ("SD", "South Dakota"),
        ("TN", "Tennessee"),
        ("TX", "Texas"),
        ("UT", "Utah"),
        ("VT", "Vermont"),
        ("VA", "Virginia"),
        ("WA", "Washington"),
        ("WV", "West Virginia"),
        ("WI", "Wisconsin"),
        ("WY", "Wyoming"),
    ];
    S.iter().find(|x| x.0 == code).map_or(code, |x| x.1)
}

pub(crate) fn record_text(w: &World, t: TeamRef) -> String {
    season_record(w, t).0
}

pub(crate) fn record_note(w: &World, t: TeamRef) -> String {
    season_record(w, t).1
}

/// The people on a team's books, as indices into `people`.
pub fn roster_people(w: &World, t: TeamRef) -> Vec<usize> {
    let mut v: Vec<usize> = match t {
        TeamRef::Nfl(i) => w.clubs.get(i).map_or(Vec::new(), |c| c.roster.clone()),
        TeamRef::College(i) => w.programs[i].roster.clone(),
        TeamRef::HighSchool(i) => w.high_schools[i].roster.clone(),
    };
    // A player who walked away from football (or whose playing days ended)
    // stays on the list until the roster is rebuilt; he is not on the team.
    v.retain(|&k| w.people.get(k).is_some_and(|p| p.stage != Stage::Done));
    v
}

pub(crate) fn is_mine(w: &World, t: TeamRef) -> bool {
    mine(w) == Some(t)
}

/// The eye of the coach who reads a player on your staff (1-20): his
/// position coach, else the coordinator, else you. Used only to size the
/// range, never shown.
pub(crate) fn eye_for(w: &World, t: TeamRef, pos: Position) -> f64 {
    let g = Group::of(pos);
    let coach = match t {
        TeamRef::College(i) => {
            let p = &w.programs[i];
            p.position_coaches[g as usize].or(if g.offense() { p.oc } else { p.dc }).or(p.head_coach)
        }
        TeamRef::Nfl(i) => {
            let tm = &w.nfl[i];
            if g.offense() { tm.oc } else { tm.dc }.or(tm.head_coach)
        }
        TeamRef::HighSchool(i) => w.high_schools[i].coach,
    };
    coach.and_then(|c| w.coaches.get(c)).map_or(10.0, |c| c.evaluation())
}

pub(crate) fn reader_key(w: &World) -> u64 {
    w.career.as_ref().map_or(0x5EED, |c| c.seed as u64 ^ (c.coach as u64) << 32)
}

/// His class or years, in the words of his level.
pub(crate) fn class_text(w: &World, p: &Person) -> String {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            // The football year turns after the Super Bowl, the school year
            // in August: until then this year's graduating class are the seniors.
            let seniors = if w.today < w.dates.hs_school_start { w.year } else { w.year + 1 };
            let grade = 12 - (p.class_of - seniors).clamp(0, 3);
            match grade {
                12 => "Sr".into(),
                11 => "Jr".into(),
                10 => "So".into(),
                _ => "Fr".into(),
            }
        }
        Stage::College | Stage::Declared => {
            let c = ["Fr", "So", "Jr", "Sr", "Sr"][(p.college_seasons as usize).min(4)];
            if p.redshirt_used { format!("RS {c}") } else { c.to_string() }
        }
        Stage::Pro => {
            let y = p.pro.as_ref().map_or(0, |c| c.accrued);
            if y == 0 { "Rookie".into() } else { format!("{y} yr{}", if y == 1 { "" } else { "s" }) }
        }
        Stage::Done => "Retired".into(),
    }
}

pub(crate) fn level_of(w: &World, p: &Person) -> Level {
    match p.stage {
        Stage::Pro => Level::NFL,
        Stage::Declared | Stage::Done => match p.college {
            Some(c) if p.stage == Stage::Declared => afm_engine::world::division_level(w.programs[c].division),
            _ => p.player.history.last().map_or(Level::HS, |h| h.level),
        },
        _ => p.level(&w.programs),
    }
}

pub(crate) fn hometown(w: &World, p: &Person) -> String {
    // A Canadian in the CFL grew up across the border.
    if let Some(&school) = w.others.nationals.get(&p.player.id) {
        return afm_engine::world::pro::other::national_home(p.player.id, school);
    }
    let s = w.high_schools.get(p.hs);
    match s {
        Some(s) => format!("{}, {}", s.town, w.states.get(s.state).map_or("", |x| x.code.as_str())),
        None => w.states.get(p.home_state).map_or(String::new(), |x| x.code.clone()),
    }
}

pub(crate) fn status_text(w: &World, p: &Person) -> String {
    if let Some(c) = &p.pro {
        use afm_engine::world::pro::Status;
        let s = match c.status {
            Status::Roster => "",
            Status::PracticeSquad => "Practice squad",
            Status::Reserve => "Injured reserve",
            Status::FreeAgent => "Free agent",
            Status::Other(_) => "Other league",
        };
        if c.out_weeks > 0 {
            return format!("Injured, {} wk", c.out_weeks);
        }
        return s.to_string();
    }
    if p.cal.portal.is_some() {
        return "In the portal".into();
    }
    if p.cal.trouble {
        return "Suspended".into();
    }
    if p.cal.academic_sit {
        return "Academically ineligible".into();
    }
    if p.sit_out > 0.0 || p.cal.sitting_transfer {
        return "Sitting out (transfer)".into();
    }
    let _ = w;
    String::new()
}

/// The schools that offered a recruit, once each: his early offers first
/// (by day), then the last stage's, best first, then the school he
/// committed or signed with if it isn't among them (its scholarship came
/// with the commitment). A walk-on place is no offer.
pub(crate) fn offers_of(p: &Person) -> Vec<usize> {
    let mut early = p.cal.early_offers.clone();
    early.sort_by_key(|o| o.1);
    let mut v: Vec<usize> = Vec::new();
    let took = p.cal.signed.filter(|s| !s.walk_on).map(|s| s.program).or(p.cal.commit.filter(|c| !c.2).map(|c| c.0));
    for o in early.iter().map(|o| o.0).chain(p.cal.offers.iter().copied()).chain(took) {
        if !v.contains(&o) {
            v.push(o);
        }
    }
    v
}

pub(crate) fn level_words(l: Level) -> &'static str {
    match l {
        Level::HS => "High school",
        Level::FBS => "FBS",
        Level::FCS => "FCS",
        Level::SmallCollege => "Small college",
        Level::NFL => "NFL",
    }
}

pub(crate) fn role_words(r: PlayingTime) -> &'static str {
    match r {
        PlayingTime::Starter => "Starter",
        PlayingTime::Rotation => "Rotation",
        PlayingTime::Backup => "Backup",
        PlayingTime::Redshirt => "Redshirt",
        PlayingTime::PracticeSquad => "Practice squad",
        PlayingTime::Unsigned => "Unsigned",
    }
}

pub(crate) fn person_index(w: &World, id: u32) -> Option<usize> {
    // People are kept in id order: ids are handed out in turn
    // (`next_person`) and people are only ever appended or retained, so a
    // binary search finds anyone at once (a scan took seconds on a full
    // world's leaderboards).
    w.people.binary_search_by_key(&id, |p| p.player.id).ok()
}

pub(crate) fn ft_in(inches: f64) -> String {
    afm_engine::body::height_text(inches)
}

pub(crate) fn mood_words(p: &Person) -> &'static str {
    match p.mood {
        m if m > 0.5 => "Very happy",
        m if m > 0.15 => "Happy",
        m if m > -0.15 => "Settled",
        m if m > -0.5 => "Unhappy",
        _ => "Very unhappy",
    }
}

pub(crate) fn row(w: &World, k: usize, t: TeamRef, own: bool, reader: u64) -> Value {
    let p = &w.people[k];
    let pos = p.player.position;
    let eye = if own { eye_for(w, t, pos) } else { 10.0 };
    let ovr = reads::overall(&p.player, own, eye, reader);
    let notes = w.notes_on(p.player.id).len();
    let starter = p.last_role == Some(PlayingTime::Starter);
    json!({
        "id": p.player.id,
        "name": p.name,
        "pos": pos.name(),
        "group": unit(pos),
        "age": p.age_on(w.today),
        "class": class_text(w, p),
        "ht": ft_in(p.player.body.height),
        "wt": p.player.body.weight.round(),
        "stars": p.stars,
        "home": hometown(w, p),
        "ovr": ovr.json(),
        "status": status_text(w, p),
        "mood": mood_words(p),
        "starter": starter,
        "notes": notes,
    })
}

pub(crate) fn unit(pos: Position) -> &'static str {
    if pos.is_specialist() {
        "ST"
    } else if matches!(pos, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL) {
        "OFF"
    } else {
        "DEF"
    }
}

pub fn status(w: &World) -> Value {
    let mut next = next_dates(w, 3);
    // Your next game is on the bar too, in its place by date.
    if let Some(g) = mine(w).and_then(|t| next_game(w, t))
        && let Some(days) = g["in"].as_i64()
    {
        let text = format!(
            "{} {}",
            if g["home"].as_bool().unwrap_or(false) { "vs" } else { "at" },
            g["opp"]["name"].as_str().unwrap_or("")
        );
        let at = next.iter().position(|e| e["in"].as_i64().is_some_and(|d| d > days)).unwrap_or(next.len());
        next.insert(at, json!({ "date": g["date"], "in": days, "text": text }));
        next.truncate(3);
    }
    json!({
        "today": date_text(w.today),
        "season": w.year,
        "mine": mine(w).map(|t| team_brief(w, t)),
        "you": w.career.as_ref().map(|c| c.you.name.clone()),
        "seat": crate::lane_frontoffice::setup::seat_json(w),
        "next": next,
        "pinned": pinned(w).into_iter().chain(crate::lane_core::desk::pinned(w)).collect::<Vec<_>>(),
        "names": format!("{:?}", w.options.names),
        "unread": crate::lane_core::desk::unread(w),
        "reveals": w.career.as_ref().map_or(0, |c| c.desk.reveals.len()),
        "my_players": w.career.as_ref().map_or(0, |c| c.desk.listed.len()),
    })
}

/// The next dated moments on your calendar (for the Continue bar).
pub(crate) fn next_dates(w: &World, n: usize) -> Vec<Value> {
    let seat = w.seat;
    w.calendar
        .events
        .range(w.today..)
        .flat_map(|(d, evs)| evs.iter().map(move |e| (d, e)))
        .filter(|(_, e)| e.loud >= Loudness::Big && seat.is_none_or(|s| s.cares(e)))
        .take(n)
        .map(|(d, e)| json!({ "date": date_text(*d), "in": w.today.days_to(*d), "text": e.text }))
        .collect()
}

pub(crate) fn pinned(w: &World) -> Vec<Value> {
    let seat = w.seat;
    w.calendar
        .events
        .range(w.today.add(-14)..=w.today)
        .flat_map(|(d, evs)| evs.iter().map(move |e| (d, e)))
        .filter(|(_, e)| {
            e.needs_decision
                && e.rule != "people: meeting"
                && (seat.is_some_and(|s| s.cares(e)) || matches!(e.rule, "Your school" | "Your career"))
        })
        .map(|(d, e)| json!({ "date": date_text(*d), "text": e.text, "rule": e.rule }))
        .chain(
            // A meeting that waits for you (people): it opens the room.
            w.folk.scenes.iter().filter(|s| s.stops && !s.done && s.date <= w.today).map(|s| {
                json!({ "date": date_text(s.date), "text": crate::lane_people::plain(&s.title), "rule": "A meeting waits for you", "go": "people_scene", "id": s.id })
            }),
        )
        .collect()
}

/// After Continue: the days that ran and why it stopped.
pub fn stopped(w: &World, from: Date, at: Date) -> Value {
    let seat = w.seat;
    // Stopped on the morning of a day still to play: what it holds.
    let morning = (at == w.today && at > from).then(|| w.morning_stop(at)).flatten();
    let mut why: Vec<Value> = w
        .calendar
        .on(at)
        .iter()
        .filter(|e| {
            e.needs_decision || e.loud == Loudness::Huge || (e.loud == Loudness::Big && seat.is_none_or(|s| s.cares(e)))
        })
        .map(|e| json!({ "text": e.text, "loud": e.loud.name(), "decision": e.needs_decision }))
        .collect();
    if let Some(m) = &morning {
        why.insert(0, json!({ "text": m, "loud": "Big", "decision": false }));
    }
    // Your game that day, if you played: the screens open its page.
    let games = w.seat_games();
    let game = games.iter().rposition(|g| g.date == at && at >= from).map(|k| {
        let g = &games[k];
        json!({ "index": k, "text": format!("{} {} at {} {}", g.names[1], g.stats.score[1], g.names[0], g.stats.score[0]) })
    });
    // A meeting waiting this morning (people): the screens open it.
    let meeting = morning
        .as_ref()
        .and_then(|_| w.folk.scenes.iter().filter(|s| !s.done && s.stops && s.date == at).map(|s| s.id).min());
    json!({ "from": date_text(from), "days": from.days_to(w.today), "stopped_on": date_text(at), "morning": morning.is_some(), "today": date_text(w.today), "why": why, "game": game, "meeting": meeting, "status": status(w) })
}

pub fn home(w: &World) -> Value {
    let me = mine(w);
    let team = me.map(|t| {
        let mut v = team_brief(w, t);
        v["record"] = json!(record_text(w, t));
        v["record_note"] = json!(record_note(w, t));
        v["level"] = json!(match t {
            TeamRef::Nfl(_) => "NFL".to_string(),
            TeamRef::College(i) => {
                format!("{} · {}", w.programs[i].division.name(), w.conference_name(&w.programs[i].conference))
            }
            TeamRef::HighSchool(i) => {
                let s = &w.high_schools[i];
                format!("{} high school · {}", w.states[s.state].code, hs_class(w, s.id))
            }
        });
        v
    });
    let next_game = me.and_then(|t| next_game(w, t));
    json!({
        "status": status(w),
        "team": team,
        "next_game": next_game,
        "inbox": inbox(w, 21)["items"].clone(),
        "news": news(w, &json!({ "limit": 12 }))["items"].clone(),
        "followed": followed(w),
    })
}

pub(crate) fn next_game(w: &World, t: TeamRef) -> Option<Value> {
    match t {
        TeamRef::College(i) => w
            .college_fixtures(Some(i))
            .into_iter()
            .filter(|g| g.score.is_none() && g.date >= w.today)
            .min_by_key(|g| g.date)
            .map(|g| {
                let home = g.home == i;
                let opp = if home { g.away } else { g.home };
                json!({
                    "date": date_text(g.date),
                    "in": w.today.days_to(g.date),
                    "opp": team_brief(w, TeamRef::College(opp)),
                    "home": home,
                    "name": g.name,
                    "why": g.why,
                })
            }),
        TeamRef::Nfl(i) => w.nfl_fixtures(Some(i)).into_iter().find(|g| !g.played).map(|g| {
            let home = g.home == i;
            let opp = if home { g.away } else { g.home };
            json!({
                "date": g.when.map(date_text),
                "in": g.when.map(|d| w.today.days_to(d)),
                "week": g.week,
                "opp": team_brief(w, TeamRef::Nfl(opp)),
                "home": home,
            })
        }),
        TeamRef::HighSchool(i) => w.hs_fixtures(i).into_iter().find(|g| g.date >= w.today).map(|g| {
            let home = g.home == i;
            let opp = if home { g.away } else { g.home };
            json!({
                "date": date_text(g.date),
                "in": w.today.days_to(g.date),
                "opp": team_brief(w, TeamRef::HighSchool(opp)),
                "home": home,
            })
        }),
    }
}

/// The players on your "my players" list: everyone you have a note on.
pub(crate) fn followed(w: &World) -> Vec<Value> {
    let Some(c) = &w.career else { return Vec::new() };
    c.notes
        .iter()
        .filter(|(_, l)| !l.is_empty())
        .filter_map(|(id, l)| {
            let k = person_index(w, *id)?;
            let p = &w.people[k];
            Some(json!({ "id": id, "name": p.name, "pos": p.player.position.name(), "where": where_now(w, p), "note": l.last().map(|n| n.text.clone()) }))
        })
        .collect()
}

pub(crate) fn where_now(w: &World, p: &Person) -> String {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => w.school_name(p.hs).to_string(),
        Stage::College => p.college.map_or("College".into(), |c| w.program_name(c).to_string()),
        Stage::Declared => "Draft pool".into(),
        Stage::Pro => match p.pro.as_ref().map(|c| (c.team, c.status)) {
            Some((Some(t), _)) => w.nfl_team_name(t),
            Some((None, afm_engine::world::pro::Status::Other(k))) => {
                use afm_engine::world::pro::other;
                match person_index(w, p.player.id).and_then(|i| w.others.team_of(i)) {
                    Some(fr) => format!("{} ({})", other::name(fr, w.year), other::league_short(k)),
                    None => format!("Between teams ({})", other::league_short(k)),
                }
            }
            _ => "Free agent".into(),
        },
        Stage::Done => "Out of football".into(),
    }
}

/// For a high school seat, the names that make a moment yours: your
/// school's and your players' (the calendar files a moment under a college
/// program or an NFL club, never a high school).
pub(crate) fn hs_names(w: &World) -> Vec<String> {
    let Some(t @ TeamRef::HighSchool(h)) = mine(w) else { return Vec::new() };
    let mut v: Vec<String> = roster_people(w, t).iter().map(|&k| w.people[k].name.clone()).collect();
    v.push(w.school_name(h).to_string());
    v
}

/// Whether a line is about a high school seat's own people: one of his
/// players by full name anywhere, or his school in a high school line (a
/// school named Allen is not every college player called Allen, nor Allen
/// East).
pub(crate) fn about_us(names: &[String], e: &afm_engine::calendar::Event) -> bool {
    let Some((school, people)) = names.split_last() else { return false };
    let named = |n: &str, whole: bool| {
        e.text.match_indices(n).any(|(i, _)| {
            let before = e.text[..i].chars().next_back();
            let rest = &e.text[i + n.len()..];
            let longer = whole && rest.strip_prefix(' ').and_then(|r| r.chars().next()).is_some_and(char::is_uppercase);
            !before.is_some_and(char::is_alphanumeric)
                && !rest.chars().next().is_some_and(char::is_alphanumeric)
                && !longer
        })
    };
    people.iter().any(|n| named(n, false)) || (e.level == CalLevel::HighSchool && named(school, true))
}

/// Whether a calendar line is about your own team: your program or club,
/// at any loudness, or a high school seat's school and players by name.
pub(crate) fn yours_test(w: &World) -> impl Fn(&afm_engine::calendar::Event) -> bool + use<> {
    let seat = w.seat;
    let names = hs_names(w);
    move |e| {
        seat.is_some_and(|s| {
            (s.level == CalLevel::College && e.program.is_some() && e.program == s.program)
                || (e.nfl_team.is_some() && e.nfl_team == s.nfl_team)
        }) || about_us(&names, e)
            || matches!(e.rule, "Your school" | "Your career")
    }
}

/// Your own day's lines (meetings, talks, your staff's news): `Slot` lines
/// about your team. A high school line carries no team, and the world posts
/// one only for a high school seat, so every one is yours.
pub(crate) fn my_slot_test(w: &World) -> impl Fn(&afm_engine::calendar::Event) -> bool + use<> {
    let yours = yours_test(w);
    move |e| e.kind == afm_engine::calendar::Kind::Slot && (e.level == CalLevel::HighSchool || yours(e))
}

/// Turns the linked names of lines into the screens' parts (L2): plain text
/// `{t}`, or a link `{t, go, kind?, id?, index?}` that `app.js` opens. A
/// person is linked while he is in the world; a game opens its game page
/// when you kept it, else the home team's schedule.
struct Linker {
    people: std::collections::HashSet<u32>,
}

impl Linker {
    fn new<'a>(w: &World, refs: impl Iterator<Item = &'a Ref>) -> Linker {
        let want: std::collections::HashSet<u32> = refs
            .filter_map(|r| match r.to {
                Link::Person(id) => Some(id),
                _ => None,
            })
            .collect();
        let people = if want.is_empty() {
            Default::default()
        } else {
            w.people.iter().map(|p| p.player.id).filter(|id| want.contains(id)).collect()
        };
        Linker { people }
    }

    /// A line's parts. `program` and `club` are what the line is filed
    /// under: their names are linked too when the writer didn't.
    fn parts(&self, w: &World, text: &str, refs: &[Ref], program: Option<usize>, club: Option<usize>) -> Value {
        let mut refs = refs.to_vec();
        if let Some(p) = program.filter(|&p| p < w.programs.len()) {
            for name in [w.program_name(p), w.programs[p].real_name.as_str()] {
                afm_engine::calendar::events::add_ref(&mut refs, text, Link::Program(p as u32), name);
            }
        }
        if let Some(t) = club.filter(|&t| t < w.nfl.len()) {
            afm_engine::calendar::events::add_ref(&mut refs, text, Link::Club(t as u32), w.nfl[t].name);
        }
        refs.sort_by_key(|r| r.at);
        let parts: Vec<Value> = afm_engine::calendar::events::segments(text, &refs)
            .into_iter()
            .map(|(t, to)| match to.and_then(|to| self.target(w, to)) {
                Some(mut v) => {
                    v["t"] = json!(t);
                    v
                }
                None => json!({ "t": t }),
            })
            .collect();
        json!(parts)
    }

    fn target(&self, w: &World, to: Link) -> Option<Value> {
        Some(match to {
            Link::Person(id) => {
                if !self.people.contains(&id) {
                    return None;
                }
                json!({ "go": "player", "id": id })
            }
            Link::Coach(c) if (c as usize) < w.coaches.len() => json!({ "go": "coach", "id": c }),
            Link::Program(p) if (p as usize) < w.programs.len() => json!({ "go": "team", "kind": "college", "id": p }),
            Link::School(h) if (h as usize) < w.high_schools.len() => json!({ "go": "team", "kind": "hs", "id": h }),
            Link::Club(t) if (t as usize) < w.nfl.len() => json!({ "go": "team", "kind": "nfl", "id": t }),
            Link::Game(date, level, home) => {
                let home = home as usize;
                let kept = w.seat_games().iter().position(|g| g.date == date && g.level == level && g.teams[0] == home);
                match (kept, level) {
                    (Some(k), _) => json!({ "go": "game", "index": k }),
                    (None, CalLevel::Nfl) if home < w.nfl.len() => {
                        json!({ "go": "schedule", "kind": "nfl", "id": home })
                    }
                    (None, CalLevel::College) if home < w.programs.len() => {
                        json!({ "go": "schedule", "kind": "college", "id": home })
                    }
                    (None, CalLevel::HighSchool) if home < w.high_schools.len() => {
                        json!({ "go": "schedule", "kind": "hs", "id": home })
                    }
                    _ => return None,
                }
            }
            _ => return None,
        })
    }
}

/// The refs of a seat game's score line: both teams and the score.
fn final_line(g: &afm_engine::world::seat::SeatGame) -> (String, Vec<Ref>) {
    let [h, a] = g.stats.score;
    let text = format!("Final: {} {a}, {} {h}.", g.names[1], g.names[0]);
    let team = |k: usize| match g.level {
        CalLevel::Nfl => Link::Club(g.teams[k] as u32),
        CalLevel::HighSchool => Link::School(g.teams[k] as u32),
        _ => Link::Program(g.teams[k] as u32),
    };
    let mut refs = Vec::new();
    afm_engine::calendar::events::add_ref(&mut refs, &text, Link::Game(g.date, g.level, g.teams[0] as u32), "Final");
    afm_engine::calendar::events::add_ref(&mut refs, &text, team(1), &g.names[1]);
    afm_engine::calendar::events::add_ref(&mut refs, &text, team(0), &g.names[0]);
    (text, refs)
}

pub fn inbox(w: &World, days: i32) -> Value {
    let seat = w.seat;
    // Yours: anything about your team. Around you: your level's big moments
    // and huge ones anywhere (the rest is in the news).
    let yours = yours_test(w);
    let from = w.today.add(-days);
    let picked: Vec<(&Date, &afm_engine::calendar::Event)> = w
        .calendar
        .events
        .range(from..=w.today)
        .rev()
        .flat_map(|(d, evs)| evs.iter().rev().map(move |e| (d, e)))
        .filter(|(_, e)| {
            // Conference moves reach the coaches they matter to (1b): FBS
            // moves every college coach, lower ones only their own level.
            if let Some(reach) = afm_engine::world::finance::college::move_reaches(w, e) {
                return reach || yours(e);
            }
            e.needs_decision
                || yours(e)
                || e.loud == Loudness::Huge
                || (e.loud >= Loudness::Big && seat.is_none_or(|s| s.cares(e)))
        })
        .take(80)
        .collect();
    let staff: Vec<&afm_engine::world::news::NewsItem> = match mine(w) {
        Some(TeamRef::College(p)) => {
            w.news.iter().rev().filter(|n| n.program == Some(p) && n.date.is_none_or(|d| d >= from)).take(20).collect()
        }
        _ => Vec::new(),
    };
    let link =
        Linker::new(w, picked.iter().flat_map(|(_, e)| e.refs.iter()).chain(staff.iter().flat_map(|n| n.refs.iter())));
    let mut items: Vec<(Date, Value)> = picked
        .into_iter()
        .map(|(d, e)| {
            let loud = if yours(e) { e.loud.max(Loudness::Medium) } else { e.loud };
            let parts = link.parts(w, &e.text, &e.refs, e.program, e.nfl_team);
            (*d, json!({ "date": date_text(*d), "loud": loud.name(), "text": e.text, "parts": parts, "rule": e.rule, "decision": e.needs_decision }))
        })
        .collect();
    // Your own games, won or lost.
    for g in w.seat_games().iter().filter(|g| g.date >= from) {
        let (text, refs) = final_line(g);
        let parts = link.parts(w, &text, &refs, None, None);
        items.push((
            g.date,
            json!({ "date": date_text(g.date), "loud": "Big", "text": text, "parts": parts, "rule": "", "decision": false }),
        ));
    }
    // Your staff's own messages to you (the program's inbox channel).
    for n in staff {
        let d = n.date.unwrap_or(w.today);
        let parts = link.parts(w, &n.text, &n.refs, n.program, None);
        items.push((d, json!({ "date": n.date.map(date_text), "loud": "Medium", "text": n.text, "parts": parts, "rule": "", "decision": false })));
    }
    items.sort_by_key(|x| std::cmp::Reverse(x.0));
    json!({ "items": items.into_iter().map(|x| x.1).collect::<Vec<_>>() })
}

pub fn news(w: &World, args: &Value) -> Value {
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(120) as usize;
    let mine_only = args.get("mine").and_then(Value::as_bool).unwrap_or(false);
    let my_prog = match mine(w) {
        Some(TeamRef::College(p)) => Some(p),
        _ => None,
    };
    let mut items: Vec<Value> = Vec::new();
    // Yours: your team, and for a high school coach his state's high school
    // lines too ("TX high school poll").
    let yours = yours_test(w);
    let state = match mine(w) {
        Some(TeamRef::HighSchool(h)) => w.high_schools.get(h).map(|s| format!("{} ", w.states[s.state].code)),
        _ => None,
    };
    let ours = |e: &afm_engine::calendar::Event| {
        yours(e) || state.as_ref().is_some_and(|code| e.level == CalLevel::HighSchool && e.text.contains(code.as_str()))
    };
    // The calendar's small lines are the news wire.
    let wire: Vec<(&Date, &afm_engine::calendar::Event)> = w
        .calendar
        .events
        .range(..=w.today)
        .rev()
        .flat_map(|(d, evs)| evs.iter().rev().map(move |e| (d, e)))
        .filter(|(_, e)| !mine_only || ours(e))
        .take(limit)
        .collect();
    let desk_room = limit - wire.len();
    let my_club = match mine(w) {
        Some(TeamRef::Nfl(t)) => Some(format!("The {} ", w.nfl[t].name)),
        _ => None,
    };
    // The news desk's stories: a college program's, or the NFL's (a story
    // with no program, except the FBS diversity report).
    let desk: Vec<(&afm_engine::world::news::NewsItem, bool)> = if desk_room == 0 {
        Vec::new()
    } else {
        w.news
            .iter()
            .rev()
            .map(|n| {
                let college =
                    n.program.is_some() || matches!(n.kind, afm_engine::world::news::NewsKind::DiversityReport);
                (n, college)
            })
            .filter(|(n, college)| {
                let yours = match (&my_club, my_prog) {
                    (_, Some(p)) => n.program == Some(p),
                    (Some(club), None) => !college && n.text.starts_with(club.as_str()),
                    _ => false,
                };
                !mine_only || yours
            })
            .take(desk_room)
            .collect()
    };
    let link = Linker::new(
        w,
        wire.iter().flat_map(|(_, e)| e.refs.iter()).chain(desk.iter().flat_map(|(n, _)| n.refs.iter())),
    );
    for (d, e) in wire {
        let parts = link.parts(w, &e.text, &e.refs, e.program, e.nfl_team);
        items.push(json!({ "date": date_text(*d), "text": e.text, "parts": parts, "level": format!("{:?}", e.level), "loud": e.loud.name() }));
    }
    for (n, college) in desk {
        let level = if college { "College" } else { "Nfl" };
        let parts = link.parts(w, &n.text, &n.refs, n.program, None);
        items.push(
            json!({ "date": n.date.map(date_text), "text": n.text, "parts": parts, "level": level, "loud": "Small" }),
        );
    }
    json!({ "items": items })
}

/// Your level's year: every dated moment from today to the end of the
/// football year, yours marked.
/// The football year's fixed dates at your level (and the few everyone
/// watches) for the given seasons: the calendar list and the month grid.
pub(crate) fn fixed_dates(w: &World, years: &[i32]) -> Vec<(Date, String)> {
    use afm_engine::calendar::{Level, YearDates};
    let level = w.seat.map(|s| s.level);
    let mut items: Vec<(Date, String)> = Vec::new();
    for &year in years {
        let y = YearDates::new(year);
        let mut add = |d: Date, text: String, lv: Level| {
            let everyone = matches!(text.as_str(), "Super Bowl" | "NFL draft, round 1" | "National championship game");
            if everyone || level.is_none_or(|l| l == lv) {
                items.push((d, text));
            }
        };
        add(y.super_bowl, "Super Bowl".into(), Level::Nfl);
        add(y.combine_start, "NFL combine".into(), Level::Nfl);
        add(y.league_year.date, "The NFL's new league year: free agency opens".into(), Level::Nfl);
        add(y.draft[0].date, "NFL draft, round 1".into(), Level::Nfl);
        add(y.draft[1].date, "NFL draft, rounds 2 and 3".into(), Level::Nfl);
        add(y.draft[2].date, "NFL draft, rounds 4 to 7".into(), Level::Nfl);
        add(y.otas.0, "Organized team activities start".into(), Level::Nfl);
        add(y.minicamp.0, "Mandatory minicamp".into(), Level::Nfl);
        add(y.rookies_report, "Rookies report to camp".into(), Level::Nfl);
        add(y.veterans_report, "Veterans report to camp".into(), Level::Nfl);
        add(y.cutdown.date, "Cutdown day: 90 to 53".into(), Level::Nfl);
        add(y.nfl_kickoff, "NFL kickoff game".into(), Level::Nfl);
        add(y.trade_deadline.date, "NFL trade deadline".into(), Level::Nfl);
        add(y.black_monday, "Black Monday".into(), Level::Nfl);
        add(y.wild_card, "NFL wild card round".into(), Level::Nfl);
        add(y.spring_practice.0, "Spring practice opens".into(), Level::College);
        add(y.spring_game, "Spring game".into(), Level::College);
        add(y.media_days.0, "Conference media days".into(), Level::College);
        add(y.fall_camp, "Fall camp opens".into(), Level::College);
        add(y.portal_open, "Transfer portal opens".into(), Level::College);
        add(y.portal_close, "Transfer portal closes".into(), Level::College);
        add(y.rivalry_week, "Rivalry week".into(), Level::College);
        add(y.conference_championships, "Conference championships".into(), Level::College);
        add(y.selection_day, "Playoff selection day".into(), Level::College);
        add(y.heisman, "Heisman ceremony".into(), Level::College);
        add(y.cfp_first_round.0, "College Football Playoff, first round".into(), Level::College);
        add(y.title_game, "National championship game".into(), Level::College);
        add(y.declare_deadline, "Deadline to declare for the NFL draft".into(), Level::College);
        if let Some(d) = y.early_signing {
            add(d, "Early signing period opens".into(), Level::College);
        }
        add(y.signing_day, "National signing day".into(), Level::College);
        add(y.hs_school_start, "High school year starts".into(), Level::HighSchool);
        add(y.official_visits_from, "Official visits open for juniors".into(), Level::HighSchool);
        add(y.all_american_bowl, "All-American Bowl".into(), Level::HighSchool);
        add(y.signing_day, "National signing day".into(), Level::HighSchool);
    }
    items
}

pub fn calendar(w: &World) -> Value {
    let seat = w.seat;
    type Line<'a> = (Date, String, &'a str, bool, Vec<Ref>, Option<usize>, Option<usize>);
    let mut items: Vec<Line> = fixed_dates(w, &[w.year, w.year + 1])
        .into_iter()
        .map(|(d, t)| (d, t, "date", false, Vec::new(), None, None))
        .collect();
    // Your games.
    if let Some(t) = mine(w) {
        for g in schedule_rows(w, t) {
            items.push((g.0, g.1, "game", true, g.2, None, None));
        }
    }
    // Moments already set for days ahead, and your own meetings.
    let my_slot = my_slot_test(w);
    for (d, evs) in w.calendar.events.range(w.today..) {
        for e in evs.iter().filter(|e| e.loud >= Loudness::Big || e.loud >= Loudness::Medium && my_slot(e)) {
            items.push((
                *d,
                e.text.to_string(),
                e.loud.name(),
                my_slot(e) || seat.is_some_and(|s| s.cares(e)),
                e.refs.clone(),
                e.program,
                e.nfl_team,
            ));
        }
    }
    items.retain(|x| x.0 >= w.today);
    // One line a moment: a date two levels share (signing day) shows once,
    // your own game's line first on its day.
    items.sort_by(|a, b| (a.0, !a.3, &a.1).cmp(&(b.0, !b.3, &b.1)));
    let mut seen = std::collections::HashSet::new();
    items.retain(|x| seen.insert((x.0, x.1.clone())));
    items.truncate(160);
    let link = Linker::new(w, items.iter().flat_map(|x| x.4.iter()));
    let items: Vec<Value> = items
        .into_iter()
        .map(|(d, text, loud, mine, refs, prog, club)| {
            let parts = link.parts(w, &text, &refs, prog, club);
            json!({ "date": date_text(d), "in": w.today.days_to(d), "text": text, "parts": parts, "loud": loud, "mine": mine })
        })
        .collect();
    json!({ "items": items })
}

/// Your team's games ahead as (date, text).
pub(crate) fn schedule_rows(w: &World, t: TeamRef) -> Vec<(Date, String, Vec<Ref>)> {
    match t {
        TeamRef::College(p) => w
            .college_fixtures(Some(p))
            .into_iter()
            .filter(|f| f.score.is_none())
            .map(|f| {
                let home = f.home == p;
                let opp = if home { f.away } else { f.home };
                let at = if home || f.neutral { "vs" } else { "at" };
                let name = team_name(w, TeamRef::College(opp));
                let text = format!("{at} {name}");
                let mut refs = Vec::new();
                afm_engine::calendar::events::add_ref(&mut refs, &text, Link::Program(opp as u32), &name);
                (f.date, text, refs)
            })
            .collect(),
        TeamRef::Nfl(c) => w
            .nfl_fixtures(Some(c))
            .into_iter()
            .filter(|f| !f.played)
            .filter_map(|f| {
                let home = f.home == c;
                let opp = if home { f.away } else { f.home };
                let name = w.nfl_team_name(opp);
                let text = format!("{} {name}", if home { "vs" } else { "at" });
                let mut refs = Vec::new();
                afm_engine::calendar::events::add_ref(&mut refs, &text, Link::Club(opp as u32), &name);
                Some((f.when?, text, refs))
            })
            .collect(),
        TeamRef::HighSchool(h) => w
            .hs_fixtures(h)
            .into_iter()
            .filter(|f| f.date >= w.today)
            .map(|f| {
                let home = f.home == h;
                let opp = if home { f.away } else { f.home };
                let name = w.school_name(opp);
                let text = format!("{} {name}", if home { "vs" } else { "at" });
                let mut refs = Vec::new();
                afm_engine::calendar::events::add_ref(&mut refs, &text, Link::School(opp as u32), name);
                (f.date, text, refs)
            })
            .collect(),
    }
}

/// How far an NFL season went, by `NflTeamSeason::round`.
pub(crate) const NFL_ROUNDS: [&str; 6] = [
    "",
    "Lost in the wild card round",
    "Lost in the divisional round",
    "Lost the conference title game",
    "Lost the Super Bowl",
    "Won the Super Bowl",
];

pub fn team(w: &World, t: TeamRef) -> Value {
    let mut v = team_brief(w, t);
    v["record"] = json!(record_text(w, t));
    v["record_note"] = json!(record_note(w, t));
    v["mine"] = json!(is_mine(w, t));
    v["next_game"] = next_game(w, t).unwrap_or(Value::Null);
    match t {
        TeamRef::College(i) => {
            let p = &w.programs[i];
            v["level"] = json!(format!("{} · {}", p.division.name(), w.conference_name(&p.conference)));
            v["place"] = json!(format!("{}, {}", p.city, w.states[p.state].code));
            v["stadium"] = json!(p.stadium);
            v["titles"] = json!(p.titles);
            v["conference_titles"] = json!(p.conference_titles);
            v["prestige"] = json!(prestige_words(p.prestige_rel));
            v["seasons"] = json!(
                p.seasons
                    .iter()
                    .rev()
                    .take(30)
                    .map(|s| json!({
                        "year": s.year,
                        "record": format!("{}-{}", s.wins, s.losses),
                        "conf": s.conference_title,
                        "nat": s.national_title,
                        "coach": s.coach.and_then(|c| w.coaches.get(c)).map(|c| c.name.clone()),
                        "pf": s.totals.points_for, "pa": s.totals.points_against, "g": s.totals.games,
                    }))
                    .collect::<Vec<_>>()
            );
        }
        TeamRef::Nfl(i) => {
            v["level"] = json!("NFL");
            if let Some(c) = w.clubs.get(i) {
                v["roster_count"] = json!(c.roster.len());
            }
            let titles: Vec<i32> = w.nfl_champions.iter().filter(|(_, t)| *t == i).map(|(y, _)| *y).collect();
            if !titles.is_empty() {
                let years: Vec<String> = titles.iter().map(i32::to_string).collect();
                v["prestige"] = json!(format!(
                    "{} Super Bowl{}: {}",
                    titles.len(),
                    if titles.len() == 1 { "" } else { "s" },
                    years.join(", ")
                ));
            }
            v["titles"] = json!(titles);
            v["seasons"] = json!(
                w.nfl_seasons
                    .iter()
                    .rev()
                    .filter(|s| s.team == i)
                    .take(30)
                    .map(|s| {
                        let record = if s.ties > 0 {
                            format!("{}-{}-{}", s.wins, s.losses, s.ties)
                        } else {
                            format!("{}-{}", s.wins, s.losses)
                        };
                        let note = NFL_ROUNDS[s.round.min(5) as usize];
                        let coach = s.coach.and_then(|c| w.coaches.get(c)).map(|c| c.name.clone());
                        json!({ "year": s.year, "record": record, "coach": coach, "note": note, "nat": s.round == 5 })
                    })
                    .collect::<Vec<_>>()
            );
        }
        TeamRef::HighSchool(i) => {
            let s = &w.high_schools[i];
            v["level"] = json!(format!("High school · {} {}", w.states[s.state].code, hs_class(w, i)));
            v["place"] = json!(format!("{}, {}", s.town, w.states[s.state].code));
            v["enrollment"] = json!(s.enrollment);
            v["titles"] = json!(s.state_titles);
            v["private"] = json!(s.private);
            v["seasons"] = json!(
                s.seasons
                    .iter()
                    .rev()
                    .take(30)
                    .map(|h| json!({
                        "year": h.year, "record": format!("{}-{}", h.wins, h.losses), "nat": h.state_title,
                        "coach": h.coach.and_then(|c| w.coaches.get(c)).map(|c| c.name.clone()),
                        "pf": h.points[0], "pa": h.points[1], "g": if h.points == [0, 0] { 0 } else { h.wins + h.losses },
                    }))
                    .collect::<Vec<_>>()
            );
        }
    }
    v
}

pub(crate) fn prestige_words(rel: f64) -> &'static str {
    match rel {
        r if r > 0.85 => "A blue blood",
        r if r > 0.65 => "A national power",
        r if r > 0.45 => "A solid program",
        r if r > 0.25 => "Rebuilding",
        _ => "A tough job",
    }
}

pub fn roster(w: &World, t: TeamRef) -> Value {
    let own = is_mine(w, t);
    let reader = reader_key(w);
    let mut rows: Vec<(usize, Value)> = roster_people(w, t)
        .into_iter()
        .map(|k| (w.people[k].player.position.index(), row(w, k, t, own, reader)))
        .collect();
    rows.sort_by_key(|x| x.0);
    json!({ "team": team_brief(w, t), "mine": own, "rows": rows.into_iter().map(|r| r.1).collect::<Vec<_>>() })
}

pub fn schedule(w: &World, t: TeamRef) -> Value {
    let games: Vec<Value> = match t {
        TeamRef::College(i) => {
            let mut g = w.college_fixtures(Some(i));
            g.sort_by_key(|g| g.date);
            g.into_iter()
                .map(|g| {
                    let home = g.home == i;
                    let opp = if home { g.away } else { g.home };
                    let called_off = g.cancelled.then(|| "Called off".to_string());
                    let result = g.score.map(|s| {
                        let (me, them) = if home { (s[0], s[1]) } else { (s[1], s[0]) };
                        format!(
                            "{} {}-{}",
                            if me > them {
                                "W"
                            } else if me < them {
                                "L"
                            } else {
                                "T"
                            },
                            me,
                            them
                        )
                    });
                    json!({
                        "date": date_text(g.date),
                        "kick": format!("{}:{:02}", g.kickoff.0, g.kickoff.1),
                        "opp": team_brief(w, TeamRef::College(opp)),
                        "at": if g.neutral { "vs (n)" } else if home { "vs" } else { "at" },
                        "name": g.name,
                        "conf": g.conference,
                        "why": g.why,
                        "result": result.or(called_off),
                        "line": line_json(g.line, home, g.score, g.attendance),
                    })
                })
                .collect()
        }
        TeamRef::Nfl(i) => w
            .nfl_fixtures(Some(i))
            .into_iter()
            .map(|g| {
                let home = g.home == i;
                let opp = if home { g.away } else { g.home };
                // The game's score, or your kept game page's for a game
                // from before scores were kept per game.
                let score = g.score.or_else(|| {
                    let d = g.when?;
                    w.seat_games()
                        .iter()
                        .find(|k| k.level == CalLevel::Nfl && k.date == d && k.teams == [g.home, g.away])
                        .map(|k| k.stats.score.map(|x| x.max(0) as u32))
                });
                let result = match score {
                    Some([a, b]) => {
                        let (me, them) = if home { (a, b) } else { (b, a) };
                        Some(format!(
                            "{} {me}-{them}",
                            if me > them {
                                "W"
                            } else if me < them {
                                "L"
                            } else {
                                "T"
                            }
                        ))
                    }
                    None if g.played => Some("Played".to_string()),
                    None => None,
                };
                json!({
                    "week": g.week,
                    "date": g.when.map(date_text),
                    "kick": g.minute.map(|m| format!("{}:{:02} ET", m / 60, m % 60)),
                    "opp": team_brief(w, TeamRef::Nfl(opp)),
                    "at": if home { "vs" } else { "at" },
                    "result": result,
                    "line": line_json(g.line, home, score, g.attendance),
                })
            })
            .collect(),
        TeamRef::HighSchool(i) => {
            // Played games from the kept game pages (your school's are all
            // kept), then the regular season still to come.
            let season_start = w.hs_fixtures(i).first().map(|f| f.date);
            let mut rows: Vec<(Date, Value)> = w
                .seat_games()
                .iter()
                .filter(|k| k.teams.contains(&i) && season_start.is_none_or(|d| k.date >= d.add(-21)))
                .map(|k| {
                    let home = k.teams[0] == i;
                    let opp = if home { k.teams[1] } else { k.teams[0] };
                    let [a, b] = k.stats.score;
                    let (me, them) = if home { (a, b) } else { (b, a) };
                    let res = format!("{} {me}-{them}", if me > them { "W" } else if me < them { "L" } else { "T" });
                    (k.date, json!({ "date": date_text(k.date), "kick": "19:00", "opp": team_brief(w, TeamRef::HighSchool(opp)), "at": if home { "vs" } else { "at" }, "result": res }))
                })
                .collect();
            for g in w.hs_fixtures(i).into_iter().filter(|g| g.date >= w.today) {
                if rows.iter().any(|r| r.0 == g.date) {
                    continue;
                }
                let home = g.home == i;
                let opp = if home { g.away } else { g.home };
                rows.push((g.date, json!({ "date": date_text(g.date), "kick": "19:00", "opp": team_brief(w, TeamRef::HighSchool(opp)), "at": if home { "vs" } else { "at" }, "result": null })));
            }
            rows.sort_by_key(|r| r.0);
            rows.into_iter().map(|r| r.1).collect()
        }
    };
    let kept: Vec<Value> = w
        .seat_games()
        .iter()
        .enumerate()
        .rev()
        .map(|(k, g)| json!({ "index": k, "date": date_text(g.date), "home": g.names[0], "away": g.names[1], "score": g.stats.score }))
        .collect();
    json!({ "team": team_brief(w, t), "games": games, "box_scores": if is_mine(w, t) { kept } else { Vec::new() } })
}

/// A game's betting line from one side (C8): the spread as that team
/// sees it ("-3.5" favored, "+3.5" the underdog, "Pick"), the total, how
/// the game went against it, and the crowd once played.
pub(crate) fn line_json(line: Option<[f32; 2]>, home: bool, score: Option<[u32; 2]>, crowd: u32) -> Value {
    let Some([spread, total]) = line else {
        return if crowd > 0 { json!({ "crowd": crowd }) } else { Value::Null };
    };
    let mine = if home { spread } else { -spread };
    let text = if mine == 0.0 { "Pick".to_string() } else { format!("{mine:+.1}") };
    let against = score.map(|[h, a]| {
        let margin = if home { h as f32 - a as f32 } else { a as f32 - h as f32 };
        let cover = margin + mine;
        let ou = (h + a) as f32;
        let side = if ou > total {
            "over"
        } else if ou < total {
            "under"
        } else {
            "push on the total"
        };
        let ats = if cover > 0.0 {
            "Covered"
        } else if cover < 0.0 {
            "Didn't cover"
        } else {
            "Push"
        };
        format!("{ats}, {side}")
    });
    json!({ "spread": text, "total": total, "against": against, "crowd": if crowd > 0 { Some(crowd) } else { None } })
}

pub fn standings(w: &World, args: &Value) -> Value {
    // The page names conferences as this world shows them (fictional in a
    // fictional league); the programs keep the real names.
    let shown = args.get("conference").and_then(Value::as_str);
    let real = shown.and_then(|s| {
        w.programs.iter().map(|p| &p.conference).find(|c| w.conference_name(c) == s || c.as_str() == s).cloned()
    });
    let conf = real.or_else(|| match mine(w) {
        Some(TeamRef::College(p)) => Some(w.programs[p].conference.clone()),
        _ => None,
    });
    if matches!(mine(w), Some(TeamRef::Nfl(_))) || args.get("kind").and_then(Value::as_str) == Some("nfl") {
        let Some((cf, dv)) = w.nfl_alignment() else { return json!({ "groups": [] }) };
        let names =
            ["AFC East", "AFC North", "AFC South", "AFC West", "NFC East", "NFC North", "NFC South", "NFC West"];
        let groups: Vec<Value> = (0..8)
            .map(|d| {
                let mut teams: Vec<(usize, (u32, u32, u32))> =
                    (0..cf.len()).filter(|&t| cf[t] * 4 + dv[t] % 4 == d || dv[t] == d).map(|t| (t, w.nfl_record(t))).collect();
                teams.dedup_by_key(|x| x.0);
                teams.sort_by_key(|x| std::cmp::Reverse(x.1.0 * 2 + x.1.2));
                json!({ "name": names[d], "teams": teams.iter().map(|(t, r)| json!({ "team": team_brief(w, TeamRef::Nfl(*t)), "w": r.0, "l": r.1, "t": r.2 })).collect::<Vec<_>>() })
            })
            .collect();
        return json!({ "groups": groups });
    }
    if let Some(TeamRef::HighSchool(h)) = mine(w) {
        let Some((name, rows)) = w.hs_standings(h) else { return json!({ "groups": [] }) };
        let teams: Vec<Value> = rows
            .iter()
            .map(|&(s, wn, l)| json!({ "team": team_brief(w, TeamRef::HighSchool(s)), "w": wn, "l": l, "t": 0 }))
            .collect();
        return json!({ "groups": [{ "name": name, "teams": teams }] });
    }
    let Some(conf) = conf else { return json!({ "groups": [] }) };
    let div = match mine(w) {
        Some(TeamRef::College(p)) => Some(w.programs[p].division),
        _ => None,
    };
    let mut teams: Vec<(usize, afm_engine::world::seat::Record)> = w
        .programs
        .iter()
        .filter(|p| p.conference == conf && div.is_none_or(|d| p.division == d))
        .map(|p| (p.id, w.college_record(p.id)))
        .collect();
    teams.sort_by(|a, b| (b.1.conf_wins, b.1.wins, a.1.conf_losses).cmp(&(a.1.conf_wins, a.1.wins, b.1.conf_losses)));
    let mut confs: Vec<String> = w
        .programs
        .iter()
        .filter(|p| div.is_none_or(|d| p.division == d))
        .map(|p| w.conference_name(&p.conference))
        .collect();
    confs.sort();
    confs.dedup();
    json!({
        "conferences": confs,
        "groups": [{ "name": w.conference_name(&conf), "teams": teams.iter().map(|(p, r)| json!({
            "team": team_brief(w, TeamRef::College(*p)),
            "cw": r.conf_wins, "cl": r.conf_losses, "w": r.wins, "l": r.losses,
        })).collect::<Vec<_>>() }],
    })
}

pub(crate) fn reputation_words(r: f64) -> &'static str {
    match r {
        x if x >= 16.0 => "One of the best in the game",
        x if x >= 13.0 => "Highly regarded",
        x if x >= 10.0 => "Respected",
        x if x >= 7.0 => "Little known",
        _ => "Unproven",
    }
}

/// What a coach came up as: his first group, or for you, your road in.
pub(crate) fn came_up(w: &World, c: usize) -> String {
    match &w.career {
        Some(k) if k.coach == c && !k.you.road.is_empty() => k.you.road.clone(),
        _ => w.coaches[c].came_up.coach_title().to_string(),
    }
}

/// A coach's record in the job he holds: a head coach's games there (from
/// the first season of kept records), none for an assistant, whose job has
/// no record of its own.
pub(crate) fn job_record(w: &World, c: usize) -> String {
    let Some(j) = w.coaches[c].job.filter(|j| j.role == Role::HeadCoach) else { return "-".into() };
    let (wn, l) = w.stint_record(c);
    let kept_from = w.options.start_year - w.options.history_years as i32;
    if j.since < kept_from { format!("{wn}-{l} (since {kept_from})") } else { format!("{wn}-{l}") }
}

pub(crate) fn coach_row(w: &World, c: usize, role: &str) -> Value {
    let co = &w.coaches[c];
    json!({
        "id": c,
        "name": co.name,
        "role": role,
        "age": co.age_on(w.today),
        "reputation": reputation_words(co.reputation),
        "came_up": came_up(w, c),
        "record": job_record(w, c),
        "since": co.job.map(|j| j.since),
        "ex_pro": co.ex_pro,
        "you": w.career.as_ref().is_some_and(|k| k.coach == c),
    })
}

pub fn staff(w: &World, t: TeamRef) -> Value {
    let mut rows = Vec::new();
    match t {
        TeamRef::College(i) => {
            let p = &w.programs[i];
            for (c, role) in
                [(p.head_coach, "Head coach"), (p.oc, "Offensive coordinator"), (p.dc, "Defensive coordinator")]
            {
                if let Some(c) = c {
                    rows.push(coach_row(w, c, role));
                }
            }
            for g in Group::ALL {
                if let Some(c) = p.position_coaches[g as usize] {
                    rows.push(coach_row(w, c, g.coach_title()));
                }
            }
        }
        TeamRef::Nfl(i) => {
            let tm = &w.nfl[i];
            for (c, role) in
                [(tm.head_coach, "Head coach"), (tm.oc, "Offensive coordinator"), (tm.dc, "Defensive coordinator")]
            {
                if let Some(c) = c {
                    rows.push(coach_row(w, c, role));
                }
            }
        }
        TeamRef::HighSchool(i) => {
            if let Some(c) = w.high_schools[i].coach {
                rows.push(coach_row(w, c, "Head coach"));
            }
            // Your own assistants: teachers who coach for a stipend.
            if is_mine(w, t)
                && let Some(k) = w.career.as_ref()
            {
                for a in &k.hs_staff {
                    let g = Group::ALL[a.group as usize];
                    let role = match a.coordinator {
                        Some(true) => "Offensive coordinator".to_string(),
                        Some(false) => "Defensive coordinator".to_string(),
                        None => g.coach_title().to_string(),
                    };
                    rows.push(json!({
                        "id": null, "name": a.name, "role": role, "age": a.age_on(w.today),
                        "reputation": "Local",
                        "came_up": a.subject.as_ref().map_or("Walk-on coach (doesn't teach)".to_string(), |s| format!("Teaches {s}")),
                        "record": "", "since": a.since, "pay": a.stipend, "you": false,
                    }));
                }
            }
        }
    }
    json!({ "team": team_brief(w, t), "rows": rows })
}

pub fn coach(w: &World, c: usize) -> Result<Value, String> {
    let co = w.coaches.get(c).ok_or("No such coach.")?;
    let job = co.job.map(|j| {
        let team = employer_brief(w, j.employer);
        let name = employer_name(w, j.employer);
        let record = (j.role == Role::HeadCoach).then(|| job_record(w, c));
        json!({ "team": team["ref"], "name": name, "role": j.role.name(), "since": j.since, "record": record })
    });
    // Records are kept from the first season of recorded history: a job
    // that began before it counts its games from then.
    let kept_from = w.options.start_year - w.options.history_years as i32;
    let history: Vec<Value> = co
        .history
        .iter()
        .rev()
        .map(|s| {
            let name = employer_name(w, s.employer);
            // Only a head coach's job has a record, and only its seasons
            // from the first kept one.
            let record = if s.role != Role::HeadCoach || s.to < kept_from {
                "-".to_string()
            } else if s.from < kept_from {
                format!("{}-{} (since {kept_from})", s.wins, s.losses)
            } else {
                format!("{}-{}", s.wins, s.losses)
            };
            json!({ "where": name, "role": s.role.name(), "from": s.from, "to": s.to, "record": record, "ended": s.ended })
        })
        .collect();
    let mentor = co.mentor.and_then(|m| w.coaches.get(m)).map(|m| json!({ "id": m.id, "name": m.name }));
    Ok(json!({
        "id": c, "name": co.name, "age": co.age_on(w.today), "reputation": reputation_words(co.reputation),
        "came_up": came_up(w, c), "ex_pro": co.ex_pro, "job": job, "history": history,
        "mentor": mentor, "titles": co.career_titles,
        "you": w.career.as_ref().is_some_and(|k| k.coach == c),
        "ats": ats_text(w, c),
    }))
}

/// A head coach's record against the spread (C8), from the games the
/// books hung a line on: "34-28-2 against the spread".
pub(crate) fn ats_text(w: &World, c: usize) -> Option<String> {
    let [cov, miss, push] = afm_engine::world::finance::lines::ats(w, c);
    (cov + miss + push > 0).then(|| {
        let pct = 100.0 * cov as f64 / (cov + miss).max(1) as f64;
        format!("{cov}-{miss}-{push} against the spread ({pct:.0}% covers)")
    })
}

/// Your depth chart: the order the engine will play, position by position,
/// built the way the engine builds it on game day (a two-way high schooler
/// or a corner at receiver shows at the spot he would play), your own order
/// first where you set one. The rest of the position's players follow.
pub fn depth(w: &World) -> Value {
    let Some(t) = mine(w) else { return json!({ "positions": [] }) };
    let people = roster_people(w, t);
    let reader = reader_key(w);
    let game = w.seat_game_depth();
    let by_id = |id: u32| people.iter().copied().find(|&k| w.people[k].player.id == id);
    let positions: Vec<Value> = Position::ALL
        .iter()
        .map(|&pos| {
            let mut own: Vec<usize> = people.iter().copied().filter(|&k| w.people[k].player.position == pos).collect();
            // Without a game team (too few players to dress), the staff's read.
            let eye = eye_for(w, t, pos);
            let read = |k: usize| match reads::overall(&w.people[k].player, true, eye, reader) {
                reads::Read::Range(lo, hi) => lo as f64 + hi as f64,
                reads::Read::NotSeen => 0.0,
            };
            own.sort_by(|&a, &b| read(b).total_cmp(&read(a)));
            let mut list: Vec<usize> = game
                .as_ref()
                .and_then(|g| g.iter().find(|(p, _)| *p == pos))
                .map_or(Vec::new(), |(_, ids)| ids.iter().filter_map(|&id| by_id(id)).collect());
            let dressed = list.len();
            for k in own {
                if !list.contains(&k) {
                    list.push(k);
                }
            }
            let yours = w.seat_depth.iter().any(|(p, _)| *p == pos);
            json!({
                "pos": pos.name(),
                "controls": w.controls(pos),
                "starters": afm_engine::game::roster::starters(pos),
                "set_by_you": yours,
                "listed": dressed,
                "players": list.iter().map(|&k| {
                    let mut v = row(w, k, t, true, reader);
                    v["here"] = json!(w.people[k].player.position == pos);
                    v
                }).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "team": team_brief(w, t), "positions": positions })
}

pub fn set_depth(w: &mut World, args: &Value) -> Result<Value, String> {
    let pos = args.get("pos").and_then(Value::as_str).ok_or("Missing pos.")?;
    let pos = Position::ALL.into_iter().find(|p| p.name() == pos).ok_or("No such position.")?;
    let ids: Vec<u32> = args
        .get("ids")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_u64).map(|v| v as u32).collect())
        .unwrap_or_default();
    // An assistant proposes: the head coach can overrule him.
    let note = w.propose_depth(pos, ids)?;
    let mut v = depth(w);
    v["note"] = json!(note);
    Ok(v)
}

pub fn player(w: &World, id: u32) -> Result<Value, String> {
    let k = person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[k];
    let team = team_of(w, p);
    let own = team.is_some_and(|t| is_mine(w, t));
    let reader = reader_key(w);
    let pos = p.player.position;
    let eye = team.filter(|_| own).map_or(10.0, |t| eye_for(w, t, pos));
    let attrs = reads::attributes(&p.player, own, eye, reader);
    let groups: Vec<Value> = [
        (Category::Physical, "Physical"),
        (Category::Passing, "Passing"),
        (Category::Running, "Running"),
        (Category::Receiving, "Receiving"),
        (Category::LinePlay, "Line play"),
        (Category::Defense, "Defense"),
        (Category::KickingAndSnapping, "Kicking and snapping"),
        (Category::Mental, "Mental"),
    ]
    .iter()
    .map(|&(cat, name)| {
        json!({
            "name": name,
            "attrs": attrs.iter().filter(|(a, _)| a.category() == cat).map(|(a, r)| json!({ "key": a.key(), "label": a.label(), "read": r.json() })).collect::<Vec<_>>(),
        })
    })
    .collect();
    let level = level_of(w, p);
    let potential = if own { reads::potential_words(&p.player, eye, level, reader, w.year) } else { Value::Null };
    // Each season's team: his high school, his college in the order of his
    // college seasons (the coaches' book has one row a season), his NFL club.
    let college = |l: Level| matches!(l, Level::FBS | Level::FCS | Level::SmallCollege);
    let college_rows = p.player.history.iter().filter(|h| college(h.level)).count();
    let mut k = 0;
    let mut history: Vec<Value> = p
        .player
        .history
        .iter()
        .map(|h| {
            let team = match h.level {
                Level::HS => w.high_schools.get(p.hs).map(|s| w.school_name(s.id).to_string()),
                // A pro season with no club: unsigned all year.
                Level::NFL => Some(h.team.map_or("Unsigned".to_string(), |t| w.nfl_team_name(t))),
                _ => {
                    k += 1;
                    (p.college_staff.len() == college_rows)
                        .then(|| p.college_staff.get(k - 1).map(|s| w.program_name(s.1).to_string()))
                        .flatten()
                }
            };
            json!({
                "age": h.age,
                "level": level_words(h.level),
                "pos": h.position.name(),
                "role": role_words(h.playing_time),
                "missed": h.games_missed,
                "team": team,
                "events": h.events.iter().map(|e| e.name()).collect::<Vec<_>>(),
            })
        })
        .collect();
    history.reverse();
    let notes: Vec<Value> = w
        .notes_on(id)
        .iter()
        .enumerate()
        .rev()
        .map(|(i, n)| json!({ "k": i, "date": date_text(n.date), "text": n.text }))
        .collect();
    let pro = p.pro.as_ref().map(|c| {
        json!({
            "team": c.team.map(|t| w.nfl_team_name(t)),
            "accrued": c.accrued,
            "draft": c.draft.map(|(y, r, pk, t)| format!("{y} round {r}, pick {pk} by {}", w.nfl_team_name(t))),
            "contract": c.contract.as_ref().map(|k| json!({
                "years": k.real_years(), "total": k.total().round(), "apy": k.apy().round(),
                "guaranteed": k.guaranteed().round(), "through": k.last_year(),
                "cap_hit": k.cap_hit(w.year).round(),
            })),
            "earnings": c.earnings.round(),
        })
    });
    let offers: Vec<Value> = offers_of(p).iter().map(|&o| json!(w.program_name(o))).collect();
    let tendencies: Vec<Value> = if own {
        afm_engine::tendencies::Tend::ALL
            .iter()
            .map(|&t| {
                let v = p.player.tend.get(t);
                let r = reads::range(v * 99.0 / 20.0, 6.0, reader, id, 200 + t.index());
                json!({ "label": t.label(), "read": r.json() })
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(json!({
        "id": id,
        "name": p.name,
        "pos": pos.name(),
        "positions": p.player.position_history.iter().map(|(a, ps)| json!([a, ps.name()])).collect::<Vec<_>>(),
        "age": p.age_on(w.today),
        "born": born_text(p.born),
        "class": class_text(w, p),
        // A player in another league links to his club there.
        "team": team.map(|t| team_brief(w, t)).or_else(|| {
            matches!(p.pro.as_ref().map(|c| c.status), Some(afm_engine::world::pro::Status::Other(_)))
                .then(|| w.others.team_of(k))
                .flatten()
                .map(|fr| other_brief(w, fr))
        }),
        "where": where_now(w, p),
        "followers": (p.followers > 0).then_some(p.followers),
        "brand": (p.brand >= 1000.0).then_some((p.brand as f64 / 1000.0).round() * 1000.0),
        "on_camera": afm_engine::world::fame::aura_read(p, if own { 13.0 } else { 9.0 }, reader),
        "mine": own,
        "home": hometown(w, p),
        "high_school": w.high_schools.get(p.hs).filter(|_| !w.others.national(p.player.id)).map(|s| w.school_name(s.id).to_string()),
        "high_school_id": w.high_schools.get(p.hs).filter(|_| !w.others.national(p.player.id)).map(|_| p.hs),
        "national": w.others.nationals.get(&p.player.id).map(|&s| afm_engine::world::pro::other::U_SPORTS.get(s as usize).copied().unwrap_or("")),
        "ht": ft_in(p.player.body.height),
        "wt": p.player.body.weight.round(),
        "arm": p.player.body.arm,
        "hand": p.player.body.hand,
        "stars": p.stars,
        "status": status_text(w, p),
        "mood": mood_words(p),
        "ovr": reads::overall(&p.player, own, eye, reader).json(),
        "potential": potential,
        "groups": groups,
        "tendencies": tendencies,
        "links": reads::links(&p.player, own, p.stage == Stage::Pro || p.cal.combine_invite, eye, reader),
        "history": history,
        "pro": pro,
        "offers": offers,
        "notes": notes,
        "stories": p.notes.iter().rev().take(20).collect::<Vec<_>>(),
    }))
}

/// A date of birth as the player page writes it: "14 March 2009".
fn born_text(d: Date) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    (d != afm_engine::calendar::born::UNKNOWN).then(|| {
        let (y, m, day) = d.parts();
        format!("{day} {} {y}", MONTHS[m as usize - 1])
    })
}

pub(crate) fn team_of(w: &World, p: &Person) -> Option<TeamRef> {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => Some(TeamRef::HighSchool(p.hs)),
        Stage::College => p.college.map(TeamRef::College),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map(TeamRef::Nfl),
        _ => None,
    }
    .filter(|t| match t {
        TeamRef::HighSchool(i) => *i < w.high_schools.len(),
        _ => true,
    })
}

pub fn my_games(w: &World) -> Value {
    let games: Vec<Value> = w
        .seat_games()
        .iter()
        .enumerate()
        .rev()
        .map(|(k, g)| json!({ "index": k, "date": date_text(g.date), "home": g.names[0], "away": g.names[1], "score": g.stats.score }))
        .collect();
    json!({ "games": games })
}

/// A whole game of yours: the box score, each player's line and the
/// play-by-play with the play callers' reasons.
pub fn game(w: &World, k: usize) -> Result<Value, String> {
    let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
    let name = |id: u32| -> String { person_index(w, id).map_or(format!("#{id}"), |i| w.people[i].name.clone()) };
    let s = &g.stats;
    let top = possession(s);
    let mut sides = Vec::new();
    for (side, &top) in top.iter().enumerate() {
        let t = &s.team[side];
        let mut passing = Vec::new();
        let mut rushing = Vec::new();
        let mut receiving = Vec::new();
        let mut defense = Vec::new();
        for (&id, l) in &s.players[side] {
            if l.att > 0 {
                passing.push(json!({ "name": name(id), "id": id, "cmp": l.cmp, "att": l.att, "yds": l.pass_yds.round(), "td": l.pass_td, "int": l.int, "sk": l.sacked }));
            }
            if l.rush > 0 {
                rushing.push(json!({ "name": name(id), "id": id, "car": l.rush, "yds": l.rush_yds.round(), "td": l.rush_td, "long": l.rush_long.round() }));
            }
            if l.targets > 0 {
                receiving.push(json!({ "name": name(id), "id": id, "rec": l.rec, "tgt": l.targets, "yds": l.rec_yds.round(), "td": l.rec_td }));
            }
            if l.tackles + l.assists > 0 || l.sacks > 0.0 || l.tfl > 0.0 || l.ints > 0 || l.pbus > 0 {
                defense.push(json!({ "name": name(id), "id": id, "tkl": l.tackles + l.assists, "tfl": l.tfl, "sk": l.sacks, "int": l.ints, "pd": l.pbus }));
            }
        }
        fn by(v: &mut [Value], k: &str) {
            v.sort_by(|a, b| b[k].as_f64().unwrap_or(0.0).total_cmp(&a[k].as_f64().unwrap_or(0.0)));
        }
        by(&mut passing, "att");
        by(&mut rushing, "yds");
        by(&mut receiving, "yds");
        by(&mut defense, "tkl");
        sides.push(json!({
            "name": g.names[side],
            "score": s.score[side],
            "totals": {
                "plays": t.plays, "yards": t.yards.round(), "pass": t.pass_yards.round(), "rush": t.rush_yards.round(),
                "first_downs": t.first_downs, "turnovers": t.turnovers, "penalties": t.penalties, "pen_yds": t.penalty_yds.round(),
                "third": format!("{}/{}", t.third_conv, t.third_att), "top": format!("{}:{:02}", (top / 60.0) as u32, (top % 60.0) as u32),
            },
            "passing": passing, "rushing": rushing, "receiving": receiving, "defense": defense,
        }));
    }
    let plays: Vec<Value> = s
        .plays
        .iter()
        .map(|r| {
            use afm_engine::game::stats::PlayType;
            // Kicks after a score have no down.
            let downless = matches!(r.kind, PlayType::Kickoff | PlayType::ExtraPoint | PlayType::TwoPoint);
            json!({
                "q": r.quarter,
                "clock": format!("{}:{:02}", (r.clock / 60.0) as u32, (r.clock % 60.0) as u32),
                "off": g.names[r.offense.min(1)],
                "down": if downless { 0 } else { r.down },
                // Goal to go, inches, or whole yards (never "& 0").
                "togo": if r.togo >= r.yardline_100 - 0.5 {
                    "Goal".to_string()
                } else if r.togo < 0.75 {
                    "inches".to_string()
                } else {
                    format!("{}", r.togo.round().max(1.0))
                },
                "spot": spot_text(r.yardline_100),
                "text": play_text(r, &name),
                "call": r.call,
                "why": r.call_why,
                "score": r.score,
            })
        })
        .collect();
    // The line the books hung on it and the crowd (C8, later extras topic 1).
    let (spread, crowd) = kept_line(w, g);
    let score = [s.score[0].max(0) as u32, s.score[1].max(0) as u32];
    let mut line = line_json(spread, true, Some(score), crowd);
    if let Some(l) = spread {
        line["words"] = json!(afm_engine::world::finance::lines::words(l, &g.names[0], &g.names[1]));
    }
    Ok(json!({
        "date": date_text(g.date), "quarters": s.quarters, "overtime": s.overtime, "sides": sides, "plays": plays,
        "line": line,
    }))
}

/// A kept game's line (home side's spread first) and its crowd.
fn kept_line(w: &World, g: &afm_engine::world::seat::SeatGame) -> (Option<[f32; 2]>, u32) {
    let [h, a] = g.teams;
    match g.level {
        CalLevel::College => w
            .college_fixtures(Some(h))
            .into_iter()
            .find(|f| f.date == g.date && f.home == h && f.away == a)
            .map_or((None, 0), |f| (f.line, f.attendance)),
        CalLevel::Nfl => w
            .nfl_fixtures(Some(h))
            .into_iter()
            .find(|f| f.when == Some(g.date) && f.home == h && f.away == a)
            .map_or((None, 0), |f| (f.line, f.attendance)),
        _ => (None, 0),
    }
}

pub(crate) fn spot_text(to_goal: f64) -> String {
    let y = to_goal.round() as i32;
    if y == 50 {
        "50".into()
    } else if y > 50 {
        format!("OWN {}", 100 - y)
    } else {
        format!("OPP {y}")
    }
}

/// Time of possession from the game clock: each snap's team holds the ball
/// until the next snap (or the end of the quarter), a kickoff's receiving
/// team from the kick. The engine's own total counts only the seconds the
/// ball is live.
pub(crate) fn possession(s: &afm_engine::game::stats::GameStats) -> [f64; 2] {
    use afm_engine::game::stats::PlayType;
    let mut top = [0.0; 2];
    for (i, r) in s.plays.iter().enumerate() {
        if r.offense > 1 {
            continue;
        }
        let end = match s.plays.get(i + 1) {
            Some(n) if n.quarter == r.quarter => n.clock,
            _ => 0.0,
        };
        // A kickoff's seconds belong to the team that receives it.
        let side = if matches!(r.kind, PlayType::Kickoff) { 1 - r.offense } else { r.offense };
        top[side] += (r.clock - end).max(0.0);
    }
    top
}

pub(crate) fn play_text(r: &afm_engine::game::stats::PlayRecord, name: &dyn Fn(u32) -> String) -> String {
    use afm_engine::game::stats::PlayType;
    let yds = r.yards.round() as i32;
    let gain = |y: i32| {
        if y == 0 {
            "no gain".to_string()
        } else if y == -1 {
            "a loss of 1".to_string()
        } else if y < 0 {
            format!("a loss of {}", -y)
        } else if y == 1 {
            "1 yard".to_string()
        } else {
            format!("{y} yards")
        }
    };
    let mut t = match r.kind {
        PlayType::Pass => {
            let qb = r.passer.map(name).unwrap_or_default();
            let to = r.target.map(name).unwrap_or_default();
            if r.throw_away && !r.interception {
                format!("{qb} throws it away")
            } else if r.interception {
                let by = r.interceptor.map(|i| format!(" by {}", name(i))).unwrap_or_default();
                format!("{qb} pass intended for {to} is INTERCEPTED{by}")
            } else if r.complete {
                format!("{qb} pass to {to} ({}) for {}", r.route, gain(yds))
            } else {
                format!("{qb} pass incomplete, intended for {to}")
            }
        }
        PlayType::Run => format!("{} runs for {}", r.carrier.map(name).unwrap_or_default(), gain(yds)),
        PlayType::Scramble => {
            format!("{} scrambles for {}", r.carrier.or(r.passer).map(name).unwrap_or_default(), gain(yds))
        }
        PlayType::Sack => {
            let by: Vec<String> =
                if r.sackers.is_empty() { &r.tacklers } else { &r.sackers }.iter().map(|&i| name(i)).collect();
            let by = if by.is_empty() { String::new() } else { format!(" by {}", by.join(" and ")) };
            format!("{} sacked{by} for {}", r.passer.map(name).unwrap_or_default(), gain(yds))
        }
        PlayType::Kickoff | PlayType::Punt => {
            let what = if r.kind == PlayType::Punt { "punts" } else { "kicks off" };
            let k = r.kicker.map(name).unwrap_or_default();
            let dist = r.kick_distance.round() as i32;
            if r.blocked {
                format!("{k} {what}: BLOCKED")
            } else if r.touchback {
                format!("{k} {what} {dist} yards, touchback")
            } else if r.fair_catch {
                format!("{k} {what} {dist} yards, fair catch by {}", r.returner.map(name).unwrap_or_default())
            } else if let Some(ret) = r.returner {
                format!("{k} {what} {dist} yards, {} returns it {}", name(ret), gain(r.return_yards.round() as i32))
            } else {
                format!("{k} {what} {dist} yards")
            }
        }
        PlayType::FieldGoal => {
            let k = r.kicker.map(name).unwrap_or_default();
            let dist = r.kick_distance.round() as i32;
            let res = if r.blocked {
                "BLOCKED"
            } else if r.kick_good {
                "good"
            } else {
                "no good"
            };
            format!("{k} {dist}-yard field goal is {res}")
        }
        PlayType::ExtraPoint => {
            let k = r.kicker.map(name).unwrap_or_default();
            let res = if r.blocked {
                "BLOCKED"
            } else if r.kick_good {
                "good"
            } else {
                "no good"
            };
            format!("{k} extra point is {res}")
        }
        PlayType::NoPlay => {
            let fouls: Vec<String> = r
                .penalties
                .iter()
                .map(|(f, _, id, y, _)| format!("{} on {}, {} yards", f.name(), name(*id), y.round()))
                .collect();
            format!("Penalty before the snap: {}", fouls.join("; "))
        }
        PlayType::TwoPoint => {
            // The try as the play it was, then whether it counted.
            let mut snap = r.clone();
            snap.kind = if !r.sackers.is_empty() {
                PlayType::Sack
            } else if r.target.is_some() || r.throw_away || r.interception {
                PlayType::Pass
            } else if r.carrier.is_some() && r.carrier == r.passer {
                PlayType::Scramble
            } else {
                PlayType::Run
            };
            snap.touchdown = false;
            snap.first_down = false;
            snap.turnover = false;
            snap.fumbled = None;
            snap.penalties.clear();
            let what = play_text(&snap, name);
            format!("Two-point try: {what}, {}", if r.touchdown { "GOOD" } else { "no good" })
        }
        other => format!("{other:?}: {}", gain(yds)),
    };
    if r.nullified {
        t = format!("No play ({t})");
    }
    if let Some(f) = r.fumbled {
        let forced = r.forced_fumble.map(|by| format!(", forced by {}", name(by))).unwrap_or_default();
        let fell = if r.fumble_lost { "the defense recovers" } else { "his team keeps it" };
        t.push_str(&format!(". {} FUMBLES{forced}, {fell}", name(f)));
    }
    if r.touchdown && r.kind != PlayType::TwoPoint {
        t.push_str(if r.defense_td { ", returned for a TOUCHDOWN" } else { ", TOUCHDOWN" });
    }
    if r.safety {
        t.push_str(", SAFETY");
    }
    if r.kind != PlayType::NoPlay {
        for (f, _, id, y, accepted) in &r.penalties {
            t.push_str(&format!(
                ". Flag: {} on {}, {} yards{}",
                f.name(),
                name(*id),
                y.round(),
                if *accepted { "" } else { ", declined" }
            ));
        }
    }
    // A missed field goal or a punt hands the ball over without being a
    // turnover; a snap that does is one on downs.
    let scrimmage = matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble);
    if r.turnover && scrimmage && !r.interception && r.fumbled.is_none() && !r.defense_td {
        t.push_str(if r.down == 4 { ", turnover on downs" } else { ", turnover" });
    }
    if r.first_down && !r.touchdown {
        t.push_str(", first down");
    }
    t
}

pub fn search(w: &World, q: &str) -> Value {
    let q = q.trim().to_lowercase();
    if q.len() < 2 {
        return json!({ "players": [], "teams": [], "coaches": [] });
    }
    // Your own players first, then players still in football, then the
    // closest names: a surname or a first name that starts with what you
    // typed before one that only contains it.
    let mine = mine(w).map(|t| roster_people(w, t)).unwrap_or_default();
    let close = |name: &str| {
        let n = name.to_lowercase();
        if n == q {
            0
        } else if n.split_whitespace().any(|part| part.starts_with(&q)) || n.starts_with(&q) {
            1
        } else {
            2
        }
    };
    let mut found: Vec<(u8, u8, u8, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.name.to_lowercase().contains(&q))
        .map(|(k, p)| (!mine.contains(&k) as u8, (p.stage == Stage::Done) as u8, close(&p.name), k))
        .collect();
    found.sort();
    let players: Vec<Value> = found
        .iter()
        .take(60)
        .map(|&(_, _, _, k)| {
            let p = &w.people[k];
            json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "age": p.age_on(w.today), "where": where_now(w, p), "level": if p.stage == Stage::Done { "Retired" } else { level_words(level_of(w, p)) } })
        })
        .collect();
    let brief = |t: TeamRef| {
        let mut v = team_brief(w, t);
        v["sub"] = json!(match t {
            TeamRef::Nfl(_) => "NFL".to_string(),
            TeamRef::College(i) => {
                let p = &w.programs[i];
                format!(
                    "{} · {} · {}, {}",
                    p.division.name(),
                    w.conference_name(&p.conference),
                    p.city,
                    w.states[p.state].code
                )
            }
            TeamRef::HighSchool(i) => {
                let s = &w.high_schools[i];
                format!("High school · {}, {}", s.town, w.states[s.state].code)
            }
        });
        v
    };
    let mut teams: Vec<Value> = (0..w.nfl.len())
        .filter(|&t| w.nfl_team_name(t).to_lowercase().contains(&q))
        .map(|t| brief(TeamRef::Nfl(t)))
        .collect();
    teams.extend(
        w.programs
            .iter()
            .filter(|p| w.program_name(p.id).to_lowercase().contains(&q))
            .take(20)
            .map(|p| brief(TeamRef::College(p.id))),
    );
    teams.extend(
        w.high_schools
            .iter()
            .filter(|s| w.school_name(s.id).to_lowercase().contains(&q))
            .take(20)
            .map(|s| brief(TeamRef::HighSchool(s.id))),
    );
    let coaches: Vec<Value> = w
        .coaches
        .iter()
        .filter(|c| !c.retired && c.name.to_lowercase().contains(&q))
        .take(20)
        .map(|c| {
            let at = c.job.map(|j| employer_name(w, j.employer));
            json!({ "id": c.id, "name": c.name, "role": c.job.map(|j| j.role.name()), "at": at })
        })
        .collect();
    json!({ "players": players, "teams": teams, "coaches": coaches })
}

/// Your own profile: you see your own ratings (people topic 15).
pub fn you(w: &World) -> Value {
    let Some(c) = &w.career else { return Value::Null };
    let co = &w.coaches[c.coach];
    let Some(ab) = w.your_ability() else { return Value::Null };
    let y = &c.you;
    let (wn, l) = w.career_record(c.coach);
    // A photo is shown only if it is a plain JPEG (a shared save can't
    // carry anything else into the page).
    let mut profile = serde_json::to_value(&y.profile).unwrap_or(Value::Null);
    if !y.profile.photo.as_deref().is_some_and(crate::setup::photo_ok) {
        profile["photo"] = Value::Null;
    }
    json!({
        "name": y.name, "age": co.age_on(w.today), "reputation": reputation_words(co.reputation),
        "record": format!("{wn}-{l}"), "titles": co.career_titles,
        "started": date_text(c.started), "road": y.road,
        "teaching": Group::ALL.iter().map(|g| json!({ "group": g.coach_title(), "value": (ab.teaching[*g as usize] * 10.0).round() / 10.0 })).collect::<Vec<_>>(),
        "scheme": ab.scheme, "game_management": ab.game_management, "motivation": ab.motivation,
        "discipline": ab.discipline, "recruiting": ab.recruiting, "network": ab.network,
        "relationships": ab.relationships,
        "extra": w.your_extra().map(|x| json!({
            "play_calling": x.play_calling, "adaptation": x.adaptation,
            "evaluation": x.evaluation, "negotiating": x.negotiating,
            "calls": calls_text(x.calls_offense, x.calls_defense),
        })),
        "profile": profile,
        "home_state": w.states.get(y.home_state).map(|s| state_name(&s.code)),
        "style": { "risk": y.style.risk, "freedom": y.style.freedom, "physical": y.style.physical, "ball_hunting": y.style.ball_hunting, "mould": y.style.mould },
        "history": coach(w, c.coach).map(|v| v["history"].clone()).unwrap_or(Value::Null),
        "title": crate::lane_frontoffice::setup::title_line(w),
        "family": crate::lane_frontoffice::setup::family_json(w),
        "known": crate::lane_frontoffice::setup::known_json(w, 12),
    })
}

/// Who calls your plays, in words.
pub(crate) fn calls_text(offense: bool, defense: bool) -> &'static str {
    match (offense, defense) {
        (true, true) => "Both sides",
        (true, false) => "The offense",
        (false, true) => "The defense",
        (false, false) => "Your coordinators call both sides",
    }
}

#[allow(dead_code)]
fn role_name(r: Role) -> &'static str {
    r.name()
}

/// Training (section 6b topic 5): each of your players and his plan's
/// focus, with the skills his position uses most to pick from.
pub fn training(w: &World) -> Value {
    let Some(t) = mine(w) else { return json!({ "rows": [] }) };
    let reader = reader_key(w);
    let mut rows: Vec<(usize, Value)> = roster_people(w, t)
        .into_iter()
        .map(|k| {
            let p = &w.people[k];
            let mut v = row(w, k, t, true, reader);
            let pos = p.player.position;
            let mut opts: Vec<(f64, afm_engine::attributes::Attr)> = afm_engine::attributes::Attr::ALL
                .iter()
                .map(|&a| (afm_engine::positions::weight(pos, a), a))
                .filter(|x| x.0 > 0.0)
                .collect();
            opts.sort_by(|a, b| b.0.total_cmp(&a.0));
            v["focus"] = json!(w.focus_of(p.player.id).map(|a| a.key()));
            // An assistant plans only his own job's players (lane 10).
            v["may_plan"] = json!(w.may_plan(p.player.id));
            v["options"] = json!(
                opts.iter().take(14).map(|(_, a)| json!({ "key": a.key(), "label": a.label() })).collect::<Vec<_>>()
            );
            (pos.index(), v)
        })
        .collect();
    rows.sort_by_key(|x| x.0);
    json!({ "team": team_brief(w, t), "rows": rows.into_iter().map(|r| r.1).collect::<Vec<_>>() })
}

pub fn set_focus(w: &mut World, args: &Value) -> Result<Value, String> {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    let focus = match args.get("key").and_then(Value::as_str) {
        Some(k) if !k.is_empty() => {
            Some(afm_engine::attributes::Attr::ALL.into_iter().find(|a| a.key() == k).ok_or("No such skill.")?)
        }
        _ => None,
    };
    let t = mine(w).ok_or("No career.")?;
    if !roster_people(w, t).iter().any(|&k| w.people[k].player.id == id) {
        return Err("He isn't on your team.".into());
    }
    if !w.may_plan(id) {
        return Err(format!("His plan is the head coach's. Your job covers {}.", w.your_controls()));
    }
    w.set_focus(id, focus);
    Ok(json!({ "id": id, "focus": focus.map(|a| a.key()) }))
}

/// Recruiting as it stands (the AI's recruiting staff runs it for now):
/// for a college, every high schooler your program has offered, visited,
/// or who committed or signed with you; for a high school, your players
/// with college interest. Your staff knows only what is public about
/// another team's player: stars, body, where he is from.
pub fn recruiting(w: &World) -> Value {
    let Some(t) = mine(w) else { return json!({ "rows": [] }) };
    let reader = reader_key(w);
    let mut rows: Vec<Value> = Vec::new();
    match t {
        TeamRef::College(prog) => {
            for (k, p) in w.people.iter().enumerate() {
                if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
                    continue;
                }
                let signed = p.cal.signed.is_some_and(|s| s.program == prog);
                let committed = p.cal.commit.is_some_and(|c| c.0 == prog);
                let offered = p.cal.offers.contains(&prog) || p.cal.early_offers.iter().any(|o| o.0 == prog);
                let official = p.cal.visits.iter().any(|v| v.0 == prog && v.2 == afm_engine::world::Visit::Official);
                let visited = p.cal.visits.iter().any(|v| v.0 == prog);
                if !(signed || committed || offered || visited) || p.player.age > 19 {
                    continue;
                }
                let status = if signed {
                    "Signed with you"
                } else if committed {
                    "Committed to you"
                } else if offered && official {
                    "Offered, official visit"
                } else if offered {
                    "Offered"
                } else {
                    "Visited"
                };
                let elsewhere = p.cal.commit.filter(|c| c.0 != prog).map(|c| w.program_name(c.0).to_string());
                let mut v = row(w, k, t, false, reader);
                v["recruit"] = json!(status);
                v["rank"] = json!(if signed {
                    0
                } else if committed {
                    1
                } else if official {
                    2
                } else if offered {
                    3
                } else {
                    4
                });
                v["elsewhere"] = json!(elsewhere);
                v["offers"] = json!(offers_of(p).len());
                v["school"] = json!(w.school_name(p.hs));
                rows.push(v);
            }
        }
        TeamRef::HighSchool(h) => {
            for k in roster_people(w, t) {
                let p = &w.people[k];
                if p.cal.offers.is_empty()
                    && p.cal.early_offers.is_empty()
                    && p.cal.commit.is_none()
                    && p.cal.signed.is_none()
                {
                    continue;
                }
                let mut v = row(w, k, t, true, reader);
                let walk = |on: bool| if on { " (walk-on)" } else { "" };
                let status = match (p.cal.signed, p.cal.commit) {
                    (Some(s), _) => format!("Signed with {}{}", w.program_name(s.program), walk(s.walk_on)),
                    (_, Some(c)) => format!("Committed to {}{}", w.program_name(c.0), walk(c.2)),
                    _ => "Uncommitted".to_string(),
                };
                let offers = offers_of(p);
                v["recruit"] = json!(status);
                v["rank"] = json!(if p.cal.signed.is_some() {
                    0
                } else if p.cal.commit.is_some() {
                    1
                } else {
                    3
                });
                v["offers"] = json!(offers.len());
                v["offer_list"] = json!(offers.iter().map(|&o| w.program_name(o).to_string()).collect::<Vec<_>>());
                v["school"] = json!(w.school_name(h));
                rows.push(v);
            }
        }
        TeamRef::Nfl(_) => {}
    }
    json!({ "team": team_brief(w, t), "level": t.json()["kind"], "rows": rows })
}

/// Your club's contracts this league year (the cap manager runs them for now).
pub fn contracts(w: &World) -> Value {
    let Some(t @ TeamRef::Nfl(club)) = mine(w) else { return json!({ "rows": [] }) };
    let reader = reader_key(w);
    let mut total = 0.0;
    let rows: Vec<Value> = roster_people(w, t)
        .into_iter()
        .filter_map(|k| {
            let p = &w.people[k];
            let c = p.pro.as_ref()?.contract.as_ref()?;
            if c.team != club {
                return None;
            }
            let hit = c.cap_hit(w.year);
            total += hit;
            let mut v = row(w, k, t, true, reader);
            v["cap_hit"] = json!(hit.round());
            v["apy"] = json!(c.apy().round());
            v["total"] = json!(c.total().round());
            v["guaranteed"] = json!(c.guaranteed().round());
            v["through"] = json!(c.last_year());
            v["kind"] = json!(format!("{:?}", c.kind));
            // A deal that ran out with last season: he is a free agent
            // when the new league year opens, unless he signs again first.
            v["ending"] = json!(c.last_year() < w.year);
            Some(v)
        })
        .collect();
    let dead: f64 = w.clubs.get(club).map_or(0.0, |c| c.dead_in(w.year));
    // The books as the league keeps them: until the first game only the 51
    // biggest cap hits count (the rule of 51), and last year's unused room
    // carries over.
    use afm_engine::world::pro;
    let counted = pro::payroll(w, club, w.year) - dead;
    let top51 = counted + 0.5 < total;
    let carry = w.clubs.get(club).map_or(0.0, |c| c.carryover);
    json!({
        "team": team_brief(w, t), "rows": rows, "year": w.year,
        "cap": pro::money::price_cap(w.year).round(),
        "carryover": carry.round(),
        "committed": counted.round(), "all": total.round(), "dead": dead.round(),
        "room": (pro::money::price_cap(w.year) + carry - counted - dead).round(),
        "top51": top51,
    })
}
