//! Favors (Ground part L, lane G11b; GR22, GR60, GR63): placing kids builds
//! standing, grown into an internal economy (`docs/design/ground/placement.md`).
//!
//! Every program has too much of one thing and too little of another, and
//! the kids you can't take are how you trade between them:
//!
//! - **A look** costs a favor: when your call makes a staff look at a kid,
//!   you owe that man one (two in a scarce week), or spend what he owes you.
//!   The look reads your word with him: a man whose staff has seen you be
//!   right looks harder (the call's weight on the kid's file).
//! - **A home found** pays: when a kid you helped place signs, his coach owes
//!   you by how hard that home was for that kid (two for a scholarship where
//!   he had none at that level or above, one for a step up, nothing for a
//!   place any staff would give). One comes at signing and the rest when your
//!   word on him settles; a quit or a cut in year one takes it back.
//! - **Your word** settles on the kid's merit at the level you claimed, with
//!   everyone who heard it, after year one for a transfer and year two for a
//!   high school signee. The staff that took him owes you back one, or two if
//!   he starts. A worry you hid that comes true costs your word.
//! - **A favor buys one answer one level up** (GR60), behind the staffs who
//!   already hold that level, and never faster than standing's gates (GR28).
//!   It never buys a commitment, a signature or a transfer (rule 3).
//! - **What you owe never fades**: he calls it in within about a year, at
//!   his hour, and a no counts as a broken promise. What he owes you fades
//!   slowly if you never ask.
//! - **Your coordinator** places the routine no's by a rule you set; the
//!   kids of Trusted and Home schools come to you (GR63).
//! - **Your leavers**: a high school coach watches where his kid lands when
//!   he leaves you.
//!
//! Everything shows in words, never numbers (GR63).
//!
//! It ships switched off (GR61): `WorldOptions.favors` is false in every
//! world, and with it off nothing here runs and placing a kid works as it
//! did (`folk/backchannel.rs`, `place_player`). The trial (lane G12) turns it
//! on. Your seat only: the AI's own placements and its spending come with the
//! AI working its ground (lane G9). Every size is the build's pick, proposed,
//! not confirmed, to be tuned in GR61's trial.

use super::memory::{Favor, Memory};
use super::people::{self, Holder, Kind, Who};
use super::standing::{self, Level};
use super::{key, unit};
use crate::calendar::{Date, Weekday};
use crate::player::{PlayingTime, Trait};
use crate::world::coach::Employer;
use crate::world::{Division, Stage, World};

// The sizes (proposed, not confirmed; tuned in GR61's trial).
/// Standing a home pays its sender per favor it earned (help, GR23).
pub const HOME_POINTS: f32 = 4.0;
/// Calls a kid gets in one recruiting year, from you and your coordinator.
pub const MAX_CALLS: usize = 3;
/// Days a look waits for him to sign before it lapses.
pub const LOOK_DAYS: i32 = 400;
/// A transfer's word settles after a year, a high school signee's after two.
pub const SETTLE_TRANSFER: i32 = 365;
pub const SETTLE_SIGNEE: i32 = 730;
/// A self-serving man dodges an ask this often when a bigger program
/// already holds that level with him.
pub const DODGE: f64 = 0.6;
/// Asking for more than he owes costs warmth.
pub const OVERASK: f32 = -3.0;
/// A pulled offer whose kid you place by signing day costs a quarter.
pub const PLACED_NO_SHARE: f32 = 0.25;
/// A second pull at the same school inside this many years costs in full.
pub const SECOND_PULL_YEARS: i32 = 3;
/// Your word levels off near six to eight settled claims: its pull is
/// n / (n + this).
pub const WORD_SETTLE: f32 = 2.0;
/// A debt he hasn't called in by this many days, he calls now.
pub const CALL_IN_BY: i32 = 330;
/// The weekly chance he calls a debt in before then.
pub const CALL_IN_WEEK: f64 = 0.06;
/// What he owes you fades by one after this many days without a favor.
pub const OWED_FADE: i32 = 730;
/// A leaver you helped land well, and one who drifted.
pub const LEAVER_HELD: f32 = 3.0;
pub const LEAVER_DRIFT: f32 = -4.0;
/// Your coordinator's calls for each routine no until you set a rule.
pub const DEFAULT_CALLS: u8 = 1;

/// The favor book on `Ground.favors`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Favors {
    /// Kids you and your coordinator called about, newest last.
    pub placed: Vec<Placement>,
    /// Answers bought one level up (GR60), until they are used.
    pub lifts: Vec<Lift>,
    /// Your coordinator's rule for the routine no's (GR63).
    pub rule: Rule,
    /// Your players who entered the portal, for their coaches' watch.
    pub leavers: Vec<Leaver>,
    pub(crate) next: u32,
}

impl Default for Favors {
    fn default() -> Self {
        Favors {
            placed: Vec::new(),
            lifts: Vec::new(),
            rule: Rule { calls: DEFAULT_CALLS },
            leavers: Vec::new(),
            next: 0,
        }
    }
}

/// Your coordinator's rule: how many calls each kid you pass on gets.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rule {
    pub calls: u8,
}

/// A worry you name in a claim ("Watch his grades").
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Worry {
    Grades,
    Temper,
    Health,
    Work,
}

impl Worry {
    pub const ALL: [Worry; 4] = [Worry::Grades, Worry::Temper, Worry::Health, Worry::Work];

    pub fn words(self) -> &'static str {
        match self {
            Worry::Grades => "his grades",
            Worry::Temper => "his temper",
            Worry::Health => "his health",
            Worry::Work => "his work",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Worry::Grades => "grades",
            Worry::Temper => "temper",
            Worry::Health => "health",
            Worry::Work => "work",
        }
    }

    pub fn from(s: &str) -> Option<Worry> {
        Worry::ALL.into_iter().find(|w| w.key() == s)
    }
}

/// Where a placement stands.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum State {
    /// His staff is looking.
    Looked,
    /// He signed there (or landed there from the portal) on this day.
    Signed(Date),
    /// Your word on him settled: held or not.
    Settled(bool),
    /// He went somewhere else, or the look lapsed.
    Lost,
}

/// One call about one kid to one staff.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Placement {
    pub id: u32,
    pub kid: u32,
    /// His coach, who owes you for the home (his high school, junior
    /// college or prep coach).
    pub sender: Option<Who>,
    /// The program you called and the man you called there.
    pub taker: u32,
    pub man: u32,
    /// Your man who made the call (you or your coordinator): favors are
    /// personal (GR29).
    pub by: u32,
    pub date: Date,
    /// The recruiting year (one chain per kid per year).
    pub season: i32,
    /// 0 worth a look, 1 can play for you, 2 a sure thing (`tips::CLAIMS`).
    pub claim: u8,
    /// The worry you named.
    pub worry: Option<Worry>,
    /// A worry he really has that you didn't name, read at the call.
    pub hidden: Option<Worry>,
    pub transfer: bool,
    /// The level you claimed (the taker's division).
    pub division: u8,
    /// His best standing (the highest division that offered him, yours
    /// included); 9 none.
    pub best: u8,
    pub best_prestige: f32,
    /// You pulled his offer before you called.
    pub pulled: bool,
    pub state: State,
    /// Favors the home earned, and how many his coach has paid.
    pub owed: u8,
    pub paid: u8,
    /// His college snaps and injury seasons when he signed (the settle reads
    /// what came after).
    pub snaps: u32,
    pub hurt: u8,
}

