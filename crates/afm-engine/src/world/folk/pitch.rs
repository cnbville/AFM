//! The recruit and portal conversation (talk facts design, section 6, "How
//! a call flows"; lane L5): your home visit with a high school recruit and
//! whoever decides for him, and your call, home visit and campus sit-down
//! with a transfer, all on the fact sheet (`claims::recruit`).
//!
//! 1. **He opens on what changed** since you last spoke (an event on his
//!    sheet he hasn't talked through with you), else on where you stand:
//!    the first talk, a while since you called, your last season.
//! 2. **Each exchange** he asks about a group (what he weighs most, unless
//!    he dresses it up) or leaves you room. You get up to three answers,
//!    your best proof in that group with your staff's read of its strength;
//!    "be straight" when your proof is weak; free questions that use no
//!    exchange (what he's looking for, who decides or why he left, what his
//!    other schools tell him, how things are at home); "bring up something
//!    else", every group of yours.
//! 3. **He reacts** and says why: it is what he came for, a rival told him
//!    the same, he remembers it, it isn't his thing. His words can differ
//!    from what reached him, and your read of people decides whether you
//!    catch it.
//! 4. **A promise** for a transfer, as built (`front_office::portal`).
//! 5. **The file keeps** what he brought up, how he took each point and
//!    what he told you (`pulls`), and the school's case keeps what landed
//!    (`case`).
//!
//! A call is `case::CALL_EXCHANGES` exchanges, a sit-down
//! `case::SIT_DOWN_EXCHANGES`. Handed to your staff, the position coach
//! makes the case in one step, as every AI staff does, and brings back what
//! his own read caught.
//!
//! Lines: each part tries the design's new key first (section 8) and falls
//! back to the recruiting lines' parts and then to today's lines, so the
//! talk plays before the bank run fills the new parts. Hidden numbers (his
//! wants, the lean, the case) pick parts and never appear in a line.
//!
//! Every size here is Claude's (proposed, not confirmed).

use super::case::{self, CALL_EXCHANGES, Case, PHONE, SIT_DOWN_EXCHANGES};
use super::claims::{self, Ear, Family, Group, Kind, Point, Sheet, Tier};
use super::dialogue::{self, ON_CAMPUS};
use super::pulls::{Ev, Pull, Where};
use super::scene::{self, Kind as Scenes, Pick, PickKind, Scene};
use super::voice::{self, Facts, Speaker};
use crate::player::{PlayingTime, Trait};
use crate::world::background::{Bloodline, Decider, FamilyStructure};
use crate::world::front_office::portal;
use crate::world::{Stage, World};

/// After his first question, how often an exchange opens on another
/// question of his (else he leaves you room). Proposed, not confirmed.
pub const ASK_SHARE: f64 = 0.65;
/// How far chance moves which group he asks about, against how much he
/// weighs it. Proposed, not confirmed.
pub const ASK_NOISE: f32 = 0.2;
/// The answers shown to his question. Proposed, not confirmed.
pub const ANSWERS: usize = 3;
/// He lights up at a point that lands from this much care (1.0 is a want
/// of average weight). Proposed, not confirmed.
pub const LIT_CARE: f32 = 1.15;
/// Under this much care a point isn't his thing. Proposed, not confirmed.
pub const COOL_CARE: f32 = 0.8;
/// A money-minded recruit with a handler or greed this high dresses up
/// what he wants this often: he says development (section 7, "about 1 in
/// 3"). Proposed, not confirmed.
pub const DRESS_RATE: f64 = 1.0 / 3.0;
/// The greed (1-20) from which a money-minded recruit may dress it up.
/// Proposed, not confirmed.
pub const DRESS_GREED: f64 = 14.0;
/// Recruits who keep it close: a point that reached them reads as polite.
/// Proposed, not confirmed.
pub const GUARDED_SHARE: f64 = 0.25;
/// Recruits who are polite about everything: a point that isn't their
/// thing reads as polite. Proposed, not confirmed.
pub const POLITE_SHARE: f64 = 0.3;
/// How often a read catches his words and his reaction parting, from a
/// people rating of 1 to one of 20. Proposed, not confirmed.
pub const CATCH: (f64, f64) = (0.1, 0.9);
/// He doesn't know what matters most when his top two weigh within this.
/// Proposed, not confirmed.
pub const UNSURE_GAP: f32 = 0.15;
/// He draws a line from this much care for one thing. Proposed, not
/// confirmed.
pub const DEALBREAKER: f32 = 2.0;
/// A talk after this many days of nothing opens on the silence. Proposed,
/// not confirmed.
pub const SILENCE_DAYS: i32 = 42;
/// A season of this many wins is a good year to bring up, this few a bad
/// one. Proposed, not confirmed.
pub const GOOD_YEAR: u32 = 9;
/// A bad year: this many wins or fewer. Proposed, not confirmed.
pub const BAD_YEAR: u32 = 4;
/// A talk closes warm when its case moved this much at a sit-down (half
/// on a call). Proposed, not confirmed.
pub const CLOSE_WARM: f32 = 0.01;
/// His ambition (1-20) that makes him "driven to reach the league" at
/// home. Proposed, not confirmed.
pub const AMBITIOUS: f64 = 15.0;

// The picks' codes. 709, 711, 712 and 719 keep today's meanings.
const LOOKING: u32 = 709;
const PROMISE_START: u32 = 711;
const PROMISE_ROLE: u32 = 712;
const LEAVE: u32 = 719;
const POINT: u32 = 7100;
const STRAIGHT: u32 = 7300;
const GROUP: u32 = 7500;
const BACK: u32 = 7509;
const ELSE: u32 = 7510;
const PROMISE_MENU: u32 = 7520;
/// Straight about a group you have no point in (7530 + the group).
const STRAIGHT_GROUP: u32 = 7530;
const DECIDES: u32 = 7601;
const OTHERS: u32 = 7602;
const HOME: u32 = 7603;

/// `St.ask` for a question about a group you have no point in: this plus
/// the group.
const ASK_GROUP: u8 = 200;

/// The view the picks show (`St.nav`).
const NAV_ASKED: u8 = 0;
const NAV_MENU: u8 = 9;
const NAV_PROMISE: u8 = 8;

/// A talk's state between answers, kept in the scene's `c`.
#[derive(Clone, Copy, Debug, Default)]
struct St {
    /// Exchanges used.
    used: u8,
    /// The point his question is about (`Kind::ALL` place + 1), `ASK_GROUP`
    /// + the group when you have no point in it, 0 when he left you room.
    ask: u8,
    /// What the picks show: his question, a group (1-7), the group menu,
    /// the promise.
    nav: u8,
    /// Free questions asked: what he's after, who decides (why he left),
    /// the others, home.
    free: u8,
    /// Answers given straight in this talk.
    straight: u8,
    /// Groups he asked about.
    asked: u8,
    /// Groups you made a point in.
    made: u8,
    /// 1 once the talk has wound down to the promise.
    stage: u8,
}

impl St {
    fn of(s: &Scene) -> St {
        let c = s.c.max(0) as u64;
        St {
            used: (c & 0xF) as u8,
            ask: (c >> 4 & 0xFF) as u8,
            nav: (c >> 12 & 0xF) as u8,
            free: (c >> 16 & 0xF) as u8,
            straight: (c >> 20 & 0xFF) as u8,
            asked: (c >> 28 & 0x7F) as u8,
            made: (c >> 36 & 0x7F) as u8,
            stage: (c >> 44 & 0x3) as u8,
        }
    }
    fn put(self, s: &mut Scene) {
        s.c = ((self.used as u64 & 0xF)
            | (self.ask as u64) << 4
            | (self.nav as u64 & 0xF) << 12
            | (self.free as u64 & 0xF) << 16
            | (self.straight as u64) << 20
            | (self.asked as u64 & 0x7F) << 28
            | (self.made as u64 & 0x7F) << 36
            | (self.stage as u64 & 0x3) << 44) as i64;
    }
}

/// Who the talk is with, and where.
struct Talk {
    i: usize,
    id: u32,
    j: usize,
    transfer: bool,
    ear: Ear,
    at: Where,
}

impl Talk {
    fn phone(&self) -> bool {
        self.at == Where::Call
    }
    fn scale(&self) -> f32 {
        if self.phone() { PHONE } else { 1.0 }
    }
    fn limit(&self) -> u8 {
        (if self.phone() { CALL_EXCHANGES } else { SIT_DOWN_EXCHANGES }) as u8
    }
    fn setting(&self) -> &'static str {
        if self.phone() { "call" } else { "visit" }
    }
}

fn talk(w: &World, s: &Scene) -> Option<Talk> {
    let id = s.a as u32;
    let i = super::person(w, id)?;
    let j = s.b as usize;
    if j >= w.programs.len() {
        return None;
    }
    let (transfer, ear, at) = match s.kind {
        Scenes::HomeVisit => (false, Ear::Decider, Where::Home),
        Scenes::PortalCall => (true, Ear::Himself, Where::Call),
        Scenes::PortalVisit => (true, Ear::Himself, if s.f[1] == ON_CAMPUS { Where::Campus } else { Where::Home }),
        _ => return None,
    };
    Some(Talk { i, id, j, transfer, ear, at })
}

