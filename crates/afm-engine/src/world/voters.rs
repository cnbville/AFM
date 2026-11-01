//! Poll and award voters (section 6 topic 9: "Polls and award voters are
//! media people"). The AP poll's panel of about 62 sportswriters and
//! broadcasters, the Harris Interactive panel the BCS read from 2005 to
//! 2013, the Coaches Poll's rotating panel of 65 FBS head coaches, the
//! Heisman electorate (870 media voters in six regions of 145, the living
//! former winners and one fan ballot) and the playoff selection committee
//! (13 members from 2014). How they vote is in `polls.rs` and, for the
//! Heisman, `college.rs`.
//!
//! Stored compactly: ids, a home state, an outlet index and habits in
//! bytes. A media voter's name is drawn from his id when it is shown, so
//! the panels cost a few kilobytes. Only each voter's latest ballot is
//! kept; nothing is kept per voter for past weeks or seasons.

use super::World;
use super::coach::{Employer, Role};
use super::data::Division;
use super::ethnicity::Ethnicity;
use super::geo;
use crate::rng::{Rng, clamp};

/// The AP panel's size (62 in 2024, 63-65 in other years).
pub const AP_PANEL: usize = 62;
/// The Harris Interactive College Football Poll's panel (114 a season).
pub const HARRIS_PANEL: usize = 114;
/// The Coaches Poll's panel (the AFCA board picks 65 or so a year).
pub const COACHES_PANEL: usize = 65;
/// The playoff selection committee.
pub const COMMITTEE_SIZE: usize = 13;
/// Heisman media voters per region (six regions, 870 in all).
pub const HEISMAN_PER_REGION: usize = 145;

/// The Heisman Trust's six voting regions.
pub const HEISMAN_REGIONS: [&str; 6] = ["Far West", "Mid-Atlantic", "Midwest", "Northeast", "South", "Southwest"];

/// A state's Heisman region (index into `HEISMAN_REGIONS`).
pub fn heisman_region(code: &str) -> u8 {
    match code {
        "AK" | "AZ" | "CA" | "CO" | "HI" | "ID" | "MT" | "NV" | "OR" | "UT" | "WA" | "WY" => 0,
        "DE" | "DC" | "MD" | "NJ" | "PA" | "VA" | "WV" => 1,
        "IL" | "IN" | "IA" | "KS" | "MI" | "MN" | "MO" | "NE" | "ND" | "OH" | "SD" | "WI" => 2,
        "CT" | "ME" | "MA" | "NH" | "NY" | "RI" | "VT" => 3,
        "AL" | "FL" | "GA" | "KY" | "MS" | "NC" | "SC" | "TN" => 4,
        _ => 5, // AR, LA, NM, OK, TX
    }
}