/// An answer bought one level up (GR60): for the next news on the kid
/// (`kid` 0: his list), behind the staffs that hold that level.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Lift {
    pub who: Who,
    pub kid: u32,
    pub level: Level,
    pub date: Date,
    pub used: bool,
}

/// One of your players who entered the portal.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Leaver {
    pub kid: u32,
    pub coach: Option<Who>,
    pub date: Date,
    pub judged: bool,
}

/// The layer is on: the world's switch, and a college seat.
pub fn on(w: &World) -> bool {
    w.options.favors && standing::seat_program(w).is_some()
}

fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

fn div(w: &World, prog: usize) -> u8 {
    w.programs.get(prog).map_or(9, |p| p.division as u8)
}

fn last(w: &World, i: usize) -> String {
    crate::world::names::surname_of(&w.people[i].name).to_string()
}

/// A scarce week: the days around a signing day and the portal window's
/// last days. A look there costs (and buys) two.
pub fn scarce(w: &World, d: Date) -> bool {
    let y = &w.dates;
    let near = |a: Date, before: i32, after: i32| d >= a.add(-before) && d <= a.add(after);
    if near(y.signing_day, 0, 1) || y.early_signing.is_some_and(|e| near(e, 0, 2)) {
        return true;
    }
    [y.season - 1, y.season].into_iter().any(|s| {
        let (_, close) = crate::calendar::anchors::january_window(s);
        d >= close.add(-2) && d <= close
    })
}

/// Your word with a man, as a pull from 0 to 1: 0.5 until a claim settles,
/// then toward his record, levelling off near six to eight claims.
pub fn word_read(m: Option<&Memory>) -> f32 {
    let (n, held) = m.map_or((0, 0), |m| m.your_word);
    if n == 0 {
        return 0.5;
    }
    let n = n as f32;
    0.5 + (held as f32 / n - 0.5) * (n / (n + WORD_SETTLE))
}

/// Favors between a person and a holder: + he owes you. A college
/// colleague's balance is your network's (`folk/network.rs`).
pub fn balance(w: &World, who: Who, holder: Holder) -> i8 {
    if let (Who::Coach(c), true) = (who, Some(holder) == you(w).map(|y| Holder::Coach(y as u32)))
        && people::kind(w, who) == Kind::College
        && let Some(x) = w.folk.network.contacts.iter().find(|x| x.coach_id() == Some(c as usize))
    {
        return x.balance;
    }
    w.ground.get(who, holder).map_or(0, |m| m.owes)
}

/// Moves the favors between a person and a holder (+ he owes you), with
/// their words. A college colleague's moves your network's balance and
/// ledger too, so the contacts screen shows it.
pub(crate) fn owe(w: &mut World, who: Who, holder: Holder, by: i8, words: String) {
    if by == 0 {
        return;
    }
    let d = w.today;
    let start = standing::base(w, who, holder);
    let m = standing::entry(&mut w.ground, start, who, holder, d);
    m.favor(by, words.clone(), d);
    let yours = Some(holder) == you(w).map(|y| Holder::Coach(y as u32));
    if let (Who::Coach(c), true) = (who, yours && people::kind(w, who) == Kind::College) {
        let c = c as usize;
        if !w.folk.network.contacts.iter().any(|x| x.coach_id() == Some(c)) {
            crate::world::folk::network::touch(w, c, 0.05);
        }
        if let Some(x) = w.folk.network.contacts.iter_mut().find(|x| x.coach_id() == Some(c)) {
            x.shift(by);
        }
        w.folk.network.ledger.push(crate::world::folk::network::Favour {
            date: d,
            coach: c,
            words,
            asked: by < 0,
            other: None,
        });
    }
}

/// A real hole on the taker's roster for him (the guard "no hole, no
/// favor"): he would be in its two-deep at his spot, or the spot is short
/// of a real roster's numbers at its level for next fall.
pub fn hole(w: &World, prog: usize, i: usize) -> bool {
    let (Some(pr), Some(p)) = (w.programs.get(prog), w.people.get(i)) else { return false };
    if !pr.plays(w.dates.season + 1) {
        return false;
    }
    let pos = p.player.position;
    let ovr = p.player.overall();
    let slots = crate::league::depth_slots(pos);
    let (mut staying, mut ahead) = (0usize, 0usize);
    for &k in &pr.roster {
        let Some(q) = w.people.get(k) else { continue };
        if k == i || q.player.position != pos || q.stage != Stage::College || q.cal.portal.is_some() {
            continue;
        }
        if q.college_seasons >= 4 {
            continue;
        }
        staying += 1;
        if q.player.overall() >= ovr {
            ahead += 1;
        }
    }
    let share = crate::world::recruiting::level_roster(pr.division)
        / crate::world::recruiting::level_roster(Division::FBS).max(1.0);
    let need = (2.0 * slots.roster as f64 * share).round().max(1.0) as usize;
    ahead < 2 * slots.starters || staying < need
}

/// His coach, who owes you for a home: his junior college or prep coach
/// when he is there, else his high school's head coach.
fn sender_of(w: &World, i: usize) -> Option<Who> {
    let p = &w.people[i];
    if p.stage == Stage::PrepSchool {
        return people::prep_of(w, i);
    }
    if p.stage == Stage::College
        && let Some(c) = p.college
        && w.programs[c].division == Division::JUCO
    {
        return w.programs[c].head_coach.map(|h| Who::Coach(h as u32));
    }
    people::head_coach_of(w, p.hs)
}

/// His best standing: the highest division that offered him (your board
/// counts, pulled or not), or the program he plays for on scholarship.
fn best(w: &World, i: usize, you_prog: usize) -> (u8, f32) {
    let p = &w.people[i];
    let mut b = (9u8, 0.0f32);
    let mut see = |j: usize| {
        if let Some(q) = w.programs.get(j) {
            let d = q.division as u8;
            if d < b.0 || (d == b.0 && q.prestige as f32 > b.1) {
                b = (d, q.prestige as f32);
            }
        }
    };
    for &j in &p.cal.offers {
        see(j);
    }
    for &(j, _) in &p.cal.early_offers {
        see(j);
    }
    if p.stage == Stage::College
        && !p.walk_on
        && let Some(c) = p.college
    {
        see(c);
    }
    if w.front_office.college.board.iter().any(|t| t.id == p.player.id && t.offered.is_some()) {
        see(you_prog);
    }
    b
}