/// Why there is nothing left to talk about, if so.
fn over(w: &World, t: &Talk) -> Option<&'static str> {
    let p = &w.people[t.i];
    if t.transfer {
        p.cal.portal.is_none().then_some("He has made his choice: there is nothing left to talk about.")
    } else {
        (p.stage != Stage::HighSchool || p.cal.signed.is_some())
            .then_some("He has signed: there is nothing left to talk about.")
    }
}

fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

/// Who speaks for him: a transfer himself; at a high school recruit's
/// home whoever decides, in that voice (a recruit who decides himself
/// speaks as himself, never as a parent).
fn speaker(w: &World, t: &Talk) -> Speaker {
    if t.transfer {
        return Speaker::player(w, t.i);
    }
    let mut sp = Speaker::parent(w, t.i);
    match w.people[t.i].family.decider {
        Decider::Himself => {
            sp.tags.retain(|&x| x != "parent");
            sp.tags.push("himself");
        }
        Decider::Grandparent => sp.tags.push("grandparent"),
        Decider::Handler => sp.tags.push("handler"),
        Decider::Parent => {}
    }
    sp
}

/// The name on his side's lines.
fn who(w: &World, t: &Talk) -> String {
    let p = &w.people[t.i];
    if t.transfer || p.family.decider == Decider::Himself {
        "Him".into()
    } else {
        dialogue::decider_words(p).to_string()
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

fn first(n: &str) -> String {
    n.split_whitespace().next().unwrap_or(n).to_string()
}

/// The facts every line of the talk may use: his first name, your school,
/// you, his position.
fn base(w: &World, t: &Talk) -> Facts {
    let p = &w.people[t.i];
    let me = scene::your_name(w);
    let last = me.split_whitespace().last().unwrap_or(&me).to_string();
    Facts::new()
        .with("name", first(&p.name))
        .with("team", w.program_name(t.j).to_string())
        .with("coach", format!("Coach {last}"))
        .with("you", last)
        .with("pos", p.player.position.to_string())
}

/// One line from the first part in `keys` the speaker can say.
fn say(w: &mut World, s: &Scene, k: u64, keys: &[String], sp: &Speaker, f: &Facts) -> Option<String> {
    let mut pen = scene::pen(w, s, k);
    let out = keys.iter().find_map(|key| pen.part(key, sp, f)).map(|x| voice::tidy(&x));
    pen.put(w);
    out
}

/// The point a pick stands on, by its place in the catalogue.
fn kind_at(n: u32) -> Option<Kind> {
    Kind::ALL.get(n as usize).copied()
}

fn place_of(k: Kind) -> u32 {
    Kind::ALL.iter().position(|&x| x == k).unwrap_or(0) as u32
}

/// His words for a group, when he tells you what he weighs.
fn told_words(g: Group) -> &'static str {
    match g {
        Group::PlayingTime => "playing time",
        Group::Money => "the money",
        Group::School => "the school",
        Group::Home => "staying close to home",
        Group::People => "the coaches",
        Group::League => "getting to the league",
        Group::Winning => "winning",
    }
}

/// Today's pitch key for a group (the line bank's `homevisit.ask.<drive>`
/// and `portal.ask.<drive>`).
fn drive(g: Group) -> &'static str {
    match g {
        Group::PlayingTime => "playing",
        Group::Money => "nil",
        Group::School => "school",
        Group::Home => "distance",
        Group::People => "family",
        Group::League => "development",
        Group::Winning => "winning",
    }
}

/// The recruiting lines' aspect a point's reaction can borrow.
fn aspect(k: Kind) -> Option<&'static str> {
    Some(match k {
        Kind::Stadium => "atmosphere",
        Kind::Exposure => "brand",
        Kind::AfterFootball => "campus",
        Kind::Conference => "conference",
        Kind::Buildings => "facilities",
        Kind::Tenure | Kind::YourGuys => "prestige",
        Kind::Titles | Kind::PositionU => "tradition",
        _ => return None,
    })
}

fn bit(g: Group) -> u8 {
    1 << g as u8
}

// ---------------------------------------------------------------------------
// How he is.

fn care(w: &World, t: &Talk) -> [f32; 7] {
    claims::group_cares(&w.people[t.i], t.ear)
}

/// What he weighs most.
fn top_group(c: &[f32; 7]) -> Group {
    Group::ALL.into_iter().max_by(|a, b| c[*a as usize].total_cmp(&c[*b as usize]).then(b.cmp(a))).unwrap()
}

/// He follows the money and says development (section 7): money-minded,
/// with a handler or greed of `DRESS_GREED`, about `DRESS_RATE` of them.
fn dresser(w: &World, t: &Talk) -> bool {
    let p = &w.people[t.i];
    let c = care(w, t);
    if top_group(&c) != Group::Money {
        return false;
    }
    let handler = t.ear == Ear::Decider && p.family.decider == Decider::Handler;
    // hidden: his greed is a gate only.
    let greedy = p.player.personality.traits.get(Trait::Greed) >= DRESS_GREED;
    (handler || greedy) && super::roll(w, 0xD2E5, p.player.id as u64) < DRESS_RATE
}

fn guarded(w: &World, t: &Talk) -> bool {
    super::roll(w, 0x6A2D, w.people[t.i].player.id as u64) < GUARDED_SHARE
}

fn polite(w: &World, t: &Talk) -> bool {
    super::roll(w, 0x9017, w.people[t.i].player.id as u64) < POLITE_SHARE
}

/// What a point did, and what he says it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Took {
    /// It reached him.
    Lit,
    /// It landed, without moving him much.
    Flat,
    /// He cares and didn't believe you (you oversold it).
    Doubted,
    /// It isn't his thing.
    Cool,
    /// Words only: polite.
    Polite,
}

fn truth(care: f32, lean: f32, tier: Tier) -> Took {
    if care < COOL_CARE {
        Took::Cool
    } else if tier == Tier::Weak && care > 1.0 && lean <= 0.0 {
        Took::Doubted
    } else if lean > 0.0 && care >= LIT_CARE {
        Took::Lit
    } else {
        Took::Flat
    }
}

/// What he says a point did: the same, unless he dresses up the money,
/// keeps it close or is polite about everything.
fn shown(w: &World, t: &Talk, g: Group, truth: Took) -> Took {
    if dresser(w, t) {
        if g == Group::Money && truth == Took::Lit {
            return Took::Cool;
        }
        if g == Group::League {
            return Took::Lit;
        }
    }
    match truth {
        Took::Lit if guarded(w, t) => Took::Polite,
        Took::Cool if polite(w, t) => Took::Polite,
        x => x,
    }
}

/// Whether a read catches his words parting from his reaction: your own
/// people rating, or the coach's when your staff has the talk.
fn catches(w: &World, coach: Option<usize>, salt: u64) -> bool {
    let r = match coach {
        Some(c) => w.coaches[c].ability().relationships,
        None => w.career.as_ref().map_or(10.0, |c| c.you.relationships),
    };
    let p = CATCH.0 + (CATCH.1 - CATCH.0) * ((r - 1.0) / 19.0).clamp(0.0, 1.0);
    super::roll(w, 0xCA7C ^ salt, w.today.0 as u64) < p
}

/// The evidence his words leave on the file.
fn words_ev(x: Took) -> Ev {
    match x {
        Took::Lit => Ev::TookWell,
        Took::Doubted => Ev::Doubted,
        Took::Cool => Ev::Cool,
        Took::Flat | Took::Polite => Ev::Polite,
    }
}

/// A row on his file.
fn note(w: &mut World, t: &Talk, group: u8, ev: Ev, by: Option<usize>) {
    let today = w.today;
    let decider = super::pulls::decider_rows(w, t.i) && t.ear == Ear::Decider;
    let from = if by.is_some() { Where::Staff } else { t.at };
    w.folk.pulls.note(today, Pull { who: t.id, group, ev, from, on: today, by, decider });
}

/// The case this talk adds to.
fn case_for<'a>(w: &'a mut World, t: &Talk) -> Option<&'a mut Case> {
    if t.transfer { portal::case_mut(w, t.id) } else { Some(case::case_mut(w, t.i, t.j)) }
}

/// What reaches him from what your staff can back up, for a staff's case
/// in one step.
fn backing(w: &World, t: &Talk) -> [f64; 7] {
    if t.transfer {
        let (role, nil) = portal::place(w, t.i, t.j);
        case::transfer_backing(w, t.i, t.j, role, nil)
    } else {
        case::backing(w, t.i, t.j)
    }
}

// ---------------------------------------------------------------------------
// Your words.

fn tier_words(t: Tier) -> &'static str {
    match t {
        Tier::Elite => "the best on his list",
        Tier::Strong => "among the best on his list",
        Tier::Fair => "the middle of his list",
        Tier::Weak => "weak next to his list: you'd be overselling it",
    }
}