/// Outlets the media voters write or broadcast for: (name, state, first
/// season it existed). Papers, sites and networks of the kind the AP
/// panel really draws on, a few national.
pub const OUTLETS: [(&str, &str, i32); 102] = [
    ("AL.com", "AL", 1998),
    ("The Tuscaloosa News", "AL", 0),
    ("Montgomery Advertiser", "AL", 0),
    ("The Arizona Republic", "AZ", 0),
    ("Arizona Daily Star", "AZ", 0),
    ("Arkansas Democrat-Gazette", "AR", 0),
    ("Los Angeles Times", "CA", 0),
    ("The Orange County Register", "CA", 0),
    ("San Jose Mercury News", "CA", 0),
    ("The Sacramento Bee", "CA", 0),
    ("San Francisco Chronicle", "CA", 0),
    ("The Athletic", "CA", 2016),
    ("Yahoo Sports", "CA", 2000),
    ("The Denver Post", "CO", 0),
    ("The Gazette (Colorado Springs)", "CO", 0),
    ("Hartford Courant", "CT", 0),
    ("ESPN", "CT", 0),
    ("Orlando Sentinel", "FL", 0),
    ("Tampa Bay Times", "FL", 0),
    ("Miami Herald", "FL", 0),
    ("Tallahassee Democrat", "FL", 0),
    ("The Gainesville Sun", "FL", 0),
    ("South Florida Sun Sentinel", "FL", 0),
    ("The Atlanta Journal-Constitution", "GA", 0),
    ("Athens Banner-Herald", "GA", 0),
    ("The Macon Telegraph", "GA", 0),
    ("Honolulu Star-Advertiser", "HI", 2010),
    ("Idaho Statesman", "ID", 0),
    ("Chicago Tribune", "IL", 0),
    ("Chicago Sun-Times", "IL", 0),
    ("The Indianapolis Star", "IN", 0),
    ("South Bend Tribune", "IN", 0),
    ("The Des Moines Register", "IA", 0),
    ("The Gazette (Cedar Rapids)", "IA", 0),
    ("The Wichita Eagle", "KS", 0),
    ("Lawrence Journal-World", "KS", 0),
    ("Lexington Herald-Leader", "KY", 0),
    ("The Courier-Journal", "KY", 0),
    ("The Advocate", "LA", 0),
    ("The Times-Picayune", "LA", 0),
    ("The Baltimore Sun", "MD", 0),
    ("The Boston Globe", "MA", 0),
    ("Detroit Free Press", "MI", 0),
    ("The Detroit News", "MI", 0),
    ("MLive", "MI", 2000),
    ("Star Tribune", "MN", 0),
    ("The Clarion-Ledger", "MS", 0),
    ("The Sun Herald", "MS", 0),
    ("St. Louis Post-Dispatch", "MO", 0),
    ("Columbia Daily Tribune", "MO", 0),
    ("The Kansas City Star", "MO", 0),
    ("Omaha World-Herald", "NE", 0),
    ("Lincoln Journal Star", "NE", 0),
    ("Las Vegas Review-Journal", "NV", 0),
    ("The Star-Ledger", "NJ", 0),
    ("Albuquerque Journal", "NM", 0),
    ("Newsday", "NY", 0),
    ("The Post-Standard", "NY", 0),
    ("The Buffalo News", "NY", 0),
    ("CBS Sports", "NY", 0),
    ("Sports Illustrated", "NY", 0),
    ("The New York Times", "NY", 0),
    ("The Charlotte Observer", "NC", 0),
    ("The News & Observer", "NC", 0),
    ("Winston-Salem Journal", "NC", 0),
    ("The Columbus Dispatch", "OH", 0),
    ("The Plain Dealer", "OH", 0),
    ("The Cincinnati Enquirer", "OH", 0),
    ("The Blade", "OH", 0),
    ("The Oklahoman", "OK", 0),
    ("Tulsa World", "OK", 0),
    ("The Oregonian", "OR", 0),
    ("The Register-Guard", "OR", 0),
    ("Pittsburgh Post-Gazette", "PA", 0),
    ("The Philadelphia Inquirer", "PA", 0),
    ("Centre Daily Times", "PA", 0),
    ("The State", "SC", 0),
    ("The Post and Courier", "SC", 0),
    ("The Greenville News", "SC", 0),
    ("The Tennessean", "TN", 0),
    ("Knoxville News Sentinel", "TN", 0),
    ("The Commercial Appeal", "TN", 0),
    ("The Dallas Morning News", "TX", 0),
    ("Houston Chronicle", "TX", 0),
    ("Austin American-Statesman", "TX", 0),
    ("San Antonio Express-News", "TX", 0),
    ("Fort Worth Star-Telegram", "TX", 0),
    ("El Paso Times", "TX", 0),
    ("Lubbock Avalanche-Journal", "TX", 0),
    ("The Salt Lake Tribune", "UT", 0),
    ("Deseret News", "UT", 0),
    ("Richmond Times-Dispatch", "VA", 0),
    ("The Virginian-Pilot", "VA", 0),
    ("The Roanoke Times", "VA", 0),
    ("USA Today", "VA", 0),
    ("The Seattle Times", "WA", 0),
    ("The Spokesman-Review", "WA", 0),
    ("The Charleston Gazette", "WV", 0),
    ("Milwaukee Journal Sentinel", "WI", 0),
    ("Wisconsin State Journal", "WI", 0),
    ("Casper Star-Tribune", "WY", 0),
    ("The Washington Post", "DC", 0),
];