/// A worry he really has: the one a staff would most want to hear.
fn real_worry(w: &World, i: usize) -> Option<Worry> {
    let p = &w.people[i];
    let pl = &p.player;
    if pl.personality.traits.get(Trait::Temper) >= 15.0 || p.incident {
        Some(Worry::Temper)
    } else if crate::world::scouting::file::gpa(p) < 2.5 {
        Some(Worry::Grades)
    } else if pl.injury_proneness >= 15.0 {
        Some(Worry::Health)
    } else if pl.personality.work_ethic < 6.0 {
        Some(Worry::Work)
    } else {
        None
    }
}

/// Whether a worry came true by the settle.
fn came_true(p: &crate::world::Person, worry: Worry, pl: &Placement) -> bool {
    match worry {
        Worry::Grades => p.cal.academic_sit || p.cal.spring_ineligible || p.cal.fall_ineligible || p.academic_redshirt,
        Worry::Temper => p.incident || p.cal.trouble,
        Worry::Health => p.roads.injury_seasons > pl.hurt,
        Worry::Work => p.college_snaps.saturating_sub(pl.snaps) < 150,
    }
}

/// The college programs where a kid has a real hole, best first for a
/// call: his home state, near his best standing, then the bigger name.
pub fn takers(w: &World, kid: u32, n: usize) -> Vec<usize> {
    let Some(prog) = standing::seat_program(w) else { return Vec::new() };
    let Some(i) = crate::world::folk::person(w, kid) else { return Vec::new() };
    let p = &w.people[i];
    let (b, _) = best(w, i, prog);
    let mut v: Vec<(f64, usize)> = (0..w.programs.len())
        .filter(|&j| j != prog && Some(j) != p.college && w.programs[j].head_coach.is_some())
        .filter(|&j| hole(w, j, i))
        .map(|j| {
            let q = &w.programs[j];
            let d = q.division as u8;
            let home = if q.state == p.home_state { 2.0 } else { 0.0 };
            let gap = (d as f64 - b.min(5) as f64).abs();
            (home - gap + q.prestige, j)
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().take(n).map(|x| x.1).collect()
}

/// You (or your coordinator, `by`) call a man on another college staff
/// about a kid (GR22): "Take a look at him." His staff looks, and you owe
/// him for it (or spend what he owes you). Any kid in any direction: a
/// senior you passed on, a player of yours in the portal, a kid in the
/// portal you know. Only for a real hole on his roster; one chain per kid
/// per recruiting year.
pub fn place(
    w: &mut World,
    kid: u32,
    man: usize,
    claim: u8,
    worry: Option<Worry>,
    by: Option<usize>,
) -> Result<String, String> {
    if !on(w) {
        return Err("Placing kids as favors isn't switched on in this world.".into());
    }
    let prog = standing::seat_program(w).ok_or("Favors are for a college staff.")?;
    let me = you(w).ok_or("Start a career first.")?;
    let caller = by.unwrap_or(me);
    let d = w.today;
    let i = crate::world::folk::person(w, kid).ok_or("No such player.")?;
    let taker = match people::employer(w, man) {
        Some(Employer::College(q)) if q != prog => q,
        _ => return Err("He isn't on another college's staff.".into()),
    };
    let season = w.dates.season;
    let p = &w.people[i];
    let name = p.name.clone();
    let transfer = match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            if p.cal.signed.is_some() {
                return Err(format!("{name} has signed already."));
            }
            if p.class_of > season + 1 {
                return Err(format!("{name} isn't a senior yet: there's no place to find for him this year."));
            }
            false
        }
        Stage::College => {
            if p.college == Some(prog) && p.last_role == Some(PlayingTime::Starter) && p.cal.portal.is_none() {
                return Err(format!("{name} starts for you."));
            }
            // A Division I staff can't talk to another school's player
            // until he is in the portal: the call comes after he enters.
            if p.cal.portal.is_none() && w.programs[prog].division != Division::JUCO {
                return Err(format!("{name} isn't in the portal: nobody can call about him yet."));
            }
            true
        }
        _ => return Err(format!("{name} isn't a kid anyone can place.")),
    };
    if p.college == Some(taker) || p.cal.commit.is_some_and(|c| c.0 == taker) {
        return Err(format!("{name} is theirs already."));
    }
    let tname = w.program_name(taker).to_string();
    let mine: Vec<&Placement> = w
        .ground
        .favors
        .placed
        .iter()
        .filter(|x| x.kid == kid && x.season == season && x.state != State::Lost)
        .collect();
    if mine.iter().any(|x| x.taker as usize == taker) {
        return Err(format!("You've called {tname} about {name} already."));
    }
    if mine.iter().any(|x| matches!(x.state, State::Signed(_) | State::Settled(_))) {
        return Err(format!("{name} has his home: one favor chain for a kid each year."));
    }
    if mine.len() >= MAX_CALLS {
        return Err(format!("{name} has had his {MAX_CALLS} calls this year."));
    }
    if !hole(w, taker, i) {
        return Err(format!("{tname} has no hole for a {}: no hole, no favor.", w.people[i].player.position.name()));
    }
    let holder = Holder::Coach(caller as u32);
    let mwho = Who::Coach(man as u32);
    let read = word_read(w.ground.get(mwho, holder));
    let mname = w.coaches[man].name.clone();
    if caller == me {
        w.ground.week.book(d, &format!("Call: {mname} about {name}"), super::week::phone::CALL_HOURS, true);
    }
    // His staff looks, as hard as your word with him is good.
    if !transfer {
        w.scouting.hs.promoted.push(crate::world::scouting::hs::Promo {
            id: kid,
            program: Some(taker),
            date: d,
            strength: claim.min(2),
            settled: None,
            warmth: read,
        });
    }
    let cost: i8 = if scarce(w, d) { 2 } else { 1 };
    let had = balance(w, mwho, holder);
    let what = format!("a look at {name}");
    owe(w, mwho, holder, -cost, what);
    let paid_words = if had >= cost {
        "That's what he owed you.".to_string()
    } else if cost == 2 {
        "You owe him a big one: it's a scarce week.".to_string()
    } else {
        "You owe him one.".to_string()
    };
    let (b, bp) = best(w, i, prog);
    let pulled = w.front_office.college.board.iter().any(|t| t.id == kid && t.pulled.is_some());
    let p = &w.people[i];
    let id = {
        w.ground.favors.next += 1;
        w.ground.favors.next
    };
    let pl = Placement {
        id,
        kid,
        sender: sender_of(w, i),
        taker: taker as u32,
        man: man as u32,
        by: caller as u32,
        date: d,
        season,
        claim: claim.min(2),
        worry,
        hidden: real_worry(w, i).filter(|r| Some(*r) != worry),
        transfer,
        division: div(w, taker),
        best: b,
        best_prestige: bp,
        pulled,
        state: State::Looked,
        owed: 0,
        paid: 0,
        snaps: p.college_snaps,
        hurt: p.roads.injury_seasons,
    };
    w.ground.favors.placed.push(pl);
    // His coach passes your worry to the family that evening.
    if let Some(wy) = worry {
        let fam = people::family_of(w, i);
        standing::pay(
            w,
            prog,
            fam,
            0.0,
            d,
            None,
            Some((format!("Coach was straight with us about {}.", wy.words()), true, false)),
        );
    }
    let claim_words = crate::world::folk::tips::CLAIMS[claim.min(2) as usize];
    let worry_words = worry.map_or(String::new(), |wy| format!(" Watch {}.", wy.words()));
    let text = format!(
        "{} called {mname} at {tname} about {name}: \"{claim_words}.{worry_words}\" His staff will look. {paid_words}",
        if caller == me { "You".to_string() } else { w.coaches[caller].name.clone() }
    );
    crate::world::folk::note(w, "network", mname, text.clone(), None);
    Ok(text)
}