fn commas(s: &str) -> String {
    let Ok(n) = s.parse::<i64>() else { return s.to_string() };
    let d = n.abs().to_string();
    let mut out = String::new();
    for (k, c) in d.chars().enumerate() {
        if k > 0 && (d.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// Your point in your own words when the bank has none for it: what the
/// fact is, and how it stands on his list. Only what the sheet holds.
fn claim_words(pt: &Point, tier: Tier) -> String {
    // A fact the words need and the point lacks: plain words instead, never
    // a sentence with a hole in it.
    let missing = std::cell::Cell::new(false);
    let g = |h: &str| match pt.get(h).filter(|x| !x.is_empty()) {
        Some(x) => x.to_string(),
        None => {
            missing.set(true);
            String::new()
        }
    };
    let n = |h: &str| match pt.get(h).and_then(|x| x.parse::<i64>().ok()) {
        Some(x) => x,
        None => {
            missing.set(true);
            0
        }
    };
    let opt = |h: &str| pt.get(h).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    let spot = g("spot");
    let said = match pt.kind {
        Kind::Room => match pt.check("depth") {
            Some("thin") => format!(
                "We're thin at {spot}: {} coming back for {} starting spots. You'd compete from day one.",
                n("staying"),
                n("starters")
            ),
            Some("crowded") => {
                format!("We have {} at {spot} coming back. You'd have to earn it, and you can.", n("staying"))
            }
            _ => format!("There's room at {spot}: {} coming back and {} moving on.", n("staying"), n("leaving")),
        },
        Kind::Role => match pt.check("role") {
            Some("starter") => format!("We see you starting at {spot}."),
            Some("rotation") => format!("We see you in the rotation at {spot} from the start."),
            _ => format!("You'd come in as depth at {spot} and push from there."),
        },
        Kind::QbRivals => match pt.get("rival") {
            Some(r) => format!("{r} is in the room. Beat him out and the job is yours."),
            None => "No blue-chip quarterback stands between you and the job here.".into(),
        },
        Kind::Scheme => {
            if pt.is("new", "yes") {
                format!("We're putting in {}, and a {spot} like you fits it.", g("system"))
            } else {
                format!("We run {}. Let me show you where a {spot} fits it.", g("system"))
            }
        }
        Kind::OfferRate => match pt.check("gap") {
            Some("over") => format!("Our offer is over the going rate at {spot}."),
            Some("at") => format!("Our offer is right at the going rate at {spot}."),
            _ => "Our offer is real, and it's in writing.".into(),
        },
        Kind::OfferOthers => "Put our offer next to the others before you decide.".into(),
        Kind::FamilyNeed => "This offer can take some weight off your family.".into(),
        Kind::Brand => {
            if pt.is("name", "big") {
                "You have a name already. Here it grows.".into()
            } else {
                "You've built a following. We'll help you grow it.".into()
            }
        }
        Kind::Degree => match pt.check("academics") {
            Some("top") | Some("research") => {
                format!("A degree from {} opens doors for the rest of your life.", g("school"))
            }
            _ => format!("You'll leave {} with a degree.", g("school")),
        },
        Kind::Support => "Our academic people will keep you on track and eligible.".into(),
        Kind::AfterFootball => {
            format!("Life after football starts here: {} and {} look after their own.", g("school"), g("town"))
        }
        Kind::Drive => {
            if pt.is("drive", "day") {
                format!("{} is {} miles from here. Your family can make every home game.", g("home"), n("miles"))
            } else {
                format!("It's about {} hours to {}. We'll help your family get here.", n("hours"), g("home"))
            }
        }
        Kind::ChildhoodTeam => format!("You grew up cheering for {}. Now you can wear it.", g("school")),
        Kind::HomeState => format!("Stay home. Play in front of {}.", g("state")),
        Kind::HsMates => format!("{} from {} are here already. You'd walk in with friends.", g("names"), g("hs")),
        Kind::Brother => format!("Your brother is here at {}.", g("school")),
        Kind::Tenure => match pt.check("seat") {
            Some("long") => format!("I've been here {}. I'm not going anywhere.", g("seasons")),
            Some("new") => "I'm new here, and I'm building this around players like you.".into(),
            Some("hot") => "People talk about my job. I'm here to win now, and I need players like you.".into(),
            _ => format!("I've been here {}, and we're just getting started.", g("seasons")),
        },
        Kind::PositionCoach => {
            let mut x = format!("{} would coach you. He has been doing it {} years", g("pc"), n("years"));
            if opt("picks") > 0 {
                x.push_str(&format!(" and has sent {} to the league", opt("picks")));
            }
            x.push('.');
            if let (Some(b), Some(r)) = (pt.get("best"), pt.get("best_round")) {
                x.push_str(&format!(" {b} went in the {r} round."));
            }
            x
        }
        Kind::HsCoachTie => match pt.get("hs_coach") {
            Some(c) => format!("{c} and I go back. {} kids do well here.", g("hs")),
            None => format!("{} kids do well here.", g("hs")),
        },
        Kind::YourWord => "Ask anyone I've coached: when I promise something, I keep it.".into(),
        Kind::WhyLeft => match pt.check("why") {
            Some("coaching_change") => "Your coach left. I'm not going anywhere.".into(),
            Some("money") => "You want to be paid what you're worth. Let's talk about that.".into(),
            Some("move_up") => "You want to play at a higher level. This is it.".into(),
            Some("graduate") => "You have your degree and a year to play. Make it count here.".into(),
            Some("playing_time") | Some("depth_chart") | Some("cut") => {
                "You want to play. Here you'll get your shot.".into()
            }
            _ => "You want a fresh start. This is the place for it.".into(),
        },
        Kind::PicksAtPosition => format!(
            "We've had {} at {spot} drafted in ten years. {} went in the {} round.",
            g("picks_words"),
            g("best"),
            g("best_round")
        ),
        Kind::PicksOverall => {
            let mut x = format!("We've had {} drafted in the last three drafts", g("picks_words"));
            if opt("first_round") > 0 {
                x.push_str(&format!(", {} in the first round", opt("first_round")));
            }
            x.push('.');
            x
        }
        Kind::YourGuys => format!("I've coached {} players who made the league: {}.", n("guys"), g("names")),
        Kind::PositionU => format!("They call us {}. There's a reason.", g("u")),
        Kind::ProDay => format!("{} NFL clubs came to our pro day.", n("clubs")),
        Kind::Titles => {
            if opt("titles") > 0 {
                format!("We've won {} national titles here.", opt("titles"))
            } else if opt("conference_titles") > 0 {
                format!("We've won {} conference titles here.", opt("conference_titles"))
            } else if pt.get("last_record").is_some() {
                format!("We went {} last season.", g("last_record"))
            } else {
                "There's a tradition here. You feel it the first day on campus.".into()
            }
        }
        Kind::TitleOdds => match pt.check("level") {
            Some("contender") => "We're playing for a title this year.".into(),
            Some("in_the_hunt") => "We're in the hunt this year.".into(),
            _ => "We're building something you can be part of.".into(),
        },
        Kind::Stadium => format!("{} people fill our place in {} on Saturdays.", commas(&g("seats")), g("town")),
        Kind::Exposure => {
            if pt.is("power", "yes") {
                "We're on national TV most Saturdays. Scouts see every snap.".into()
            } else {
                "Our games are on TV, and the scouts know where to find us.".into()
            }
        }
        Kind::Conference => format!("You'd play in the {} every week.", g("conference")),
        Kind::Rival => match pt.check("series") {
            Some("ahead") => format!("{} against {}: we lead that series.", g("game"), g("rival")),
            _ => format!("{} against {} is the game of the year here.", g("game"), g("rival")),
        },
        Kind::Buildings => {
            if pt.is("stage", "open") {
                format!("We just opened {}.", g("building"))
            } else {
                format!("{} opens in {}.", g("building"), n("opens"))
            }
        }
        _ => format!("Let me tell you about {}.", pt.kind.group().map_or("us", |g| g.name()).to_lowercase()),
    };
    let said = if missing.get() {
        format!("Let me tell you about {}.", pt.kind.group().map_or("us", |g| g.name()).to_lowercase())
    } else {
        said
    };
    let tail = match (pt.proof.is_some(), tier) {
        (true, Tier::Elite) => " Nobody on your list can match that.",
        (true, Tier::Strong) => " Few on your list can.",
        _ => "",
    };
    format!("{said}{tail}")
}

/// Your honest answer when your proof is weak, when the bank has none.
fn straight_words(pt: &Point) -> String {
    group_straight_words(pt.kind.group())
}

fn group_straight_words(g: Option<Group>) -> String {
    let g = g.map_or("this", |g| g.name()).to_lowercase();
    format!("I'll be straight with you: on {g}, others on your list can show you more than we can.")
}

/// "Be straight" about a group you have no point in.
fn straight_group_pick(w: &World, s: &Scene, t: &Talk, g: Group) -> Pick {
    let me = Speaker::you(w);
    let f = base(w, t);
    let look = voice::Look::new(w, (s.id as u64) << 16 ^ 0x5760 ^ g as u64);
    let words = [format!("recruit.pitch.straight.{}", g.key()), "recruit.pitch.straight.any".to_string()]
        .iter()
        .find_map(|k| look.part(k, &me, &f))
        .map(|x| voice::tidy(&x))
        .unwrap_or_else(|| group_straight_words(Some(g)));
    Pick {
        label: format!("\"{words}\""),
        mv: "Be straight".into(),
        tone: "calm".into(),
        hint: "You have nothing to show him there. No pull, and he believes your word a little more.".into(),
        code: STRAIGHT_GROUP + g as u32,
        ..Pick::default()
    }
}

/// Your words for a point: the bank's part for it and your proof, else your
/// own.
fn pitch_words(w: &World, s: &Scene, t: &Talk, pt: &Point, tier: Tier, straight: bool) -> String {
    let me = Speaker::you(w);
    let f = pt.facts(&base(w, t));
    let g = pt.kind.group().map_or("any", |g| g.key());
    let keys: Vec<String> = if straight {
        vec![
            format!("recruit.pitch.straight.{}", pt.kind.key()),
            format!("recruit.pitch.straight.{g}"),
            "recruit.pitch.straight.any".into(),
        ]
    } else {
        vec![format!("recruit.pitch.{}.{}", pt.kind.key(), tier.key())]
    };
    let look = voice::Look::new(w, (s.id as u64) << 16 ^ place_of(pt.kind) as u64 ^ (straight as u64) << 12);
    keys.iter()
        .find_map(|k| look.part(k, &me, &f))
        .map(|x| voice::tidy(&x))
        .unwrap_or_else(|| if straight { straight_words(pt) } else { claim_words(pt, tier) })
}

/// Your staff's note on a point: its read of your proof and, once his file
/// has it, how much the group seems to matter to him.
fn point_hint(w: &World, t: &Talk, pt: &Point, tier: Tier) -> String {
    let mut h = if pt.proof.is_some() {
        format!("Your staff reads this as {}.", tier_words(tier))
    } else {
        "His own story: no school's proof to grade.".to_string()
    };
    if let Some(g) = pt.kind.group() {
        let file = super::pulls::file(w, t.i);
        if let Some(k) = file.iter().position(|r| r.group == g && r.rows > 0) {
            let r = file[k];
            if k < 2 && r.score > 0.0 {
                h.push_str(" His file says it matters to him.");
            } else if r.score < 0.0 {
                h.push_str(" His file says it isn't his thing.");
            }
            if r.conflict {
                h.push_str(" His words and his reactions disagree on it.");
            }
        }
    }
    h
}

// ---------------------------------------------------------------------------
// The talk.

/// Your points he can hear today: the known ones on the sheet, not made
/// already in this talk.
fn on_table<'a>(w: &World, t: &Talk, sheet: &'a Sheet) -> Vec<&'a Point> {
    let today = w.today;
    sheet
        .points
        .iter()
        .filter(|p| p.known && matches!(p.kind.family(), Family::Pitch(_)))
        .filter(|p| !w.folk.talked.row(t.id, p).is_some_and(|r| r.on == today))
        .collect()
}

/// A group's points, your best proof first.
fn best_in<'a>(pts: &[&'a Point], g: Group) -> Vec<&'a Point> {
    let mut v: Vec<&Point> = pts.iter().copied().filter(|p| p.kind.group() == Some(g)).collect();
    v.sort_by(|a, b| {
        let (x, y) = (a.tier().unwrap_or(Tier::Fair), b.tier().unwrap_or(Tier::Fair));
        let (sx, sy) = (a.proof.map_or(0.0, |p| p.strength), b.proof.map_or(0.0, |p| p.strength));
        y.cmp(&x).then(sy.total_cmp(&sx)).then(a.kind.cmp(&b.kind))
    });
    v
}