/// A voter's habits, each stored 0-255 and read 0-1.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Traits {
    /// How much last week's ballot holds him (and last season's finish in
    /// the preseason).
    pub anchor: u8,
    /// How much a famous name sways him.
    pub brand: u8,
    /// Record and who you beat (high) against the eye test and power (low).
    pub resume: u8,
    /// How much margin of victory counts.
    pub margin: u8,
    /// Leaning to his own state, conference or team.
    pub homer: u8,
    /// How closely he watches (low: more noise).
    pub care: u8,
    /// Doubt about Group of Five teams.
    pub g5: u8,
}

impl Traits {
    pub fn f(v: u8) -> f64 {
        v as f64 / 255.0
    }
    fn draw(rng: &mut Rng, means: [f64; 7]) -> Traits {
        let mut b = [0u8; 7];
        for (k, m) in means.iter().enumerate() {
            b[k] = (clamp(rng.normal(*m, 0.18), 0.0, 1.0) * 255.0).round() as u8;
        }
        Traits { anchor: b[0], brand: b[1], resume: b[2], margin: b[3], homer: b[4], care: b[5], g5: b[6] }
    }
    /// Habits read from a key (a coach's), with no stored state.
    pub(crate) fn keyed(seed: u64, id: u64, means: [f64; 7]) -> Traits {
        let mut b = [0u8; 7];
        for (k, m) in means.iter().enumerate() {
            let z = super::season::key_normal(seed, id * 16 + k as u64);
            b[k] = (clamp(m + 0.18 * z, 0.0, 1.0) * 255.0).round() as u8;
        }
        Traits { anchor: b[0], brand: b[1], resume: b[2], margin: b[3], homer: b[4], care: b[5], g5: b[6] }
    }
}

/// AP voters' habit means (anchor, brand, resume, margin, homer, care, G5 doubt).
pub(crate) const AP_MEANS: [f64; 7] = [0.55, 0.4, 0.5, 0.35, 0.4, 0.7, 0.5];
/// The Harris panel: former players, coaches, administrators and media,
/// part-timers who lean on last week.
pub(crate) const HARRIS_MEANS: [f64; 7] = [0.65, 0.45, 0.5, 0.35, 0.45, 0.5, 0.55];
/// Coaches: busy with their own team (little attention), leaning on last
/// week's poll, their own team and conference first.
pub(crate) const COACH_MEANS: [f64; 7] = [0.75, 0.45, 0.55, 0.3, 0.5, 0.35, 0.55];
/// Committee members: whole-season résumés, strength of schedule and game
/// control in `resume` and `margin`, close attention, no anchoring.
pub(crate) const COMMITTEE_MEANS: [f64; 7] = [0.0, 0.15, 0.6, 0.5, 0.2, 0.85, 0.5];

/// A media voter (AP or Harris panel).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MediaVoter {
    /// Draws his name.
    pub id: u32,
    pub born: u16,
    pub ethnicity: Ethnicity,
    pub state: u8,
    /// Index into `OUTLETS`.
    pub outlet: u16,
    /// The first season he voted.
    pub since: u16,
    pub traits: Traits,
    /// His latest ballot this season, best first (program ids).
    #[serde(default)]
    pub ballot: Vec<u16>,
}

/// A seat on the Coaches Poll panel: the program whose head coach votes
/// (an interim head coach takes the ballot when a coach is fired).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CoachSeat {
    pub program: u16,
    /// The coach who held the seat when the panel was picked.
    pub coach: u32,
    pub since: u16,
    #[serde(default)]
    pub ballot: Vec<u16>,
}

/// A Heisman media voter.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct HeismanVoter {
    pub id: u32,
    pub state: u8,
    pub region: u8,
    /// Sway of preseason hype, of stat lines, of team success, of his region.
    pub hype: u8,
    pub stats: u8,
    pub team: u8,
    pub homer: u8,
}

/// A committee member's background (the first committee: five sitting
/// athletic directors, two former coaches, a former player, a former
/// journalist and four former administrators and public figures).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemberRole {
    AthleticDirector,
    FormerCoach,
    FormerPlayer,
    Journalist,
    Administrator,
}

impl MemberRole {
    pub fn name(self) -> &'static str {
        match self {
            MemberRole::AthleticDirector => "athletic director",
            MemberRole::FormerCoach => "former head coach",
            MemberRole::FormerPlayer => "former player",
            MemberRole::Journalist => "former sportswriter",
            MemberRole::Administrator => "former administrator",
        }
    }
}