/// Spend a favor (GR60): one answer one level above your standing with
/// him, behind the staffs that already hold that level, never past the
/// gates (GR28). At Stranger his real list, at Known the truth about one
/// kid, at Trusted his first call on one kid.
pub fn ask(w: &mut World, who: Who, kid: Option<u32>) -> Result<String, String> {
    if !on(w) {
        return Err("Favors aren't switched on in this world.".into());
    }
    let prog = standing::seat_program(w).ok_or("Favors are for a college staff.")?;
    let me = you(w).ok_or("Start a career first.")?;
    let d = w.today;
    let name = people::name(w, who);
    let (pts, held_by) = standing::staff(w, prog, who);
    let lv = Level::of(pts);
    if lv == Level::Home {
        return Err(format!("{name} already tells you everything first: no favor buys more."));
    }
    let next = Level::ALL[lv as usize + 1];
    // The gates (GR28): a favor never makes the clock run faster.
    let start = standing::base(w, who, held_by);
    let cap = standing::cap(standing::entry(&mut w.ground, start, who, held_by, d), d);
    if cap < next.floor() {
        return Err(match next {
            Level::Known => format!("{name} doesn't know you yet: no favor answers for a man you've never met."),
            Level::Trusted => {
                format!("No favor buys {name}'s trust inside a year of knowing you, or while he holds a broken word.")
            }
            _ => format!("{name} calls first only for a staff that has done right by one of his kids."),
        });
    }
    let kid_i = match (next, kid) {
        (Level::Known, _) => None,
        (_, Some(k)) => Some(crate::world::folk::person(w, k).ok_or("No such player.")?),
        (_, None) => return Err("Which kid?".into()),
    };
    let k = kid.filter(|_| next != Level::Known).unwrap_or(0);
    if w.ground.favors.lifts.iter().any(|l| l.who == who && l.kid == k && !l.used) {
        return Err(format!("{name} owes you that answer already."));
    }
    w.ground.week.book(d, &format!("Call: {name}"), super::week::phone::CALL_HOURS, true);
    let holder = Holder::Coach(me as u32);
    let mk = people::makeup(w, who);
    // A self-serving man dodges when a bigger program already holds that
    // level with him; the favor isn't spent, and you know what he is.
    let mine = w.programs[prog].prestige;
    let bigger = (0..w.programs.len()).find(|&j| {
        j != prog && w.programs[j].prestige > mine && standing::level(w, j, who) >= next && w.programs[j].plays(w.year)
    });
    if let Some(j) = bigger
        && mk.loyal < 0.5
        && unit(w.options.seed, &[0xFA0D, people::post_key(w, who) as u64, d.0 as u64])
            < DODGE * (1.0 - 2.0 * mk.loyal as f64)
    {
        let words = format!("he dodged your ask when {} wanted the same", w.program_name(j));
        let start = standing::base(w, who, holder);
        let m = standing::entry(&mut w.ground, start, who, holder, d);
        m.favors.push(Favor { date: d, words, his: false });
        if m.favors.len() > super::memory::FAVORS {
            m.favors.remove(0);
        }
        return Ok(format!("{name} dodged you. He'll get back to you, he says. You know what that means."));
    }
    let owes = balance(w, who, holder);
    let what = match (next, kid_i) {
        (Level::Known, _) => "his real list".to_string(),
        (Level::Trusted, Some(i)) => format!("the truth about {}", w.people[i].name),
        (_, Some(i)) => format!("his first call on {}", w.people[i].name),
        _ => "an answer".to_string(),
    };
    owe(w, who, holder, -1, what);
    let mut text = String::new();
    if owes < 1 {
        let start = standing::base(w, who, held_by);
        let m = standing::entry(&mut w.ground, start, who, held_by, d);
        standing::gain(m, OVERASK, d);
        text.push_str("You asked for more than he owed you. He answered, but he noticed. ");
    }
    w.ground.favors.lifts.push(Lift { who, kid: k, level: next, date: d, used: next == Level::Known });
    text.push_str(&match (next, kid_i) {
        (Level::Known, _) => list(w, who),
        (Level::Trusted, Some(i)) => truth(w, who, i),
        (_, Some(i)) => format!(
            "{name}: \"When something happens with {}, you'll hear it from me first. Right after the people I owe.\"",
            w.people[i].name
        ),
        _ => String::new(),
    });
    crate::world::folk::note(w, "network", name, text.clone(), None);
    Ok(text)
}

/// His real list (a Known answer): his kids in his own order, by his eye.
fn list(w: &World, who: Who) -> String {
    let name = people::name(w, who);
    let mut kids: Vec<usize> = match people::school(w, who) {
        Some(h) => people::kids_at(w, h, 12),
        None => people::clients(w, who, 12),
    };
    if kids.is_empty() {
        return format!("{name}: \"Nobody this year who's ready for you. I'll call when there is.\"");
    }
    let eye = people::makeup(w, who).eye as f64;
    let read = |i: usize| {
        let p = &w.people[i];
        let noise = (unit(w.options.seed, &[0xFA11, p.player.id as u64]) - 0.5) * (21.0 - eye) * 0.8;
        p.player.overall() + noise
    };
    kids.sort_by(|&a, &b| read(b).total_cmp(&read(a)).then(a.cmp(&b)));
    kids.truncate(5);
    let names: Vec<String> = kids
        .iter()
        .map(|&i| {
            format!("{} ({}, class of {})", w.people[i].name, w.people[i].player.position.name(), w.people[i].class_of)
        })
        .collect();
    format!("{name} gave you his real list: {}.", names.join(", "))
}