fn point_pick(w: &World, s: &Scene, t: &Talk, pt: &Point) -> Pick {
    let tier = pt.tier().unwrap_or(Tier::Fair);
    let words = pitch_words(w, s, t, pt, tier, false);
    let g = pt.kind.group().map_or("", |g| g.name());
    Pick {
        label: format!("\"{words}\""),
        mv: g.to_string(),
        tone: "warm".into(),
        hint: point_hint(w, t, pt, tier),
        code: POINT + place_of(pt.kind),
        ..Pick::default()
    }
    .on(pt.kind.key())
}

fn straight_pick(w: &World, s: &Scene, t: &Talk, pt: &Point) -> Pick {
    let words = pitch_words(w, s, t, pt, Tier::Weak, true);
    Pick {
        label: format!("\"{words}\""),
        mv: "Be straight".into(),
        tone: "calm".into(),
        hint: "No pull, and he believes your word a little more.".into(),
        code: STRAIGHT + place_of(pt.kind),
        ..Pick::default()
    }
    .on(pt.kind.key())
}

/// The free questions, the other groups, the promise and the way out.
fn rest_picks(w: &World, t: &Talk, st: &St, out: &mut Vec<Pick>) {
    let free = |code: u32, label: &str, mv: &str, hint: &str| {
        Pick { label: label.into(), mv: mv.into(), tone: "warm".into(), hint: hint.into(), code, ..Pick::default() }
            .of(PickKind::Free)
    };
    if st.free & 1 == 0 {
        out.push(free(
            LOOKING,
            "\"Before I say anything, tell me what you're looking for.\"",
            "Ask him",
            "He tells you what he weighs. It uses no time.",
        ));
    }
    if st.free & 2 == 0 {
        out.push(if t.transfer {
            free(DECIDES, "\"Why did you leave?\"", "Ask him", "His reason, in his words. It uses no time.")
        } else {
            free(DECIDES, "\"Who has the final say in this?\"", "Ask him", "Who decides at home. It uses no time.")
        });
    }
    if st.free & 4 == 0 {
        out.push(free(
            OTHERS,
            "\"What are the other schools telling you?\"",
            "Ask him",
            "What his other schools are selling. It uses no time.",
        ));
    }
    if st.free & 8 == 0 && !t.transfer {
        out.push(free(HOME, "\"How are things at home?\"", "Ask them", "The family's side. It uses no time."));
    }
    out.push(
        Pick {
            label: "Bring up something else".into(),
            mv: "Your points".into(),
            tone: "calm".into(),
            hint: "Every group you can speak to.".into(),
            code: ELSE,
            ..Pick::default()
        }
        .of(PickKind::Nav),
    );
    if t.transfer && portal::promised(w, t.id).is_none() {
        out.push(
            Pick {
                label: "Make him a promise".into(),
                mv: "Promise".into(),
                tone: "firm".into(),
                hint: "A starting job or a real role, checked after he arrives.".into(),
                code: PROMISE_MENU,
                ..Pick::default()
            }
            .of(PickKind::Nav),
        );
    }
    out.push(Pick {
        label: if t.transfer {
            "\"Think it over. Call me anytime.\"".into()
        } else {
            "\"Thank you for having me. Think it over.\"".into()
        },
        mv: "Wrap up".into(),
        tone: "calm".into(),
        hint: "Ends the talk.".into(),
        code: LEAVE,
        ..Pick::default()
    });
}

fn promise_picks(s: &mut Scene, back: bool) {
    s.picks.push(
        Pick {
            label: "\"You'll start for us. You have my word.\"".into(),
            mv: "Promise a starting job".into(),
            tone: "firm".into(),
            hint: "Checked after his first game with you.".into(),
            code: PROMISE_START,
            ..Pick::default()
        }
        // A promise binds you: a typed line that reaches it is asked first.
        .of(PickKind::Final),
    );
    s.picks.push(
        Pick {
            label: "\"You'll have a real role, every week.\"".into(),
            mv: "Promise a real role".into(),
            tone: "warm".into(),
            hint: "Checked at the season's end.".into(),
            code: PROMISE_ROLE,
            ..Pick::default()
        }
        .of(PickKind::Final),
    );
    if back {
        s.picks.push(
            Pick {
                label: "Not yet".into(),
                mv: "Back".into(),
                tone: "calm".into(),
                hint: String::new(),
                code: BACK,
                ..Pick::default()
            }
            .of(PickKind::Nav),
        );
    }
    s.picks.push(Pick {
        label: "\"Think it over. Call me anytime.\"".into(),
        mv: "Leave it there".into(),
        tone: "calm".into(),
        hint: String::new(),
        code: LEAVE,
        ..Pick::default()
    });
}