/// A playoff selection committee member.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Member {
    pub name: String,
    pub role: MemberRole,
    /// The school he works for or played or coached at, shown with his name.
    pub school: Option<u16>,
    /// Schools he leaves the room for (the recusal policy: a school that
    /// pays him, or his own last head coaching job and alma mater).
    pub recused: Vec<u16>,
    /// The world coach he is, for a former coach.
    pub coach: Option<u32>,
    pub since: i32,
    pub term_end: i32,
    pub traits: Traits,
}

/// Every panel, with each voter's latest ballot.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Voters {
    #[serde(default)]
    pub ap: Vec<MediaVoter>,
    #[serde(default)]
    pub harris: Vec<MediaVoter>,
    #[serde(default)]
    pub coaches: Vec<CoachSeat>,
    #[serde(default)]
    pub heisman: Vec<HeismanVoter>,
    #[serde(default)]
    pub committee: Vec<Member>,
    #[serde(default)]
    pub next_id: u32,
    /// The season the panels were last renewed for.
    #[serde(default)]
    pub year: i32,
}

/// Sports media by ethnicity (APSE report cards: about four in five
/// writers and editors white).
fn media_ethnicity(rng: &mut Rng) -> Ethnicity {
    // The Asian share split into its three groups at the end (faces F4).
    rng.weighted(&super::ethnicity::with_asian_split([0.78, 0.13, 0.05, 0.0, 0.04]))
}

/// A media voter's name, drawn from his id each time it is shown.
pub fn media_name(seed: u32, id: u32, born: u16, ethnicity: Ethnicity) -> String {
    let mut rng = Rng::new(seed ^ id.wrapping_mul(0x9E37_79B9) ^ 0x0A9F_0E55);
    super::names::person_name(ethnicity, born as i32, &mut rng)
}

fn fbs_count_by_state(w: &World) -> [f64; 51] {
    let mut n = [0.0; 51];
    for p in w.programs.iter().filter(|p| p.division == Division::FBS && p.fields_games(w.year)) {
        n[p.state.min(50)] += 1.0;
    }
    n
}

fn new_media(w: &World, v: &mut Voters, used: &[u16], rng: &mut Rng, means: [f64; 7]) -> MediaVoter {
    let fbs = fbs_count_by_state(w);
    let year = w.year;
    let picks: Vec<(u16, f64)> = OUTLETS
        .iter()
        .enumerate()
        .filter(|(k, o)| o.2 <= year && !used.contains(&(*k as u16)))
        .map(|(k, o)| {
            let s = geo::state_index(o.1).unwrap_or(0);
            let national = matches!(
                o.0,
                "ESPN"
                    | "CBS Sports"
                    | "Sports Illustrated"
                    | "USA Today"
                    | "The Athletic"
                    | "Yahoo Sports"
                    | "The New York Times"
                    | "The Washington Post"
            );
            (k as u16, if national { 1.5 } else { (fbs[s] + 0.3).sqrt() })
        })
        .collect();
    let outlet = if picks.is_empty() { rng.int(0, OUTLETS.len() as i32 - 1) as u16 } else { rng.weighted(&picks) };
    let state = geo::state_index(OUTLETS[outlet as usize].1).unwrap_or(0) as u8;
    v.next_id += 1;
    MediaVoter {
        id: v.next_id,
        born: (year - rng.int(27, 60)) as u16,
        ethnicity: media_ethnicity(rng),
        state,
        outlet,
        since: year as u16,
        traits: Traits::draw(rng, means),
        ballot: Vec::new(),
    }
}

fn renew_media(w: &World, v: &mut Voters, harris: bool, rng: &mut Rng) {
    let (size, means) = if harris { (HARRIS_PANEL, HARRIS_MEANS) } else { (AP_PANEL, AP_MEANS) };
    let year = w.year;
    let mut panel = std::mem::take(if harris { &mut v.harris } else { &mut v.ap });
    // A few step away each year: retirement, a new beat, the outlet's
    // choice (about 4 of 62 a year).
    panel.retain(|m| {
        let age = year - m.born as i32;
        let p = 0.045 + if age > 63 { 0.3 } else { 0.0 } + if year - m.since as i32 > 20 { 0.04 } else { 0.0 };
        !rng.chance(p)
    });
    while panel.len() < size {
        let used: Vec<u16> = if harris { Vec::new() } else { panel.iter().map(|m| m.outlet).collect() };
        let m = new_media(w, v, &used, rng, means);
        panel.push(m);
    }
    for m in panel.iter_mut() {
        m.ballot.clear();
    }
    *if harris { &mut v.harris } else { &mut v.ap } = panel;
}