/// The truth about a kid (a Trusted answer): his work, his grades, his
/// temper, what he plays through. Words, never numbers.
fn truth(w: &World, who: Who, i: usize) -> String {
    let name = people::name(w, who);
    let p = &w.people[i];
    let pl = &p.player;
    let mut parts: Vec<&str> = Vec::new();
    parts.push(match pl.personality.work_ethic {
        x if x >= 14.0 => "He's the first one in and the last one out.",
        x if x >= 8.0 => "He does the work.",
        _ => "He has to be pushed to work.",
    });
    parts.push(match crate::world::scouting::file::gpa(p) {
        x if x >= 3.3 => "He's a real student.",
        x if x >= 2.5 => "His grades are fine.",
        _ => "Watch his grades: he's close to the line.",
    });
    if pl.personality.traits.get(Trait::Temper) >= 15.0 {
        parts.push("He has a temper, and it's cost him before.");
    }
    parts.push(match pl.injury_proneness {
        x if x >= 14.0 => "He plays through things he shouldn't, and his body breaks down.",
        x if x <= 6.0 => "He's never missed a practice.",
        _ => "Nothing that keeps him off the field.",
    });
    format!("{name} on {}: \"{}\"", p.name, parts.join(" "))
}

/// Your level for one answer from a person about a kid (GR60), when a
/// favor lifted it: lane G4's news order reads it, and places you behind
/// the staffs that already hold that level.
pub fn lifted(w: &World, prog: usize, who: Who, kid: u32) -> Option<Level> {
    if !on(w) || standing::seat_program(w) != Some(prog) {
        return None;
    }
    w.ground.favors.lifts.iter().find(|l| l.who == who && (l.kid == kid || l.kid == 0) && !l.used).map(|l| l.level)
}

/// The lifted answer is spent once it has been heard.
pub fn spend_lift(w: &mut World, who: Who, kid: u32) {
    if let Some(l) = w.ground.favors.lifts.iter_mut().find(|l| l.who == who && (l.kid == kid || l.kid == 0) && !l.used)
    {
        l.used = true;
    }
}

/// Your coordinator's rule (GR63): how many calls each routine no gets.
pub fn set_rule(w: &mut World, calls: u8) -> Result<String, String> {
    if !on(w) {
        return Err("Favors aren't switched on in this world.".into());
    }
    let calls = calls.min(MAX_CALLS as u8);
    w.ground.favors.rule.calls = calls;
    Ok(match calls {
        0 => "Your coordinator won't place anyone: every call is yours.".to_string(),
        1 => "Every kid we pass on gets a call. The Trusted and Home schools' kids are yours.".to_string(),
        n => format!("Every kid we pass on gets {n} calls. The Trusted and Home schools' kids are yours."),
    })
}

/// The kids in the portal a placement made a staff look at: the call
/// reaches a small school outside its region (`recruiting.rs`).
pub(crate) fn looked(w: &World) -> std::collections::HashSet<(u32, usize)> {
    if !w.options.favors {
        return Default::default();
    }
    w.ground
        .favors
        .placed
        .iter()
        .filter(|x| x.transfer && x.state == State::Looked)
        .map(|x| (x.kid, x.taker as usize))
        .collect()
}

// ---------------------------------------------------------------------------
// Each day.

pub(crate) fn day(w: &mut World, d: Date) {
    if !on(w) {
        return;
    }
    let Some(prog) = standing::seat_program(w) else { return };
    signings(w, prog, d);
    settle(w, prog, d);
    if d.weekday() == Weekday::Mon {
        leavers(w, prog, d);
        coordinator(w, prog, d);
    }
    let (_, m, dd) = d.parts();
    if m == 9 && dd == 1 {
        judge_leavers(w, prog, d);
    }
    if m == 1 && dd == 1 {
        fade(w, d);
    }
    // Keep four years of the book.
    let cut = d.add(-4 * 365);
    w.ground.favors.placed.retain(|x| x.date >= cut || matches!(x.state, State::Signed(_)));
    w.ground.favors.lifts.retain(|l| !l.used || l.date >= d.add(-60));
    w.ground.favors.lifts.retain(|l| l.date >= d.add(-365));
    w.ground.favors.leavers.retain(|l| l.date >= d.add(-2 * 365));
}

/// How hard the home was for him (placement.md, "A home found"): two for a
/// scholarship where he had none at that level or above, one for a step up
/// by his coach's lights, nothing for a place any staff would give or a
/// kid whose offer you pulled.
fn hardness(w: &World, pl: &Placement, i: usize) -> u8 {
    let p = &w.people[i];
    let walk = p.cal.signed.map_or(p.walk_on, |s| s.walk_on);
    let t = pl.division;
    let open = t == Division::JUCO as u8 || t == Division::D3 as u8 || (t == Division::NAIA as u8 && walk);
    if open || pl.pulled {
        return 0;
    }
    if walk {
        return (pl.best > t) as u8;
    }
    if pl.best > t {
        return 2;
    }
    if pl.best < t {
        return 0;
    }
    // The same level: a step up as his coach sees it (GR23). A loyal coach
    // counts the playing time, a self-serving one the bigger name.
    let loyal = pl.sender.is_none_or(|s| people::makeup(w, s).loyal >= 0.5);
    if loyal {
        let starters = crate::league::depth_slots(p.player.position).starters;
        let ahead = w.programs[pl.taker as usize]
            .roster
            .iter()
            .filter(|&&k| {
                k != i
                    && w.people[k].player.position == p.player.position
                    && w.people[k].stage == Stage::College
                    && w.people[k].player.overall() >= p.player.overall()
            })
            .count();
        (ahead < starters) as u8
    } else {
        (w.programs[pl.taker as usize].prestige as f32 > pl.best_prestige) as u8
    }
}

/// Looks that turned into homes, and looks that lapsed.
fn signings(w: &mut World, prog: usize, d: Date) {
    for k in 0..w.ground.favors.placed.len() {
        let pl = w.ground.favors.placed[k].clone();
        if pl.state != State::Looked {
            continue;
        }
        let Some(i) = crate::world::folk::person(w, pl.kid) else {
            w.ground.favors.placed[k].state = State::Lost;
            continue;
        };
        let p = &w.people[i];
        let taker = pl.taker as usize;
        let landed = if pl.transfer {
            p.stage == Stage::College && p.college == Some(taker) && p.cal.portal.is_none()
        } else {
            p.cal.signed.is_some_and(|s| s.program == taker) || (p.stage == Stage::College && p.college == Some(taker))
        };
        let elsewhere = if pl.transfer {
            p.cal.portal.is_none() && p.college != Some(taker)
        } else {
            p.cal.signed.is_some_and(|s| s.program != taker) || matches!(p.stage, Stage::College | Stage::Done)
        };
        if landed {
            sign(w, prog, k, i, d);
        } else if elsewhere || pl.date.days_to(d) > LOOK_DAYS {
            w.ground.favors.placed[k].state = State::Lost;
        }
    }
}