/// The picks for where the talk stands.
fn picks(w: &World, s: &mut Scene, t: &Talk, st: &St, sheet: &Sheet) {
    s.picks.clear();
    s.reach.clear();
    let pts = on_table(w, t, sheet);
    let mut out: Vec<Pick> = Vec::new();
    if st.nav == NAV_PROMISE || st.stage == 1 {
        promise_picks(s, st.stage == 0);
        return;
    }
    if (1..=7).contains(&st.nav) {
        let g = Group::ALL[st.nav as usize - 1];
        let v = best_in(&pts, g);
        for p in &v {
            out.push(point_pick(w, s, t, p));
        }
        if let Some(p) = v.first().filter(|p| p.tier() == Some(Tier::Weak)) {
            out.push(straight_pick(w, s, t, p));
        }
        out.push(
            Pick { label: "Back".into(), mv: "Back".into(), tone: "calm".into(), code: BACK, ..Pick::default() }
                .of(PickKind::Nav),
        );
    } else if st.nav == NAV_MENU || st.ask == 0 {
        for g in Group::ALL {
            let n = pts.iter().filter(|p| p.kind.group() == Some(g)).count();
            if n == 0 {
                continue;
            }
            out.push(
                Pick {
                    label: g.name().into(),
                    mv: "Bring up".into(),
                    tone: "calm".into(),
                    hint: format!("{n} point{} you can make.", if n == 1 { "" } else { "s" }),
                    code: GROUP + g as u32,
                    ..Pick::default()
                }
                .of(PickKind::Nav),
            );
        }
        if st.nav == NAV_MENU {
            out.push(
                Pick {
                    label: "Back to his question".into(),
                    mv: "Back".into(),
                    tone: "calm".into(),
                    code: BACK,
                    ..Pick::default()
                }
                .of(PickKind::Nav),
            );
        }
        let mut rest = Vec::new();
        rest_picks(w, t, st, &mut rest);
        out.extend(rest.into_iter().filter(|p| p.code != ELSE));
    } else {
        let (asked, g) = match st.ask.checked_sub(ASK_GROUP) {
            Some(k) => (None, Group::ALL[k as usize % 7]),
            None => {
                let k = kind_at(st.ask as u32 - 1);
                (k, k.and_then(Kind::group).unwrap_or(Group::PlayingTime))
            }
        };
        let v = best_in(&pts, g);
        for p in v.iter().take(ANSWERS) {
            out.push(point_pick(w, s, t, p));
        }
        if v.is_empty() {
            out.push(straight_group_pick(w, s, t, g));
        } else if v.first().is_some_and(|p| p.tier() == Some(Tier::Weak)) {
            // Straight about the point he asked, or your best one.
            if let Some(p) = asked.and_then(|k| pts.iter().find(|p| p.kind == k)).or(v.first()) {
                out.push(straight_pick(w, s, t, p));
            }
        }
        rest_picks(w, t, st, &mut out);
    }
    for p in &pts {
        let code = POINT + place_of(p.kind);
        if !out.iter().any(|x| x.code == code) {
            s.reach.push(point_pick(w, s, t, p));
        }
    }
    s.picks = out;
}