fn renew_heisman(w: &World, v: &mut Voters, rng: &mut Rng) {
    let fbs = fbs_count_by_state(w);
    let mut by_region: [Vec<(u8, f64)>; 6] = Default::default();
    for (s, c) in geo::STATE_CENTERS.iter().enumerate() {
        by_region[heisman_region(c.0) as usize].push((s as u8, fbs[s] + 1.0));
    }
    let make = |region: usize, rng: &mut Rng, v: &mut Voters| {
        v.next_id += 1;
        let t = |rng: &mut Rng, m: f64| (clamp(rng.normal(m, 0.18), 0.0, 1.0) * 255.0).round() as u8;
        HeismanVoter {
            id: v.next_id,
            state: rng.weighted(&by_region[region]),
            region: region as u8,
            hype: t(rng, 0.5),
            stats: t(rng, 0.5),
            team: t(rng, 0.5),
            homer: t(rng, 0.4),
        }
    };
    if v.heisman.len() != HEISMAN_PER_REGION * 6 {
        v.heisman.clear();
        for r in 0..6 {
            for _ in 0..HEISMAN_PER_REGION {
                let h = make(r, rng, v);
                v.heisman.push(h);
            }
        }
        return;
    }
    // About one in twenty is replaced each year, in the same region.
    for k in 0..v.heisman.len() {
        if rng.chance(0.05) {
            let r = v.heisman[k].region as usize;
            v.heisman[k] = make(r, rng, v);
        }
    }
}

/// FBS conferences in proportion, for the coaches' panel.
fn renew_coaches(w: &World, v: &mut Voters, rng: &mut Rng) {
    let year = w.year;
    let heads: Vec<(usize, u32, &str)> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS && p.fields_games(year) && !p.roster.is_empty())
        .filter_map(|p| p.head_coach.map(|c| (p.id, c as u32, p.conference.as_str())))
        .collect();
    if heads.is_empty() {
        v.coaches.clear();
        return;
    }
    let size = COACHES_PANEL.min(heads.len());
    // Keep about two in three of last year's panel who still lead the same
    // program; the rest rotate off.
    let mut keep: Vec<CoachSeat> = std::mem::take(&mut v.coaches)
        .into_iter()
        .filter(|s| heads.iter().any(|h| h.0 == s.program as usize && h.1 == s.coach) && rng.chance(0.65))
        .collect();
    let mut confs: Vec<&str> = heads.iter().map(|h| h.2).collect();
    confs.sort();
    confs.dedup();
    for conf in confs {
        let members: Vec<&(usize, u32, &str)> = heads.iter().filter(|h| h.2 == conf).collect();
        let quota = ((size as f64) * members.len() as f64 / heads.len() as f64).round().max(1.0) as usize;
        let mut have = keep.iter().filter(|s| w.programs[s.program as usize].conference == conf).count();
        let mut pool: Vec<&&(usize, u32, &str)> =
            members.iter().filter(|h| !keep.iter().any(|s| s.program as usize == h.0)).collect();
        while have < quota && !pool.is_empty() {
            let k = rng.int(0, pool.len() as i32 - 1) as usize;
            let h = pool.swap_remove(k);
            keep.push(CoachSeat { program: h.0 as u16, coach: h.1, since: year as u16, ballot: Vec::new() });
            have += 1;
        }
    }
    // Rounding: trim or fill to the panel's size.
    while keep.len() > size {
        let k = rng.int(0, keep.len() as i32 - 1) as usize;
        keep.swap_remove(k);
    }
    let mut rest: Vec<&(usize, u32, &str)> =
        heads.iter().filter(|h| !keep.iter().any(|s| s.program as usize == h.0)).collect();
    while keep.len() < size && !rest.is_empty() {
        let k = rng.int(0, rest.len() as i32 - 1) as usize;
        let h = rest.swap_remove(k);
        keep.push(CoachSeat { program: h.0 as u16, coach: h.1, since: year as u16, ballot: Vec::new() });
    }
    keep.sort_by_key(|s| s.program);
    for s in keep.iter_mut() {
        s.ballot.clear();
    }
    v.coaches = keep;
}