fn sign(w: &mut World, prog: usize, k: usize, i: usize, d: Date) {
    let pl = w.ground.favors.placed[k].clone();
    let mut hard = hardness(w, &pl, i);
    // Placements inside your coaching tree count half.
    let tree = w.programs[pl.taker as usize]
        .head_coach
        .is_some_and(|h| w.coaches[h].mentor == Some(pl.by as usize) || w.coaches[pl.by as usize].mentor == Some(h));
    if tree {
        hard /= 2;
    }
    // Later homes from the same coach in the same year count one less.
    let again = w.ground.favors.placed.iter().any(|x| {
        x.id != pl.id
            && x.sender == pl.sender
            && x.sender.is_some()
            && x.season == pl.season
            && matches!(x.state, State::Signed(_) | State::Settled(_))
    });
    if again {
        hard = hard.saturating_sub(1);
    }
    let lastn = last(w, i);
    let tname = w.program_name(pl.taker as usize).to_string();
    let snaps = w.people[i].college_snaps;
    let hurt = w.people[i].roads.injury_seasons;
    {
        let x = &mut w.ground.favors.placed[k];
        x.state = State::Signed(d);
        x.owed = hard;
        x.snaps = snaps;
        x.hurt = hurt;
    }
    // The other calls for him are spent.
    for x in w.ground.favors.placed.iter_mut() {
        if x.kid == pl.kid && x.id != pl.id && x.state == State::Looked {
            x.state = State::Lost;
        }
    }
    let holder = Holder::Coach(pl.by);
    if let Some(s) = pl.sender {
        if hard >= 1 {
            owe(w, s, holder, 1, format!("you found {lastn} a home at {tname}"));
            w.ground.favors.placed[k].paid = 1;
            standing::pay(
                w,
                prog,
                s,
                HOME_POINTS * hard as f32,
                d,
                None,
                Some((format!("You found {lastn} a home at {tname}."), true, hard >= 2)),
            );
        }
        // The no you placed (placement.md): a pulled offer costs a quarter
        // when he has a home by signing day, unless he turned others down
        // for yours or it is your second pull at his school in three years.
        if pl.pulled && d <= w.dates.signing_day {
            let hs = w.people[i].hs;
            let committed = w.people[i].cal.commit.is_some_and(|c| c.0 == prog);
            let second = w.front_office.college.board.iter().any(|t| {
                t.id != pl.kid
                    && t.pulled.is_some_and(|x| x.days_to(d) <= SECOND_PULL_YEARS * 365)
                    && crate::world::folk::person(w, t.id).is_some_and(|j| w.people[j].hs == hs)
            });
            if !committed && !second {
                let words = format!("You pulled {lastn}'s offer, but you found him a home.");
                restore(w, prog, s, d, words);
                let fam = people::family_of(w, i);
                restore(w, prog, fam, d, "You found our son a home.".into());
            }
        }
    }
    let text = match hard {
        0 => format!("{} signed with {tname}. A place any staff would give: nobody owes you for it.", w.people[i].name),
        1 => format!("{} signed with {tname}. His coach owes you one.", w.people[i].name),
        _ => format!(
            "{} signed with {tname}: a scholarship he never had. His coach owes you a big one.",
            w.people[i].name
        ),
    };
    crate::world::folk::note(w, "network", tname, text, None);
}

/// A pulled offer's cost cut to a quarter (placement.md, "The no you don't
/// place"): three quarters of the loss come back, outside the gates since
/// it undoes a loss, and the pull is no longer a moment he holds for years.
fn restore(w: &mut World, prog: usize, who: Who, d: Date, words: String) {
    let before = standing::level(w, prog, who);
    let back = -standing::PROMISE_BROKEN * (1.0 - PLACED_NO_SHARE);
    let mut hs = vec![Holder::Program(prog as u32)];
    hs.extend(standing::head_holder(w, prog));
    for h in hs {
        let Some(m) = w.ground.get_mut(who, h) else { continue };
        m.points = (m.points + back).clamp(0.0, 100.0);
        for x in m.moments.iter_mut().filter(|x| !x.good && x.words.starts_with("You pulled")) {
            x.big = false;
            x.holds_to = None;
        }
        m.keep(super::memory::Moment { date: d, words: words.clone(), good: true, big: false, holds_to: None });
    }
    standing::announce(w, prog, who, before, d);
}

/// Your word settles on his merit at the level you claimed, with everyone
/// who heard it; a quit or a cut in year one takes the home back.
fn settle(w: &mut World, prog: usize, d: Date) {
    for k in 0..w.ground.favors.placed.len() {
        let pl = w.ground.favors.placed[k].clone();
        let State::Signed(sd) = pl.state else { continue };
        let Some(i) = crate::world::folk::person(w, pl.kid) else {
            w.ground.favors.placed[k].state = State::Settled(false);
            continue;
        };
        let p = &w.people[i];
        let taker = pl.taker as usize;
        let pro = matches!(p.stage, Stage::Pro | Stage::Declared);
        let gone_early = d < sd.add(365)
            && sd.add(30) < d
            && (p.stage == Stage::Done || (p.stage == Stage::College && p.college != Some(taker)));
        let due = sd.add(if pl.transfer { SETTLE_TRANSFER } else { SETTLE_SIGNEE });
        if !gone_early && d < due {
            continue;
        }
        let role = p.last_role;
        let at_level = pro || (p.stage == Stage::College && p.college.is_some_and(|c| div(w, c) <= pl.division));
        let played = p.college_snaps.saturating_sub(pl.snaps) >= 150
            || matches!(role, Some(PlayingTime::Starter | PlayingTime::Rotation));
        let starter = role == Some(PlayingTime::Starter) && p.college == Some(taker);
        let held = !gone_early
            && match pl.claim {
                0 => pro || played,
                1 => pro || (at_level && matches!(role, Some(PlayingTime::Starter | PlayingTime::Rotation))),
                _ => pro || (at_level && role == Some(PlayingTime::Starter)),
            };
        let hidden = pl.hidden.filter(|&h| came_true(p, h, &pl));
        let lastn = last(w, i);
        let tname = w.program_name(taker).to_string();
        w.ground.favors.placed[k].state = State::Settled(held);
        let holder = Holder::Coach(pl.by);
        let man = Who::Coach(pl.man);
        // Everyone who heard the claim records it in your word.
        let heard: Vec<Who> = std::iter::once(man).chain(pl.sender).collect();
        for &who in &heard {
            for h in [holder, Holder::Program(prog as u32)] {
                let start = standing::base(w, who, h);
                let m = standing::entry(&mut w.ground, start, who, h, d);
                m.your_word.0 = m.your_word.0.saturating_add(1);
                if held {
                    m.your_word.1 = m.your_word.1.saturating_add(1);
                }
            }
        }
        if gone_early && pl.paid > 0 {
            if let Some(s) = pl.sender {
                owe(w, s, holder, -(pl.paid as i8), format!("{lastn} didn't last a year at {tname}"));
            }
            w.ground.favors.placed[k].paid = 0;
        }
        if held {
            owe(w, man, holder, 1 + starter as i8, format!("your word on {lastn} held up"));
            if let Some(s) = pl.sender {
                let rest = pl.owed.saturating_sub(pl.paid);
                if rest > 0 {
                    owe(w, s, holder, rest as i8, format!("{lastn} held up at {tname}"));
                    w.ground.favors.placed[k].paid = pl.owed;
                }
                let words = format!("Your word on {lastn} held up.");
                standing::pay(w, prog, s, standing::WORD_HELD, d, None, Some((words, true, false)));
            }
        } else if let Some(h) = hidden {
            // A worry you hid that came true costs your word with both.
            let words = format!("You never mentioned {}'s {}.", lastn, &h.words()[4..]);
            for who in heard {
                standing::pay(w, prog, who, standing::WORD_BROKEN, d, None, Some((words.clone(), false, false)));
            }
        }
        let text = if held {
            format!("Your word on {lastn} held up at {tname}.")
        } else if gone_early {
            format!("{lastn} didn't last his first year at {tname}. Your word there took the hit.")
        } else {
            format!("{lastn} didn't hold up at {tname} the way you said.")
        };
        crate::world::folk::note(w, "network", tname, text, None);
    }
}