/// The group he asks about next (section 7, "What he brings up"): what he
/// weighs most among those not asked yet and not already answered in this
/// talk (with a little chance), whether or not you have a point in it;
/// after his first question, never one that isn't his thing. A dresser
/// asks about the league where he means money; a guarded recruit asks what
/// sounds good, whatever he weighs.
fn asks(w: &World, t: &Talk, asked: u8, made: u8, seed: u64) -> Option<Group> {
    let mut c = care(w, t);
    if dresser(w, t) {
        c[Group::League as usize] = c[Group::League as usize].max(c[Group::Money as usize]);
        c[Group::Money as usize] = c[Group::Money as usize].min(COOL_CARE * 0.9);
    } else if guarded(w, t) {
        c = [1.0; 7];
    }
    let first = asked == 0;
    Group::ALL
        .into_iter()
        .filter(|&g| asked & bit(g) == 0 && made & bit(g) == 0 && (first || c[g as usize] >= COOL_CARE))
        .map(|g| {
            let x = c[g as usize] * (1.0 + ASK_NOISE * crate::game::talk::normal(seed, g as u64) as f32);
            (x, g)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|x| x.1)
}

/// The next exchange: he asks about a group or leaves you room, until the
/// talk's exchanges are used.
fn next_beat(w: &mut World, s: &mut Scene, t: &Talk, mut st: St) {
    if st.used >= t.limit() {
        return wrap_up(w, s, t, st);
    }
    let sheet = claims::recruit(w, t.i, t.j, t.ear);
    let pts = on_table(w, t, &sheet);
    if pts.is_empty() {
        return wrap_up(w, s, t, st);
    }
    let firsts = st.asked == 0;
    let ask_now = firsts || super::roll(w, 0xA5C ^ (s.id as u64) << 8, st.used as u64) < ASK_SHARE;
    let seed = (s.id as u64) << 20 ^ (st.used as u64) << 4 ^ 0xA5C0;
    st.nav = NAV_ASKED;
    let heard = if firsts { Ev::BroughtUp } else { Ev::AskedLater };
    let asked = ask_now.then(|| asks(w, t, st.asked, st.made, seed)).flatten();
    let pt = asked.and_then(|g| {
        pts.iter()
            .copied()
            .filter(|p| p.kind.group() == Some(g))
            .max_by(|a, b| a.mind.total_cmp(&b.mind).then(b.kind.cmp(&a.kind)))
            .cloned()
    });
    match (asked, pt) {
        // He asks about a group you have no point in: you can be straight
        // with him, or bring up something else.
        (Some(g), None) => {
            st.asked |= bit(g);
            st.ask = ASK_GROUP + g as u8;
            let sp = speaker(w, t);
            let keys = vec![
                format!("recruit.ask.{}", g.key()),
                format!("{}.ask.{}", if t.transfer { "portal" } else { "homevisit" }, drive(g)),
            ];
            let ask = say(w, s, 20 + st.used as u64, &keys, &sp, &base(w, t))
                .unwrap_or_else(|| format!("What can you tell me about {}?", told_words(g)));
            let name = who(w, t);
            s.them(&name, ask, None);
            note(w, t, g as u8, heard, None);
        }
        (Some(g), Some(pt)) => {
            st.asked |= bit(g);
            st.ask = place_of(pt.kind) as u8 + 1;
            let sp = speaker(w, t);
            let f = pt.facts(&base(w, t));
            let mut keys = vec![format!("recruit.ask.{}", pt.kind.key())];
            if pt.kind == Kind::PicksAtPosition {
                keys.push("recruit.record.ask".into());
            }
            if pt.kind == Kind::Tenure && pt.is("seat", "new") {
                keys.push("recruit.you.new".into());
            }
            keys.push(format!("{}.ask.{}", if t.transfer { "portal" } else { "homevisit" }, drive(g)));
            let lead = if firsts || super::roll(w, 0x5EC0 ^ s.id as u64, st.used as u64) >= 0.5 {
                None
            } else {
                say(w, s, 10 + st.used as u64, &["recruit.want.second".into()], &sp, &base(w, t))
            };
            let ask = say(w, s, 20 + st.used as u64, &keys, &sp, &f)
                .unwrap_or_else(|| format!("What can you tell me about {}?", told_words(g)));
            let text = match lead {
                Some(l) => format!("{l} {ask}"),
                None => ask,
            };
            let name = who(w, t);
            s.them(&name, text, None);
            note(w, t, g as u8, heard, None);
        }
        (None, _) => {
            st.ask = 0;
            let name = if t.transfer || w.people[t.i].family.decider == Decider::Himself {
                first(&w.people[t.i].name)
            } else {
                dialogue::decider_words(&w.people[t.i]).to_string()
            };
            s.narrate(format!("{name} leaves the floor to you."));
        }
    }
    picks(w, s, t, &st, &sheet);
    st.put(s);
}

/// The exchanges are used: a transfer's promise if none is made, else the
/// close.
fn wrap_up(w: &mut World, s: &mut Scene, t: &Talk, mut st: St) {
    if t.transfer && portal::promised(w, t.id).is_none() && st.stage == 0 {
        st.stage = 1;
        st.nav = NAV_PROMISE;
        s.picks.clear();
        s.reach.clear();
        promise_picks(s, false);
        st.put(s);
        return;
    }
    close(w, s, t, &st);
}

/// The talk ends: his last words, what it came to, and (for a transfer)
/// the talk on his file and whether he has heard enough.
fn close(w: &mut World, s: &mut Scene, t: &Talk, st: &St) {
    let d = s.f[0];
    let mood = if d >= CLOSE_WARM * t.scale() {
        "warm"
    } else if d >= 0.0 {
        "flat"
    } else {
        "cool"
    };
    let sp = speaker(w, t);
    let f = base(w, t);
    if let Some(x) = say(w, s, 90, &[format!("recruit.close.{}.{mood}", t.setting())], &sp, &f) {
        let name = who(w, t);
        s.them(&name, x, None);
    }
    s.outcome.push(
        match mood {
            "warm" => "He leans your way: the talk gave him what he came for.",
            "flat" => "Polite. He'll think it over.",
            _ => "It cooled him: you sold what he doesn't weigh, or what you can't back up.",
        }
        .to_string(),
    );
    finish(w, s, t, d, st.made);
}

/// The talk's end, however it ended: a transfer's talk goes on his file,
/// and he may have heard enough.
fn finish(w: &mut World, s: &mut Scene, t: &Talk, delta: f32, made: u8) {
    s.end();
    if t.transfer {
        let groups: Vec<String> =
            Group::ALL.into_iter().filter(|&g| made & bit(g) != 0).map(|g| g.name().to_lowercase()).collect();
        let said = if groups.is_empty() { "listening".to_string() } else { groups.join(", ") };
        portal::talked(w, t.id, s.f[1], delta, &said);
        if let Some(x) = portal::after_talk(w, t.id) {
            s.outcome.push(x);
        }
    }
}

/// Your point to him: it lands by the case's formula, counts once in a
/// talk and again (smaller) in a later one, and he reacts.
fn make_point(w: &mut World, s: &mut Scene, t: &Talk, mut st: St, k: Kind, picked: &Pick) {
    let sheet = claims::recruit(w, t.i, t.j, t.ear);
    let Some(pt) = sheet.get(k).filter(|p| p.known).cloned() else {
        s.narrate("That's no longer something you can say to him.");
        return next_beat(w, s, t, st);
    };
    let Some(g) = k.group() else { return next_beat(w, s, t, st) };
    let today = w.today;
    let tier = pt.tier().unwrap_or(Tier::Fair);
    let me = scene::your_name(w);
    s.you(&me, picked.label.trim_matches('"'));
    let lean = t.scale() * case::lands(w, pt.care, tier, you(w));
    let again = w.folk.talked.again(today, t.id, &pt);
    let came_back = again < 1.0 && w.folk.talked.remembers(today, t.id, &pt);
    let delta = case_for(w, t).map_or(0.0, |c| c.add(g, lean, again, today));
    s.f[0] += delta;
    w.folk.talked.note(today, t.id, &pt, lean > 0.0, &picked.code.to_string());
    st.made |= bit(g);
    st.used += 1;
    // His reaction: what it did, what he says it did, and whether you
    // catch the difference.
    let real = truth(pt.care, lean, tier);
    let says = shown(w, t, g, real);
    let sp = speaker(w, t);
    let mut f = pt.facts(&base(w, t));
    let rival = claims::rival_for(w, t.i, t.j, k).filter(|r| r.2);
    if let Some((r, ..)) = rival {
        f.set("rival_school", w.program_name(r).to_string());
    }
    let cares = matches!(says, Took::Lit | Took::Doubted);
    let mood = match says {
        Took::Lit => "good",
        Took::Flat | Took::Polite => "flat",
        Took::Doubted | Took::Cool => "bad",
    };
    let mut keys: Vec<String> = Vec::new();
    if came_back && lean > 0.0 {
        keys.push(format!("recruit.react.again.{}", g.key()));
        keys.push("recruit.react.again.any".into());
    }
    if rival.is_some() && says != Took::Lit {
        keys.push(format!("recruit.compare.{}", g.key()));
        keys.push("recruit.compare.any".into());
    }
    keys.push(format!("recruit.react.{}.{}.{}", k.key(), tier.key(), if cares { "cares" } else { "doesnt" }));
    if k == Kind::PicksAtPosition && says != Took::Cool {
        let record = match tier {
            Tier::Elite | Tier::Strong => "strong",
            Tier::Fair => "mixed",
            Tier::Weak => "weak",
        };
        keys.push(format!("recruit.record.{record}"));
    }
    if k == Kind::Tenure && says == Took::Lit && pt.is("seat", "long") {
        keys.push("recruit.you.long".into());
    }
    if let Some(a) = aspect(k) {
        match says {
            Took::Lit if tier >= Tier::Strong => keys.push(format!("recruit.aspect.{a}.good")),
            Took::Doubted => keys.push(format!("recruit.aspect.{a}.weak")),
            Took::Cool | Took::Polite => keys.push(format!("recruit.aspect.{a}.cool")),
            _ => {}
        }
    }
    keys.push(format!("{}.react.{mood}", if t.transfer { "portal" } else { "homevisit" }));
    let line = say(w, s, 30 + st.used as u64, &keys, &sp, &f).unwrap_or_default();
    let parted = real != says && !(real == Took::Flat && says == Took::Polite);
    let caught = parted && catches(w, None, (t.id as u64) << 8 ^ place_of(k) as u64);
    let tell = caught.then(|| {
        if real == Took::Lit {
            "Your read: that reached him, whatever he says.".to_string()
        } else if says == Took::Polite {
            "Your read: he's being polite. It isn't his thing.".to_string()
        } else {
            "Your read: the right words, but it didn't reach him.".to_string()
        }
    });
    let name = who(w, t);
    if !line.is_empty() {
        s.them(&name, line, tell);
    } else if let Some(x) = tell {
        s.narrate(x);
    }
    note(w, t, g as u8, words_ev(says), None);
    if caught {
        note(w, t, g as u8, if real == Took::Lit { Ev::CaughtLit } else { Ev::CaughtFlat }, None);
    }
    s.outcome.push(format!(
        "{}: {}",
        g.name(),
        match says {
            _ if came_back && lean > 0.0 => "he remembered it, and it still helped.",
            Took::Lit => "what he came for.",
            Took::Flat | Took::Polite if rival.is_some() => "another school told him the same.",
            Took::Flat | Took::Polite => "polite.",
            Took::Doubted => "he didn't believe you could back it up.",
            Took::Cool => "not his thing.",
        }
    ));
    next_beat(w, s, t, st);
}

/// Straight with him where your proof is weak: no pull, and he believes
/// your word a little more.
fn be_straight(w: &mut World, s: &mut Scene, t: &Talk, st: St, k: Kind, picked: &Pick) {
    let sheet = claims::recruit(w, t.i, t.j, t.ear);
    let Some(pt) = sheet.get(k).cloned() else { return next_beat(w, s, t, st) };
    let Some(g) = k.group() else { return next_beat(w, s, t, st) };
    straight_on(w, s, t, st, g, Some(&pt), picked)
}

/// A straight answer on a group: about a point whose proof is weak, or a
/// group you have nothing to show him in.
fn straight_on(w: &mut World, s: &mut Scene, t: &Talk, mut st: St, g: Group, pt: Option<&Point>, picked: &Pick) {
    let today = w.today;
    let me = scene::your_name(w);
    s.you(&me, picked.label.trim_matches('"'));
    if let Some(c) = case_for(w, t) {
        c.straight = c.straight.saturating_add(1);
        c.on = c.on.max(today);
    }
    if let Some(pt) = pt {
        w.folk.talked.note(today, t.id, pt, false, "straight");
    }
    st.straight = st.straight.saturating_add(1);
    st.made |= bit(g);
    st.used += 1;
    let cares = care(w, t)[g as usize] >= LIT_CARE;
    let sp = speaker(w, t);
    let f = pt.map_or_else(|| base(w, t), |p| p.facts(&base(w, t)));
    let mut keys = Vec::new();
    if let Some(pt) = pt {
        keys.push(format!("recruit.react.straight.{}.{}", pt.kind.key(), if cares { "cares" } else { "doesnt" }));
    }
    keys.push("recruit.react.straight.any".into());
    keys.push(format!("{}.react.flat", if t.transfer { "portal" } else { "homevisit" }));
    if let Some(x) = say(w, s, 40 + st.used as u64, &keys, &sp, &f) {
        let name = who(w, t);
        s.them(&name, x, None);
    }
    s.outcome
        .push(format!("{}: you were straight with him. No pull, and he believes your word a little more.", g.name()));
    next_beat(w, s, t, st);
}

// ---------------------------------------------------------------------------
// The free questions.

/// "What are you looking for?": his top two in his own words (a dresser
/// names the league for the money), or that he doesn't know yet.
fn looking(w: &mut World, s: &mut Scene, t: &Talk) {
    let c = care(w, t);
    let mut order: Vec<Group> = Group::ALL.to_vec();
    order.sort_by(|a, b| c[*b as usize].total_cmp(&c[*a as usize]).then(a.cmp(b)));
    let dress = dresser(w, t);
    let mut told = order.clone();
    if dress
        && let (Some(m), Some(l)) =
            (told.iter().position(|&g| g == Group::Money), told.iter().position(|&g| g == Group::League))
    {
        told.swap(m, l);
    }
    let me = scene::your_name(w);
    s.you(
        &me,
        if t.transfer {
            "Before I sell you anything, tell me what you're looking for."
        } else {
            "Before I say anything, tell me what you're looking for."
        },
    );
    let sp = speaker(w, t);
    let name = who(w, t);
    let unsure = c[order[0] as usize] - c[order[1] as usize] < UNSURE_GAP;
    // A guarded recruit keeps it close, as a recruit unsure of his own mind
    // says he's still working it out; only your read can tell them apart.
    let close = !dress && guarded(w, t);
    if (unsure || close) && !dress {
        let x = say(w, s, 50, &["recruit.want.unsure".into()], &sp, &base(w, t))
            .unwrap_or_else(|| "Honestly? I'm still working that out.".into());
        let read = if unsure || !catches(w, None, (t.id as u64) << 8 ^ 0x100C) {
            "He doesn't know yet what matters most, or isn't saying."
        } else {
            "Your read: he knows what he wants. He's keeping it to himself."
        };
        s.them(&name, x, Some(read.into()));
        return;
    }
    let (a, b) = (told[0], told[1]);
    let f = base(w, t).with("first", told_words(a)).with("second", told_words(b));
    let keys =
        vec![format!("recruit.tell.{}", a.key()), if t.transfer { "portal.tell" } else { "homevisit.tell" }.into()];
    let x = say(w, s, 51, &keys, &sp, &f).unwrap_or_else(|| format!("{}, then {}.", cap(told_words(a)), told_words(b)));
    s.them(&name, x, Some(format!("{} first, then {}.", a.name(), b.name().to_lowercase())));
    if c[order[0] as usize] >= DEALBREAKER
        && !dress
        && let Some(x) = say(w, s, 52, &["recruit.want.dealbreaker".into()], &sp, &base(w, t))
    {
        s.them(&name, x, None);
    }
    note(w, t, a as u8, Ev::ToldFirst, None);
    note(w, t, b as u8, Ev::ToldSecond, None);
}

/// Who decides at his home, in their words.
fn decides(w: &mut World, s: &mut Scene, t: &Talk) {
    let p = &w.people[t.i];
    let (code, plain, note_words) = match p.family.decider {
        Decider::Himself => (0u8, "It's my call. My family's behind whatever I pick.", "He decides himself."),
        Decider::Parent => match p.family.structure {
            FamilyStructure::TwoParents => (1, "We'll decide together, as a family.", "His parents decide."),
            FamilyStructure::Foster => (1, "I'll have the last word on this.", "His guardian decides."),
            _ => (1, "I'll have the last word on this.", "His mother decides."),
        },
        Decider::Grandparent => (2, "I'll have the last word on this.", "His grandmother decides."),
        Decider::Handler => (3, "He listens to me on this. I'll have the last word.", "His trainer decides."),
    };
    let me = scene::your_name(w);
    s.you(&me, "Who has the final say in this?");
    let sp = speaker(w, t);
    let x = say(w, s, 53, &["recruit.tell.decides".into()], &sp, &base(w, t)).unwrap_or_else(|| plain.into());
    let name = who(w, t);
    s.them(&name, x, Some(note_words.into()));
    note(w, t, code, Ev::Decides, None);
}

/// Why a transfer left, and where he stood, in his words.
fn why_left(w: &mut World, s: &mut Scene, t: &Talk) {
    let p = &w.people[t.i];
    let why = p.cal.portal.map(|e| e.why).unwrap_or("");
    let (key, g) = match why {
        "coaching change" => (Some("coach"), Some(Group::People)),
        "move up" => (Some("up"), Some(Group::League)),
        "money" => (Some("money"), Some(Group::Money)),
        "graduate transfer" => (Some("grad"), None),
        "playing time" | "spring depth chart" => (Some("snaps"), Some(Group::PlayingTime)),
        "steered" => (Some("choice"), None),
        _ => (None, None),
    };
    let role = match p.last_role {
        Some(PlayingTime::Starter) => Some("starter"),
        Some(PlayingTime::Backup) | Some(PlayingTime::Redshirt) => Some("backup"),
        _ => None,
    };
    let me = scene::your_name(w);
    s.you(&me, "Why did you leave?");
    let sp = speaker(w, t);
    let f = base(w, t);
    let mut said: Vec<String> = Vec::new();
    match key.and_then(|k| say(w, s, 54, &[format!("transfer.why.{k}")], &sp, &f)) {
        Some(x) => said.push(x),
        None => said.push("I needed a new place. That's all there is to it.".into()),
    }
    if let Some(x) = role.and_then(|r| say(w, s, 55, &[format!("transfer.role.{r}")], &sp, &f)) {
        said.push(x);
    }
    s.them("Him", said.join(" "), None);
    if let Some(g) = g {
        note(w, t, g as u8, Ev::BroughtUp, None);
    }
}

/// What his other schools are selling, as he tells it: one that pays more,
/// one closer to home, one that sees him starting, or that your money is
/// the biggest he has seen.
fn others(w: &mut World, s: &mut Scene, t: &Talk) {
    let sheet = claims::recruit(w, t.i, t.j, t.ear);
    let mut key: Option<(&str, Option<usize>)> = None;
    let money = claims::rival_for(w, t.i, t.j, Kind::OfferOthers);
    if let Some((r, _, true)) = money
        && sheet.has(Kind::OfferOthers)
    {
        key = Some(("money", Some(r)));
    }
    if key.is_none()
        && let Some((r, _, true)) = claims::rival_for(w, t.i, t.j, Kind::Drive)
    {
        key = Some(("closer", Some(r)));
    }
    if key.is_none()
        && t.transfer
        && let Some((r, v, true)) = claims::rival_for(w, t.i, t.j, Kind::Role)
        && v >= claims::sheet::PROOF[0] * 0.9
    {
        key = Some(("start", Some(r)));
    }
    if key.is_none() && sheet.get(Kind::OfferOthers).and_then(Point::tier) == Some(Tier::Elite) {
        key = Some(("you_money", None));
    }
    let me = scene::your_name(w);
    s.you(&me, "What are the other schools telling you?");
    let sp = speaker(w, t);
    let mut f = base(w, t);
    if let Some((_, Some(r))) = key {
        f.set("rival_school", w.program_name(r).to_string());
    }
    let x = key
        .and_then(|(k, _)| say(w, s, 56, &["recruit.tell.others".into(), format!("recruit.other.{k}")], &sp, &f))
        .unwrap_or_else(|| "They all say about the same things.".into());
    let name = who(w, t);
    s.them(&name, x, None);
}

/// "How are things at home?": one thing about the family's life that is
/// true, in their words.
fn home(w: &mut World, s: &mut Scene, t: &Talk) {
    let p = &w.people[t.i];
    let age = p.player.age;
    let this_year =
        |phrase: &str| p.bg.story.iter().any(|x| x.starts_with(&format!("At {age},")) && x.contains(phrase));
    let ever = |phrase: &str| p.bg.story.iter().any(|x| x.contains(phrase));
    use crate::world::background::LifeEvent as L;
    let mut facts: Vec<&str> = Vec::new();
    if this_year(L::Death.phrase()) {
        facts.push("death");
    }
    if this_year(L::JobLoss.phrase()) {
        facts.push("job_loss");
    }
    if this_year(L::NewBaby.phrase()) {
        facts.push("new_baby");
    }
    if this_year(L::FamilyMove.phrase()) {
        facts.push("family_move");
    }
    if ever(L::NilChangesFamilyLife.phrase()) {
        facts.push("nil_changed");
    }
    // hidden: the household's money and his grades are gates only.
    let money = p.family_money();
    if money < claims::NEED_MONEY {
        facts.push("money_tight");
    } else if money > 0.7 {
        facts.push("money_fine");
    }
    match p.family.structure {
        FamilyStructure::SingleParent => facts.push("single_parent"),
        FamilyStructure::Grandparents => facts.push("grandparents"),
        FamilyStructure::Foster => facts.push("foster"),
        FamilyStructure::TwoParents => {}
    }
    match p.family.bloodline {
        Bloodline::NflPlayer => facts.push("bloodline_nfl"),
        Bloodline::CollegePlayer => facts.push("bloodline_college"),
        Bloodline::None => {}
    }
    if p.family.older_brother_at.is_some() {
        facts.push(if p.family.follows_brother { "brother_follow" } else { "brother_own" });
    }
    if p.bg.mentor {
        facts.push("mentor");
    }
    if p.bg.gpa < claims::GRADE_LINE {
        facts.push("grades_low");
    }
    if p.family.ambition >= AMBITIOUS {
        facts.push("ambitious");
    }
    let me = scene::your_name(w);
    s.you(&me, "How are things at home?");
    let sp = speaker(w, t);
    let f = base(w, t);
    let n = facts.len().max(1);
    let start = (s.id as usize) % n;
    let mut said = None;
    for k in 0..facts.len() {
        let fact = facts[(start + k) % n];
        if let Some(x) = say(w, s, 57, &[format!("recruit.bg.{fact}")], &sp, &f) {
            said = Some(x);
            break;
        }
    }
    let name = who(w, t);
    s.them(&name, said.unwrap_or_else(|| "We're doing all right, thank you for asking.".into()), None);
}

// ---------------------------------------------------------------------------
// The opening.

/// How warm he is to you: where you stand on his list.
fn warmth(w: &World, t: &Talk) -> &'static str {
    let race = if t.transfer { portal::race(w, t.i) } else { crate::world::front_office::college::race_at(w, t.i) };
    match race.iter().position(|r| r.0 == t.j) {
        Some(0 | 1) => "warm",
        Some(2 | 3) => "polite",
        _ => "guarded",
    }
}