pub(crate) fn power_conf(conf: &str, year: i32) -> bool {
    match conf {
        "SEC" | "Big Ten" | "Big 12" | "ACC" => true,
        "Pac-12" | "Pac-10" => year <= 2023,
        "Big East" => year <= 2013,
        "Big Eight" | "SWC" | "Southwest" => true,
        _ => false,
    }
}

/// A Group of Five (before 2014, non-BCS) program: not in a power
/// conference and not Notre Dame.
pub(crate) fn group_of_five(w: &World, prog: usize) -> bool {
    !w.programs[prog].power_conference(w.year)
}

/// A new committee member of a background.
fn new_member(w: &World, v: &Voters, role: MemberRole, rng: &mut Rng) -> Member {
    let year = w.year;
    let taken_school = |s: usize| v.committee.iter().any(|m| m.school == Some(s as u16));
    let mut school = None;
    let mut recused = Vec::new();
    let mut coach = None;
    let mut name = None;
    match role {
        MemberRole::AthleticDirector => {
            // One sitting AD from each power conference first, then from
            // the Group of Five, at schools of standing.
            let have: Vec<&str> = v
                .committee
                .iter()
                .filter(|m| m.role == MemberRole::AthleticDirector)
                .filter_map(|m| m.school.map(|s| w.programs[s as usize].conference.as_str()))
                .collect();
            let fbs: Vec<&super::program::Program> = w
                .programs
                .iter()
                .filter(|p| p.division == Division::FBS && p.fields_games(year) && !taken_school(p.id))
                .collect();
            let missing_power: Vec<&&super::program::Program> = fbs
                .iter()
                .filter(|p| power_conf(&p.conference, year) && !have.contains(&p.conference.as_str()))
                .collect();
            let pool: Vec<(usize, f64)> = if missing_power.is_empty() {
                fbs.iter().filter(|p| !power_conf(&p.conference, year)).map(|p| (p.id, 0.2 + p.prestige)).collect()
            } else {
                missing_power.iter().map(|p| (p.id, 0.2 + p.prestige)).collect()
            };
            if !pool.is_empty() {
                let s = rng.weighted(&pool);
                school = Some(s as u16);
                recused.push(s as u16);
            }
        }
        MemberRole::FormerCoach => {
            // A retired head coach of standing from the world's own coaches.
            let pool: Vec<(usize, f64)> = w
                .coaches
                .iter()
                .filter(|c| c.retired && !v.committee.iter().any(|m| m.coach == Some(c.id as u32)))
                .filter_map(|c| {
                    let fbs_heads = c
                        .history
                        .iter()
                        .filter(|s| {
                            s.role == Role::HeadCoach
                                && matches!(s.employer, Employer::College(p) if w.programs[p].division == Division::FBS)
                        })
                        .count();
                    (fbs_heads > 0 && c.age < 80).then_some((c.id, 0.5 + c.reputation / 4.0))
                })
                .collect();
            if !pool.is_empty() {
                let id = rng.weighted(&pool);
                let c = &w.coaches[id];
                coach = Some(id as u32);
                name = Some(c.name.clone());
                let last = c.history.iter().rev().find_map(|s| match s.employer {
                    Employer::College(p) if s.role == Role::HeadCoach && w.programs[p].division == Division::FBS => {
                        Some(p)
                    }
                    _ => None,
                });
                if let Some(p) = last {
                    school = Some(p as u16);
                    recused.push(p as u16);
                }
            }
        }
        MemberRole::FormerPlayer => {
            // A former FBS player who went on to the NFL, out of football
            // fifteen years or more.
            let pool: Vec<(usize, f64)> = w
                .alumni
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    a.pick.is_some_and(|p| p <= 64)
                        && a.class_of <= year - 20
                        && a.class_of >= year - 45
                        && a.colleges.last().is_some_and(|&c| w.programs[c].division == Division::FBS)
                })
                .map(|(k, a)| (k, 1.0 + a.overall / 50.0))
                .collect();
            if !pool.is_empty() {
                let k = rng.weighted(&pool);
                let a = &w.alumni[k];
                name = Some(a.name.clone());
                if let Some(&c) = a.colleges.last() {
                    school = Some(c as u16);
                    recused.push(c as u16);
                }
            }
        }
        MemberRole::Journalist | MemberRole::Administrator => {}
    }
    let born = year - rng.int(50, 72);
    let name = name.unwrap_or_else(|| {
        let mut r = rng.fork();
        super::names::person_name(media_ethnicity(&mut r), born, &mut r)
    });
    // Three-year terms; the first committee's terms were staggered.
    let term = if v.committee.is_empty() || year == 2014 { rng.int(1, 3) } else { 3 };
    Member {
        name,
        role,
        school,
        recused,
        coach,
        since: year,
        term_end: year + term - 1,
        traits: Traits::draw(rng, COMMITTEE_MEANS),
    }
}