/// Your players who went into the portal: their coaches watch where they
/// land.
fn leavers(w: &mut World, prog: usize, d: Date) {
    let fresh: Vec<(u32, Option<Who>)> = w.programs[prog]
        .roster
        .iter()
        .filter_map(|&i| {
            let p = w.people.get(i)?;
            let e = p.cal.portal?;
            (e.entered <= d && e.entered.days_to(d) <= 14).then(|| (p.player.id, people::head_coach_of(w, p.hs)))
        })
        .collect();
    for (kid, coach) in fresh {
        if !w.ground.favors.leavers.iter().any(|l| l.kid == kid && l.date.days_to(d) < 300) {
            w.ground.favors.leavers.push(Leaver { kid, coach, date: d, judged: false });
        }
    }
}

/// Fall camp: a leaver you helped land well holds his school's standing;
/// one who drifted costs it.
fn judge_leavers(w: &mut World, prog: usize, d: Date) {
    for k in 0..w.ground.favors.leavers.len() {
        let l = w.ground.favors.leavers[k];
        if l.judged || l.date.days_to(d) > 400 {
            continue;
        }
        w.ground.favors.leavers[k].judged = true;
        let (Some(i), Some(coach)) = (crate::world::folk::person(w, l.kid), l.coach) else { continue };
        let p = &w.people[i];
        let landed = (p.stage == Stage::College && p.college.is_some_and(|c| c != prog)) || p.stage == Stage::Pro;
        let helped = w
            .ground
            .favors
            .placed
            .iter()
            .any(|x| x.kid == l.kid && matches!(x.state, State::Signed(_) | State::Settled(_)));
        let lastn = last(w, i);
        if landed && helped {
            let at = p.college.map_or_else(String::new, |c| format!(" at {}", w.program_name(c)));
            let words = format!("You helped {lastn} land{at}.");
            standing::pay(w, prog, coach, LEAVER_HELD, d, None, Some((words, true, false)));
        } else if !landed && p.stage != Stage::College {
            let words = format!("{lastn} drifted after he left you.");
            standing::pay(w, prog, coach, LEAVER_DRIFT, d, None, Some((words, false, false)));
        }
    }
}

/// Your coordinator places the routine no's by your rule (GR63): the kids
/// whose offers you pulled and your players in the portal. A Trusted or
/// Home school's kid comes to you instead.
fn coordinator(w: &mut World, prog: usize, d: Date) {
    let calls = w.ground.favors.rule.calls as usize;
    let Some(coord) = super::week::coordinator(w, prog) else { return };
    let season = w.dates.season;
    let mut kids: Vec<u32> = w
        .front_office
        .college
        .board
        .iter()
        .filter(|t| t.pulled.is_some_and(|x| x.days_to(d) <= 120))
        .map(|t| t.id)
        .collect();
    kids.extend(
        w.ground
            .favors
            .leavers
            .iter()
            .filter(|l| !l.judged)
            .filter(|l| crate::world::folk::person(w, l.kid).is_some_and(|i| w.people[i].cal.portal.is_some()))
            .map(|l| l.kid),
    );
    for kid in kids {
        let Some(i) = crate::world::folk::person(w, kid) else { continue };
        if w.ground.favors.placed.iter().any(|x| x.kid == kid && x.season == season) {
            continue;
        }
        if !w.ground.once(key(&[0xFA70, kid as u64, season as u64])) {
            continue;
        }
        let lv = sender_of(w, i).map_or(Level::Stranger, |s| standing::level(w, prog, s));
        if lv >= Level::Trusted {
            let school = w.school_name(w.people[i].hs).to_string();
            let subject = format!("Yours to call: {}", w.people[i].name);
            let body = vec![format!(
                "{} is from {school}, and we're {} there. I didn't place him: that call is yours.",
                w.people[i].name,
                lv.name()
            )];
            if let Some(s) = sender_of(w, i) {
                standing::post(w, s, d, crate::world::desk::Head::Memo, subject, body, 1);
            }
            continue;
        }
        for j in takers(w, kid, calls) {
            let Some(man) = w.programs[j].head_coach else { continue };
            let _ = place(w, kid, man, 0, None, Some(coord));
        }
    }
}

/// Whether a man calling you this week is calling in what you owe him
/// (`folk/network.rs`, `asks_week`): a weekly chance, and always once the
/// debt is about a year old.
pub(crate) fn calls_in(w: &World, c: usize, d: Date) -> bool {
    if !on(w) {
        return false;
    }
    let Some(x) = w.folk.network.contacts.iter().find(|x| x.coach_id() == Some(c)) else { return false };
    if x.balance >= 0 {
        return false;
    }
    let oldest =
        w.folk.network.ledger.iter().filter(|f| f.coach == c && f.other.is_none() && f.asked).map(|f| f.date).min();
    let who = Who::Coach(c as u32);
    let mem = you(w).and_then(|y| w.ground.get(who, Holder::Coach(y as u32)));
    let oldest =
        oldest.into_iter().chain(mem.into_iter().flat_map(|m| m.favors.iter().filter(|f| f.his).map(|f| f.date))).min();
    oldest.is_some_and(|o| o.days_to(d) >= CALL_IN_BY)
        || unit(w.options.seed, &[0xFAC1, c as u64, d.0 as u64]) < CALL_IN_WEEK
}