/// What changed since you last spoke, most on his mind first: an event on
/// his sheet he hasn't talked through with you.
fn news<'a>(w: &World, t: &Talk, sheet: &'a Sheet) -> Option<&'a Point> {
    let today = w.today;
    sheet
        .points
        .iter()
        .filter(|p| p.known && p.kind.family() == Family::Event)
        .find(|p| !w.folk.talked.remembers(today, t.id, p))
}

/// Where the two of you stand, when nothing changed: a commitment, the
/// first talk, a long silence, your last season, or picking up again.
fn history(w: &World, t: &Talk, sheet: &Sheet) -> (String, Facts) {
    let p = &w.people[t.i];
    let f = base(w, t);
    if !t.transfer
        && let Some((c, ..)) = p.cal.commit
    {
        return (if c == t.j { "recruit.committed.you" } else { "recruit.committed.elsewhere" }.into(), f);
    }
    let last = w.folk.talked.with(t.id).map(|r| r.on).max();
    let today = w.today;
    match last {
        None if w.folk.pulls.of(t.id).next().is_none() => ("recruit.history.first".into(), f),
        Some(d) if d.days_to(today) > SILENCE_DAYS => ("recruit.history.silence".into(), f),
        _ => {
            let year = |pt: &Point| pt.get("last_year").and_then(|y| y.parse::<i32>().ok());
            if let Some(pt) = sheet.get(Kind::Titles).filter(|pt| year(pt).is_some_and(|y| y >= w.year - 1))
                && let Some(rec) = pt.get("last_record")
            {
                let wins = rec.split('-').next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(0);
                let f = f.clone().with("last_record", rec.to_string());
                if wins >= GOOD_YEAR {
                    return ("recruit.team.good_year".into(), f);
                }
                if wins <= BAD_YEAR {
                    return ("recruit.team.bad_year".into(), f);
                }
            }
            ("recruit.history.again".into(), f)
        }
    }
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let Some(t) = talk(w, s) else { return s.end() };
    if let Some(why) = over(w, &t) {
        s.outcome.push(why.into());
        return s.end();
    }
    let p = &w.people[t.i];
    let kid = super::player_link(w, t.i);
    let room = if !t.transfer {
        match p.family.decider {
            Decider::Himself => format!("{kid} and his family sit across from you. He does the talking."),
            _ => {
                let d = dialogue::decider_words(p);
                format!("{kid} sits beside {}. {d} does the talking.", d.to_lowercase())
            }
        }
    } else if t.at == Where::Campus {
        format!(
            "{kid} sits across your desk at the end of his visit. He has seen the place; now he wants straight answers."
        )
    } else if t.at == Where::Home {
        format!("{kid} sits across from you in his family's living room. He does the talking.")
    } else {
        format!("{kid} picks up. Every staff with a place for him has called.")
    };
    s.narrate(room);
    let sp = speaker(w, &t);
    let name = who(w, &t);
    let f = base(w, &t);
    let warm = warmth(w, &t);
    if let Some(x) = say(w, s, 1, &[format!("recruit.open.{}.{warm}", t.setting())], &sp, &f) {
        s.them(&name, x, None);
    }
    let sheet = claims::recruit(w, t.i, t.j, t.ear);
    let mut opened = false;
    // An opener names its event by the point's own facts: a coach who left
    // with no name on the point would read as you.
    if let Some(ev) = news(w, &t, &sheet).filter(|e| e.kind != Kind::CoachLeft || e.get("coach").is_some()).cloned() {
        let ff = ev.facts(&f);
        if let Some(x) = say(w, s, 2, &[format!("recruit.open.{}", ev.kind.key())], &sp, &ff) {
            s.them(&name, x, None);
            opened = true;
        }
        let today = w.today;
        w.folk.talked.note(today, t.id, &ev, false, "open");
    }
    if !opened {
        let (key, ff) = history(w, &t, &sheet);
        if let Some(x) = say(w, s, 3, &[key], &sp, &ff) {
            s.them(&name, x, None);
        }
    }
    s.f[0] = 0.0;
    next_beat(w, s, &t, St::default());
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let Some(t) = talk(w, s) else { return s.end() };
    if let Some(why) = over(w, &t) {
        s.outcome.push(why.into());
        return s.end();
    }
    let mut st = St::of(s);
    match code {
        LOOKING => {
            looking(w, s, &t);
            st.free |= 1;
        }
        DECIDES => {
            if t.transfer {
                why_left(w, s, &t);
            } else {
                decides(w, s, &t);
            }
            st.free |= 2;
        }
        OTHERS => {
            others(w, s, &t);
            st.free |= 4;
        }
        HOME => {
            home(w, s, &t);
            st.free |= 8;
        }
        ELSE => st.nav = NAV_MENU,
        BACK => st.nav = if st.ask == 0 { NAV_MENU } else { NAV_ASKED },
        PROMISE_MENU => st.nav = NAV_PROMISE,
        c if (GROUP..GROUP + 7).contains(&c) => st.nav = (c - GROUP) as u8 + 1,
        PROMISE_START | PROMISE_ROLE => return promise(w, s, &t, st, (code - PROMISE_START) as u8, picked),
        LEAVE => {
            let me = scene::your_name(w);
            s.you(&me, picked.label.trim_matches('"'));
            if st.used > 0 || st.stage > 0 {
                return close(w, s, &t, &st);
            }
            s.outcome.push("You listened and left it there.".into());
            return finish(w, s, &t, 0.0, 0);
        }
        c if (POINT..STRAIGHT).contains(&c) => {
            return match kind_at(c - POINT) {
                Some(k) => make_point(w, s, &t, st, k, picked),
                None => next_beat(w, s, &t, st),
            };
        }
        c if (STRAIGHT..GROUP).contains(&c) => {
            return match kind_at(c - STRAIGHT) {
                Some(k) => be_straight(w, s, &t, st, k, picked),
                None => next_beat(w, s, &t, st),
            };
        }
        c if (STRAIGHT_GROUP..STRAIGHT_GROUP + 7).contains(&c) => {
            return straight_on(w, s, &t, st, Group::ALL[(c - STRAIGHT_GROUP) as usize], None, picked);
        }
        _ => {}
    }
    if st.nav == NAV_ASKED && st.ask == 0 {
        st.nav = NAV_MENU;
    }
    let sheet = claims::recruit(w, t.i, t.j, t.ear);
    picks(w, s, &t, &st, &sheet);
    st.put(s);
}