const COMMITTEE_ROLES: [MemberRole; COMMITTEE_SIZE] = [
    MemberRole::AthleticDirector,
    MemberRole::AthleticDirector,
    MemberRole::AthleticDirector,
    MemberRole::AthleticDirector,
    MemberRole::AthleticDirector,
    MemberRole::FormerCoach,
    MemberRole::FormerCoach,
    MemberRole::FormerPlayer,
    MemberRole::Journalist,
    MemberRole::Administrator,
    MemberRole::Administrator,
    MemberRole::Administrator,
    MemberRole::Administrator,
];

fn renew_committee(w: &World, v: &mut Voters, rng: &mut Rng) {
    let year = w.year;
    if year < 2014 {
        v.committee.clear();
        return;
    }
    // Terms end; each seat is filled by the same background (members are
    // replaced like for like to keep the committee's balance).
    let mut roles: Vec<MemberRole> = Vec::new();
    v.committee.retain(|m| {
        let stay = m.term_end >= year;
        if !stay {
            roles.push(m.role);
        }
        stay
    });
    if v.committee.is_empty() {
        roles = COMMITTEE_ROLES.to_vec();
    }
    for role in roles {
        let m = new_member(w, v, role, rng);
        v.committee.push(m);
    }
    while v.committee.len() < COMMITTEE_SIZE {
        let m = new_member(w, v, MemberRole::Administrator, rng);
        v.committee.push(m);
    }
}

/// The panels for a new season: built when missing (a new world or an
/// older world file), renewed once a season.
pub(crate) fn year_start(w: &mut World) {
    let mut v = std::mem::take(&mut w.voters);
    if v.year == w.year && !v.ap.is_empty() {
        w.voters = v;
        return;
    }
    let mut rng = Rng::new(w.options.seed ^ 0x7A11_0B00 ^ (w.year as u32).wrapping_mul(0x01F3_5A9D));
    renew_media(w, &mut v, false, &mut rng);
    if (2005..=2013).contains(&w.year) {
        renew_media(w, &mut v, true, &mut rng);
    } else {
        v.harris.clear();
    }
    renew_heisman(w, &mut v, &mut rng);
    renew_coaches(w, &mut v, &mut rng);
    renew_committee(w, &mut v, &mut rng);
    v.year = w.year;
    w.voters = v;
}

/// Builds the panels if a world has none yet (a world file from before
/// the voters, or a test world).
pub(crate) fn ensure(w: &mut World) {
    if w.voters.ap.is_empty() || w.voters.heisman.is_empty() {
        w.voters.year = i32::MIN;
        year_start(w);
    }
}

/// A coach's voting habits, read from his id (nothing stored).
pub(crate) fn coach_traits(seed: u32, coach: u32) -> Traits {
    Traits::keyed(seed as u64 ^ 0x0C0A_C4E5, coach as u64, COACH_MEANS)
}

/// The Heisman voters' own randomness for a season.
pub(crate) fn heisman_key(seed: u32, year: i32) -> u64 {
    (seed as u64) << 20 ^ (year as u64) ^ 0x4E15_3A11
}