/// The people you owe, for the weekly call: (coach id), oldest debt first.
pub(crate) fn creditors(w: &World) -> Vec<usize> {
    if !on(w) {
        return Vec::new();
    }
    w.folk.network.contacts.iter().filter(|x| x.balance < 0).filter_map(|x| x.coach_id()).collect()
}

/// You said no to a man calling in what you owed him (or never called
/// back): a broken promise (GR24).
pub(crate) fn refused(w: &mut World, c: usize) {
    if !on(w) {
        return;
    }
    let Some(prog) = standing::seat_program(w) else { return };
    let d = w.today;
    let who = Who::Coach(c as u32);
    if people::kind(w, who) == Kind::College {
        crate::world::folk::network::touch(w, c, -0.15);
        return;
    }
    let words = "I called in what you owed me, and you said no.".to_string();
    standing::pay(w, prog, who, standing::PROMISE_BROKEN, d, None, Some((words, false, true)));
}

/// What he owes you fades slowly if you never ask (one a year after two
/// quiet years); what you owe never does.
fn fade(w: &mut World, d: Date) {
    let Some(me) = you(w) else { return };
    let yh = Holder::Coach(me as u32);
    for m in w.ground.memories.iter_mut() {
        if !matches!(m.holder, Holder::Coach(_)) || m.owes <= 0 {
            continue;
        }
        let latest = m.favors.iter().map(|f| f.date).max().unwrap_or(m.met);
        if latest.days_to(d) >= OWED_FADE {
            m.owes -= 1;
        }
    }
    // College colleagues keep theirs in the network.
    let ledger = &w.folk.network.ledger;
    let stale: Vec<usize> = w
        .folk
        .network
        .contacts
        .iter()
        .enumerate()
        .filter(|(_, x)| x.balance > 0)
        .filter(|(_, x)| {
            x.coach_id().is_some_and(|c| {
                people::kind(w, Who::Coach(c as u32)) == Kind::College
                    && ledger
                        .iter()
                        .filter(|f| f.coach == c)
                        .map(|f| f.date)
                        .max()
                        .is_none_or(|l| l.days_to(d) >= OWED_FADE)
            })
        })
        .map(|(k, _)| k)
        .collect();
    for k in stale {
        w.folk.network.contacts[k].shift(-1);
        if let Some(c) = w.folk.network.contacts[k].coach_id()
            && let Some(m) = w.ground.get_mut(Who::Coach(c as u32), yh)
        {
            m.owes = (m.owes - 1).max(0);
        }
    }
}

// ---------------------------------------------------------------------------
// The board (for the screens).

/// One line on the favor board, in words.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Line {
    pub who: Who,
    pub name: String,
    pub role: String,
    pub words: String,
}

/// One kid on the board: your haves (a kid you passed on or a leaver), or
/// a kid you called about.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Kid {
    pub id: u32,
    pub name: String,
    pub position: String,
    pub school: String,
    pub words: String,
}

/// The favor board beside the map (placement.md, "A week of it"): your
/// haves, the calls in flight, who owes you and whom you owe, in words.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Board {
    pub on: bool,
    pub haves: Vec<Kid>,
    pub calls: Vec<Kid>,
    pub owed: Vec<Line>,
    pub owe: Vec<Line>,
    pub rule: String,
}

pub fn board(w: &World) -> Board {
    let mut b = Board {
        on: on(w),
        haves: Vec::new(),
        calls: Vec::new(),
        owed: Vec::new(),
        owe: Vec::new(),
        rule: String::new(),
    };
    if !b.on {
        return b;
    }
    let Some(prog) = standing::seat_program(w) else { return b };
    let season = w.dates.season;
    let kid = |i: usize, words: String| Kid {
        id: w.people[i].player.id,
        name: w.people[i].name.clone(),
        position: w.people[i].player.position.name().to_string(),
        school: match w.people[i].college {
            Some(c) if w.people[i].stage == Stage::College => w.program_name(c).to_string(),
            _ => w.school_name(w.people[i].hs).to_string(),
        },
        words,
    };
    let placed = |id: u32| w.ground.favors.placed.iter().any(|x| x.kid == id && x.season == season);
    for t in &w.front_office.college.board {
        if t.pulled.is_some()
            && !placed(t.id)
            && let Some(i) = crate::world::folk::person(w, t.id)
            && w.people[i].cal.signed.is_none()
        {
            b.haves.push(kid(i, "You pulled his offer: he needs a home by signing day.".into()));
        }
    }
    for i in w.programs[prog].roster.iter().copied() {
        if w.people[i].cal.portal.is_some() && !placed(w.people[i].player.id) {
            b.haves.push(kid(i, "In the portal, not landed.".into()));
        }
    }
    for x in w.ground.favors.placed.iter().rev().take(40) {
        let Some(i) = crate::world::folk::person(w, x.kid) else { continue };
        let tname = w.program_name(x.taker as usize);
        let words = match x.state {
            State::Looked => format!("{tname} is looking."),
            State::Signed(_) => format!("Signed with {tname}; your word settles later."),
            State::Settled(true) => format!("Your word held up at {tname}."),
            State::Settled(false) => format!("He didn't hold up at {tname}."),
            State::Lost => format!("{tname} passed, or he went elsewhere."),
        };
        b.calls.push(kid(i, words));
    }
    let Some(me) = you(w) else { return b };
    let mut holders = vec![Holder::Coach(me as u32)];
    if let Some(c) = super::week::coordinator(w, prog) {
        holders.push(Holder::Coach(c as u32));
    }
    let mut seen: Vec<Who> = Vec::new();
    for m in &w.ground.memories {
        if !holders.contains(&m.holder) || seen.contains(&m.who) {
            continue;
        }
        let owes = balance(w, m.who, m.holder);
        let mut words = match owes {
            0 => continue,
            1 => "He owes you one".to_string(),
            2..=3 => "He owes you a big one".to_string(),
            x if x > 3 => "He owes you more than he can repay".to_string(),
            -1 => "You owe him one".to_string(),
            -3..=-2 => "You owe him a big one".to_string(),
            _ => "You owe him more than you can repay".to_string(),
        };
        if let Some(f) = m.favors.last() {
            words.push_str(&format!(
                ", for {}",
                f.words.trim_start_matches("he did you a favor: ").trim_start_matches("you did him a favor: ")
            ));
        }
        if m.holder != Holder::Coach(me as u32) {
            words.push_str(" (your coordinator's)");
        }
        seen.push(m.who);
        let line = Line { who: m.who, name: people::name(w, m.who), role: people::role(w, m.who), words };
        if owes > 0 {
            b.owed.push(line);
        } else {
            b.owe.push(line);
        }
    }
    b.rule = match w.ground.favors.rule.calls {
        0 => "Your coordinator places nobody: every call is yours.".into(),
        1 => "Every kid we pass on gets a call; the Trusted and Home schools are yours.".into(),
        n => format!("Every kid we pass on gets {n} calls; the Trusted and Home schools are yours."),
    };
    b
}