/// Your promise of a role to a transfer (as built: kept on his file,
/// checked after he arrives); then the talk closes.
fn promise(w: &mut World, s: &mut Scene, t: &Talk, st: St, kind: u8, picked: &Pick) {
    let me = scene::your_name(w);
    s.you(&me, picked.label.trim_matches('"'));
    let Some(read) = portal::promise(w, t.id, kind) else {
        s.outcome.push("Your staff has no place for him to promise: the roster is full, or he can't come.".into());
        return close(w, s, t, &st);
    };
    let key = match read {
        (_, _, PlayingTime::Starter) if kind == 0 => "knows",
        (_, b, _) if b >= 0.6 => "believes",
        _ => "doubts",
    };
    let sp = speaker(w, t);
    let f = base(w, t);
    if let Some(x) = say(
        w,
        s,
        60,
        &[format!("recruit.promise.{}.{key}", ["start", "role"][kind as usize % 2]), format!("portal.promise.{key}")],
        &sp,
        &f,
    ) {
        s.them("Him", x, None);
    }
    s.outcome.push(
        match key {
            "knows" => "He knows he'd start anyway: the promise adds little, and it binds you.",
            "believes" => "He believes you. It's the promise he wanted to hear.",
            _ => "He's heard promises before. He isn't sure he believes yours.",
        }
        .to_string(),
    );
    s.outcome.push(format!("You promised {}.", if kind == 0 { "him a starting job" } else { "him a real role" }));
    close(w, s, t, &st);
}

/// Your staff takes the talk: the position coach for his spot makes the
/// case in one step, as every AI staff does, and brings back what he asked
/// and how he took it, as far as the coach's own read catches. A talk you
/// started and left stands as you left it.
pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    let Some(t) = talk(w, s) else { return };
    let st = St::of(s);
    if over(w, &t).is_some() {
        s.handed = Some("nobody".into());
        return;
    }
    if st.used > 0 || st.stage > 0 {
        s.handed = Some("nobody".into());
        s.outcome.push("Your case stands. Nobody made more of it for you.".into());
        return finish(w, s, &t, s.f[0], st.made);
    }
    let c = dialogue::caller(w, t.i, t.j);
    let back = backing(w, &t);
    let one = case::one_step(w, t.i, t.j, c, t.ear, back, t.limit() as usize, t.scale(), s.id as u64 ^ 0x7070);
    let delta = match case_for(w, &t) {
        Some(x) => {
            let before = x.total();
            x.merge(&one);
            x.total() - before
        }
        None => 0.0,
    };
    // What he asked the coach, and how he took each point, as far as the
    // coach's read catches.
    if let Some(g) = asks(w, &t, 0, 0, s.id as u64 ^ 0x7071) {
        note(w, &t, g as u8, Ev::BroughtUp, c);
    }
    let cares = care(w, &t);
    let covered = case::covered(&one);
    for &g in &covered {
        let k = g as usize;
        let tier = Tier::of(back[k] as f32);
        let real = truth(cares[k], one.groups[k], tier);
        let says = shown(w, &t, g, real);
        note(w, &t, g as u8, words_ev(says), c);
        let parted = real != says && !(real == Took::Flat && says == Took::Polite);
        if parted && catches(w, c, (t.id as u64) << 8 ^ 0x57A ^ k as u64) {
            note(w, &t, g as u8, if real == Took::Lit { Ev::CaughtLit } else { Ev::CaughtFlat }, c);
        }
    }
    s.handed = Some(c.map_or("your staff".to_string(), |c| w.coaches[c].name.clone()));
    let said: Vec<String> = covered.iter().map(|g| g.name().to_lowercase()).collect();
    s.outcome.push(if said.is_empty() {
        "He listened more than he sold.".to_string()
    } else {
        format!("He made the case for {}.", claims::words::list(&said))
    });
    let made = covered.iter().fold(0u8, |m, &g| m | bit(g));
    finish(w, s, &t, delta, made);
}
