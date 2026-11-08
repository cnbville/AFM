//! The film room and being there (update 3, lane 9; scouting update
//! SK32-SK34, SK36-SK39, SK43-SK63).
//!
//! All film is cut-ups (SK45): a player's tape is played on the match
//! engine from his real team against his real opponents (`tape`), and a
//! session watches one cut-up snap by snap (`session`). What your eye
//! catches is kept per attribute (`Catch`) and moves your own read
//! (`sharpen`); the session becomes your report in his file. Film time is
//! one weekly ledger on the real calendar, set by how hard you work
//! (`time`); a kid nobody recruits can pop off another kid's tape
//! (`finds`); your staff send you a queue of cut-ups each week (`queue`);
//! and the days you are there in person are scenes (`there`).
//!
//! Every number no source gives is Claude's, proposed, not confirmed.

pub mod finds;
pub mod opp;
pub mod queue;
pub mod session;
pub mod tape;
pub mod there;
pub mod time;
pub mod words;

use crate::attributes::{ATTRIBUTE_COUNT, Attr};
use crate::calendar::{Date, Weekday};
use crate::positions::{Position, weight};
use crate::world::World;

/// What a session watches: a player, or a team (your next opponent).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Target {
    Player(u32),
    Team(u16),
}

/// One thing your eye (or your analyst beside you) caught on a snap: the
/// attribute, what it looked like and how sure (rating points).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Catch {
    pub id: u32,
    pub attr: u8,
    pub v: f32,
    pub sd: f32,
    pub date: Date,
    /// 0 your eye, 1 your analyst's point.
    pub by: u8,
    /// Which snap it was read on (0 for a look in person): the same snap
    /// watched again adds nothing new.
    #[serde(default)]
    pub key: u64,
    /// How big your eye's lasting misjudgment of this man's trait is (an
    /// sd in rating points, minor bug 320): the same misjudgment is in
    /// every catch your eye makes of it (`eye_lean`), so more tape can't
    /// take it out. Analysts' points and older catches carry 0.
    #[serde(default)]
    pub lean: f32,
    /// The game (or the day in person) it was read on (0 for none): what
    /// one game shows of a man is that game (minor bug 338), so catches
    /// of one game count as one look of it, not as many.
    #[serde(default)]
    pub game: u64,
    /// How far that game's look at the trait sits from him, its sd in
    /// rating points (`GAME_SD`); the same in every catch of the game.
    /// Older catches, folded ones and analysts' points carry 0.
    #[serde(default)]
    pub form: f32,
}

/// How far one game's look at a man's trait sits from him, as an sd in
/// rating points for an ordinary eye, times `file::sharp(eye)` (minor bug
/// 338, proposed, not confirmed). He has better and worse games, and an
/// opponent and a game plan show some of him and hide some, so a scout
/// watches three to five games before he trusts a read: one game's 200
/// catches can't read him much better than one game can. With your eye's
/// lasting misjudgment (`lean_sd`), an elite eye reads a well-shown trait
/// within about 2 after a game, 1.2 after four and 1 after a season; an
/// ordinary eye within about 4.9, 4.2 and 4, its blind spots for good.
pub const GAME_SD: f64 = 3.0;

/// One game's (or one day's) key for its catches of a man: `Catch::game`.
pub(crate) fn game_key(label: &str, season: i32) -> u64 {
    let h = label.bytes().fold(0x84222325_cbf29ce4u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    (h ^ (season as u64) << 40 ^ 0x6A3E) | 1
}

/// Your eye's lasting misjudgment of a trait for an ordinary eye, in rating
/// points, before the trait's own weight (minor bug 320; sized by minor bug
/// 352, Cain 2026-11-08 "Bigger gap": a 10 eye stays near ±4 after a season
/// of his tape, a 20 near ±1). It holds however much tape the eye watches:
/// without it any eye read anyone exactly after a few hours. An analyst's
/// numbers carry none (SK57: tape with a top analyst beside you can take
/// your read all the way).
pub const EYE_LEAN: f64 = 3.5;

/// Your eye's lasting misjudgment of a trait, its sd in rating points:
/// `EYE_LEAN` × (0.85 + 0.6 × how hard film makes the trait to see,
/// `words::subtle`) × sharp(eye)³, so it falls steeply as the eye rises: a
/// 20 eye 0.73 to 0.96, a 15 eye 1.7 to 2.3, a 10 eye 3.4 to 4.4, a 5 eye
/// 5.9 to 7.7 (results traits such as arm strength and speed to technique
/// and mental ones such as play recognition and footwork). Proposed, not
/// confirmed: the cube and the trait weights.
pub fn lean_sd(eye: f64, a: Attr) -> f64 {
    EYE_LEAN * (0.85 + 0.6 * words::subtle(a)) * super::file::sharp(eye).max(0.0).powi(3)
}

/// Your eye's lasting misjudgment of one man's trait, in its sd (a
/// standard normal keyed by you, him and the trait): the same on tape and
/// in person, whatever day you look.
pub fn eye_lean(w: &World, id: u32, a: Attr) -> f64 {
    let you = super::readers::you(w).and_then(|c| w.coaches.get(c)).map_or(0, |c| c.id as u64);
    crate::world::season::key_normal(0xE1E_1EA4 ^ you << 24 ^ (a.index() as u64) << 52, id as u64)
}

/// The snaps of his you have watched, kept for the next watch (minor bug
/// 319): about a season of film on 800 players.
pub const WATCHED_KEPT: usize = 60_000;

/// Catches kept one by one; past this the oldest half are folded (minor
/// bug 321: past 6,000 the oldest were dropped, and the reads they made
/// went with them).
pub const CATCHES_KEPT: usize = 40_000;

/// Folds the oldest catches when there are too many, so no read is lost.
/// First each game's catches of a man's trait by one eye become one catch
/// of that game: its centre and the catches' own sd, the game's key and
/// its own look (`form`) kept, so it reads exactly as they did and a later
/// catch of the same game joins it as the same look. Only if that leaves
/// more than three quarters of `CATCHES_KEPT`: his looks over a year old,
/// then all the folded half, become one catch a man, trait and eye each
/// (looks by inverse variance, the day weighted the same way), aged from
/// then on as one look.
pub(crate) fn compact(all: &mut Vec<Catch>, today: Date) {
    if all.len() <= CATCHES_KEPT {
        return;
    }
    let cut = all.len() / 2;
    let mut old: Vec<Catch> = all.drain(..cut).collect();
    old.sort_by_key(|c| (c.id, c.attr, c.by, c.game));
    let mut kept: Vec<Catch> = Vec::with_capacity(old.len() / 4);
    for g in old.chunk_by(|a, b| a.game != 0 && (a.id, a.attr, a.by, a.game) == (b.id, b.attr, b.by, b.game)) {
        kept.push(if g.len() == 1 { g[0] } else { one_game(g) });
    }
    for older in [Some(365), None] {
        if kept.len() + all.len() <= CATCHES_KEPT * 3 / 4 {
            break;
        }
        let mut out = Vec::with_capacity(kept.len());
        for man in kept.chunk_by(|a, b| (a.id, a.attr, a.by) == (b.id, b.attr, b.by)) {
            let (fold, keep): (Vec<Catch>, Vec<Catch>) =
                man.iter().partition(|c| older.is_none_or(|days| today.0 - c.date.0 > days));
            if fold.len() > 1 {
                out.push(one_man(&fold));
            } else {
                out.extend(fold);
            }
            out.extend(keep);
        }
        kept = out;
    }
    kept.append(all);
    *all = kept;
}

/// One game's catches of a man's trait by one eye as one catch of that
/// game (`compact`): what they read together, the game's look kept.
fn one_game(g: &[Catch]) -> Catch {
    let (mut s, mut wt, mut d, mut lean, mut form) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f32);
    for c in g {
        let x = (c.sd as f64).max(0.05);
        let k = 1.0 / (x * x);
        s += k * c.v as f64;
        wt += k;
        d += k * c.date.0 as f64;
        lean += k * c.lean as f64;
        form = form.max(c.form);
    }
    Catch {
        v: (s / wt) as f32,
        sd: (1.0 / wt).sqrt() as f32,
        date: Date((d / wt).round() as i32),
        key: 0,
        lean: (lean / wt) as f32,
        form,
        ..g[0]
    }
}

/// A man's trait by one eye as one catch (`compact`): each game as one
/// look of it, then the looks by inverse variance, the day and lean the
/// same way. It carries no snap and no game: its sd holds the games' own.
fn one_man(man: &[Catch]) -> Catch {
    let (mut s, mut wt, mut d, mut lean) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for g in grouped(man) {
        s += g.mean / g.var;
        wt += 1.0 / g.var;
        d += g.day / g.var;
        lean += g.lean / g.var;
    }
    Catch {
        v: (s / wt) as f32,
        sd: (1.0 / wt).sqrt() as f32,
        date: Date((d / wt).round() as i32),
        key: 0,
        lean: (lean / wt) as f32,
        game: 0,
        form: 0.0,
        ..man[0]
    }
}

/// One game's catches of a trait as one look: its centre, its variance
/// (the catches' own, plus the game's, `Catch::form`), its weighted day
/// and lean. Catches without a game are each their own look.
struct GameLook {
    mean: f64,
    var: f64,
    day: f64,
    lean: f64,
}

/// The looks in catches of one man's trait by one eye, sorted by game
/// (as they were read; `catches_with` widens each look by its age).
fn grouped(cs: &[Catch]) -> Vec<GameLook> {
    looks_of(cs, None)
}

/// The same, each look widened by how far he may have moved since it
/// (`DRIFT_A_YEAR`), once a look: a game's catches are of one day.
fn looks_of<C: std::borrow::Borrow<Catch>>(cs: &[C], today: Option<Date>) -> Vec<GameLook> {
    let mut out = Vec::new();
    for g in cs.chunk_by(|a, b| {
        let (a, b) = (a.borrow(), b.borrow());
        a.game != 0 && a.game == b.game
    }) {
        let (mut s, mut wt, mut d, mut lean, mut form) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for c in g {
            let c = c.borrow();
            let x = (c.sd as f64).max(0.05);
            let k = 1.0 / (x * x);
            s += k * c.v as f64;
            wt += k;
            d += k * c.date.0 as f64;
            lean += k * c.lean as f64;
            form = form.max(c.form as f64);
        }
        let day = d / wt;
        let drift = today.map_or(0.0, |t| DRIFT_A_YEAR * (t.0 as f64 - day).max(0.0) / 365.0);
        out.push(GameLook { mean: s / wt, var: 1.0 / wt + form * form + drift * drift, day, lean: lean / wt });
    }
    out
}

/// Same snap, same man, same thing, same eyes.
fn same(a: &Catch, b: &Catch) -> bool {
    a.key != 0 && a.key == b.key && a.id == b.id && a.attr == b.attr && a.by == b.by
}

/// Adds a session's catches: a snap seen before keeps the tighter read of
/// it (a better eye sees more in the same snap), never both.
pub(crate) fn add_catches(all: &mut Vec<Catch>, new: Vec<Catch>) {
    for c in new {
        match all.iter().position(|o| same(o, &c)) {
            Some(k) if c.sd < all[k].sd => all[k] = c,
            Some(_) => {}
            None => all.push(c),
        }
    }
}

/// A tag carried forward (SK44): "ask him about this" to the interview
/// room, "show the staff" to the board meetings.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tag {
    pub id: u32,
    pub date: Date,
    /// 2 ask him, 3 show the staff (`session::TAGS`).
    pub kind: u8,
    pub text: String,
    #[serde(default)]
    pub used: bool,
}

/// Everything the film room keeps (on `Scouting::film`; every field fills
/// in empty on an older file).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FilmRoom {
    /// The session in progress.
    #[serde(default)]
    pub open: Option<session::Session>,
    #[serde(default)]
    pub reports: Vec<session::Report>,
    #[serde(default)]
    pub catches: Vec<Catch>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    /// Your staff's queue (SK55).
    #[serde(default)]
    pub queue: Vec<queue::Queued>,
    /// Film hours today (the day, hours): your eye tires (SK59).
    #[serde(default)]
    pub today: Option<(Date, f32)>,
    /// How hard you work this week (SK61).
    #[serde(default)]
    pub work: time::Work,
    /// The weeks you worked: Monday, level, film hours, the budget.
    #[serde(default)]
    pub weeks: Vec<time::Week>,
    /// Wear from long weeks (0 fresh, 1 worn out), and the rest you were
    /// sent home for.
    #[serde(default)]
    pub wear: f32,
    #[serde(default)]
    pub rest_until: Option<Date>,
    /// Film hours by position (SK63), by `Position::index`.
    #[serde(default)]
    pub pos_hours: Vec<f32>,
    /// Film hours by year (your coach profile, SK63).
    #[serde(default)]
    pub years: Vec<(i32, f32)>,
    /// Kids who popped off another kid's tape (SK34).
    #[serde(default)]
    pub finds: Vec<finds::Find>,
    /// Your work ethic (SK62), 1-20, once set or moved; None reads 12.
    #[serde(default)]
    pub ethic: Option<f32>,
    /// The days you were there in person (SK32-SK39).
    #[serde(default)]
    pub there: there::There,
    /// When the word "first one in the building" last got around.
    #[serde(default)]
    pub word: Option<Date>,
    /// The snaps you watched: the player and the snap (minor bug 319).
    #[serde(default)]
    pub watched: Vec<(u32, u64)>,
    /// Bumped on every change that moves a read (the reads cache keys on it).
    #[serde(default)]
    pub version: u32,
    /// The hardest work level set this week, from its Monday (SK61).
    #[serde(default)]
    pub week_work: Option<(Date, time::Work)>,
    /// The year you last set your work ethic (SK62): once a year.
    #[serde(default)]
    pub ethic_year: Option<i32>,
}

/// How far a catch drifts from him as he changes: rating points a year
/// (a young player grows, an old one fades).
const DRIFT_A_YEAR: f64 = 3.0;

/// Your film catches of a player, attribute by attribute: each one
/// widened by its age, combined by inverse variance, your eye's catches
/// as one read that keeps its lasting misjudgment (`Catch::lean`, minor
/// bug 320) and your analyst's as another. Exact (sd 0, his real number
/// today) once it is tighter than half a point on fresh tape: "100%" for
/// what the tape shows (SK57).
pub fn catches_of(w: &World, id: u32) -> Vec<Option<(f64, f64)>> {
    catches_with(w, id, &[])
}

/// The same with catches not yet saved (a session in progress).
pub fn catches_with(w: &World, id: u32, extra: &[Catch]) -> Vec<Option<(f64, f64)>> {
    split_with(w, id, extra).into_iter().map(|s| s.and_then(|s| s.read())).collect()
}

/// One trait's film read in its parts (minor bug 352): your eye's centre,
/// the part of its variance more tape shrinks and its lasting lean (an sd,
/// `Catch::lean`); your analyst's centre and variance; his real number
/// once the read is exact.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Split {
    eye: Option<(f64, f64, f64)>,
    analyst: Option<(f64, f64)>,
    exact: Option<f64>,
}

impl Split {
    /// The read on its own: the two parts by inverse variance, your eye's
    /// with its lean kept in.
    fn read(&self) -> Option<(f64, f64)> {
        if let Some(t) = self.exact {
            return Some((t, 0.0));
        }
        let (mut s, mut wt) = (0.0, 0.0);
        for (c, v) in self.eye.map(|(c, v, l)| (c, v + l * l)).into_iter().chain(self.analyst) {
            s += c / v;
            wt += 1.0 / v;
        }
        (wt > 0.0).then(|| (s / wt, (1.0 / wt).sqrt()))
    }
}

/// Each trait's film read in its parts (`Split`).
pub(crate) fn split_with(w: &World, id: u32, extra: &[Catch]) -> Vec<Option<Split>> {
    let today = w.today;
    let stored = &w.scouting.film.catches;
    let fresh = extra.iter().filter(|c| !stored.iter().any(|o| same(o, c)));
    // His catches by trait, eye (0 yours, 1 the analyst's) and game: each
    // game's catches are one look of it (minor bug 338).
    let mut his: Vec<&Catch> =
        stored.iter().chain(fresh).filter(|c| c.id == id && (c.attr as usize) < ATTRIBUTE_COUNT).collect();
    his.sort_by_key(|c| (c.attr, c.by.min(1), c.game));
    let i = super::place(w, id);
    let mut out = vec![None; ATTRIBUTE_COUNT];
    for cs in his.chunk_by(|a, b| a.attr == b.attr) {
        let k = cs[0].attr as usize;
        let fresh = cs.iter().map(|c| c.date.days_to(today)).min().unwrap_or(i32::MAX);
        // Each eye's read: its games' centre, its variance and its lasting
        // lean.
        let mut sp = Split::default();
        for eye in cs.chunk_by(|a, b| a.by.min(1) == b.by.min(1)) {
            let (mut es, mut ew, mut el) = (0.0, 0.0, 0.0);
            // Each look widened by its age: he changes.
            for g in looks_of(eye, Some(today)) {
                es += g.mean / g.var;
                ew += 1.0 / g.var;
                el += g.lean / g.var;
            }
            if ew <= 0.0 {
                continue;
            }
            let lean = el / ew;
            if eye[0].by == 0 {
                sp.eye = Some((es / ew, 1.0 / ew, lean));
            } else {
                sp.analyst = Some((es / ew, 1.0 / ew + lean * lean));
            }
        }
        let Some((_, sd)) = sp.read() else { continue };
        if sd <= 0.5
            && fresh <= 30
            && let Some(i) = i
        {
            sp.exact = Some(w.people[i].player.attributes.get(Attr::ALL[k]));
        }
        out[k] = Some(sp);
    }
    out
}

/// Your film catches folded into your own read, with none of your looks
/// on tape: see `sharpen_with`.
pub fn sharpen(w: &World, id: u32, pos: Position, now: &mut (f64, f64), attrs: &mut Vec<Option<(f64, f64)>>) {
    sharpen_with(w, id, pos, now, attrs, &[], &[], &[]);
}

/// Your eye's catches folded into your own read (SK43), trait by trait,
/// and the overall narrowed by how much of his position's weight they
/// pinned down. Your eye on his tape is one read (minor bug 352): what
/// your film looks of him read of the trait (`tape`: the film room's, the
/// hunt's games, old film) and your eye's catches share your eye's lasting
/// misjudgment of him, so more tape narrows toward it and never past it.
/// Before, the two joined as if apart and averaged it away, so a weak eye
/// read nearly as close as a strong one. What your other looks read
/// (`others`: practices, games in person, camps, visits) and your
/// analyst's points join it apart, so more of anything never widens a
/// read. An empty `tape`: none of your looks is on tape, and `attrs` is
/// what your other looks read.
#[allow(clippy::too_many_arguments)]
pub fn sharpen_with(
    w: &World,
    id: u32,
    pos: Position,
    now: &mut (f64, f64),
    attrs: &mut Vec<Option<(f64, f64)>>,
    tape: &[Option<(f64, f64)>],
    others: &[Option<(f64, f64)>],
    extra: &[Catch],
) {
    let film = split_with(w, id, extra);
    if film.iter().all(Option::is_none) && tape.iter().all(Option::is_none) {
        return;
    }
    let e = session::eye(w, pos);
    attrs.resize(ATTRIBUTE_COUNT, None);
    let (mut shift, mut cov, mut total) = (0.0, 0.0, 0.0);
    for a in Attr::ALL {
        let wt = weight(pos, a);
        total += wt;
        let k = a.index();
        let sp = film[k];
        let t = tape.get(k).copied().flatten();
        if sp.is_none() && t.is_none() {
            continue;
        }
        let before = attrs[k];
        let after = if let Some(x) = sp.and_then(|s| s.exact) {
            (x, 0.0)
        } else {
            let z = eye_lean(w, id, a);
            let lt = lean_sd(e, a);
            // Your eye on his tape: the file's film read moved by your
            // eye's misjudgment, joined with your catches that carry it.
            let eye = match (t, sp.and_then(|s| s.eye)) {
                (Some((ct, st)), Some((ce, ve, le))) => {
                    let (x, y) = (1.0 / (st * st).max(0.01), 1.0 / ve.max(1e-6));
                    let lean = (x * lt + y * le) / (x + y);
                    Some((((ct + lt * z) * x + ce * y) / (x + y), lean * lean + 1.0 / (x + y)))
                }
                (Some((ct, st)), None) => Some((ct + lt * z, st * st + lt * lt)),
                (None, Some((ce, ve, le))) => Some((ce, ve + le * le)),
                (None, None) => None,
            };
            let other = if tape.is_empty() { before } else { others.get(k).copied().flatten() };
            let (mut s, mut wsum) = (0.0, 0.0);
            for (c, v) in
                eye.into_iter().chain(other.map(|(c, sd)| (c, (sd * sd).max(0.01)))).chain(sp.and_then(|s| s.analyst))
            {
                s += c / v;
                wsum += 1.0 / v;
            }
            if wsum <= 0.0 {
                continue;
            }
            ((s / wsum).clamp(1.0, 99.0), (1.0 / wsum).sqrt())
        };
        if let Some(b) = before {
            shift += wt * (after.0 - b.0);
            cov += wt * (1.0 - (after.1 * after.1) / (b.1 * b.1).max(0.01)).clamp(0.0, 1.0);
        } else {
            // Read for the first time: how much of a wide guess (9 points)
            // the tape replaced.
            cov += wt * (1.0 - (after.1 / 9.0).powi(2)).clamp(0.0, 1.0);
        }
        attrs[k] = Some(after);
    }
    if total > 0.0 {
        now.0 = (now.0 + shift / total).clamp(1.0, 99.0);
        now.1 = (now.1 * (1.0 - cov / total).max(0.0).sqrt()).max(if cov >= total * 0.999 { 0.0 } else { 0.5 });
    }
}

/// The film room's day: the week turns over on Monday (your hours logged,
/// wear, your ethic, the staff mirroring you, the word around the
/// building, a new queue), the look-backs settle, and the days you are
/// there come round.
pub(crate) fn day(w: &mut World, d: Date) {
    if d.weekday() == Weekday::Mon {
        time::week(w, d);
        queue::week(w, d);
    }
    // A visit ends with the day.
    if w.scouting.film.there.visit.as_ref().is_some_and(|v| v.date < d) {
        let _ = there::visit_close(w);
    }
    settle(w, d);
}

/// The look-back (SK58): three years on, each verdict you gave on a
/// player's question is checked against what he is now.
fn settle(w: &mut World, d: Date) {
    let due: Vec<usize> = (0..w.scouting.film.reports.len())
        .filter(|&k| {
            let r = &w.scouting.film.reports[k];
            r.settled.is_none() && matches!(r.verdict, Some(0 | 1)) && r.date.days_to(d) >= 1095
        })
        .collect();
    for k in due {
        let r = &w.scouting.film.reports[k];
        let (Target::Player(id), Some(q)) = (r.target, r.ask.clone()) else { continue };
        let Some(i) = super::place(w, id) else { continue };
        let p = &w.people[i];
        let m: f64 = q.attrs.iter().map(|&a| p.player.attributes.get(Attr::ALL[a as usize])).sum::<f64>()
            / q.attrs.len().max(1) as f64;
        let yes = m >= q.bar as f64;
        let right = (r.verdict == Some(0)) == yes;
        w.scouting.film.reports[k].settled = Some(right);
    }
}

/// Your tags waiting for a room: "ask him about this" for the interview
/// room, "show the staff" for the board meeting.
pub fn tags_for(w: &World, id: u32, kind: u8) -> Vec<&Tag> {
    w.scouting.film.tags.iter().filter(|t| t.id == id && t.kind == kind && !t.used).collect()
}

/// A tag used in its room.
pub fn use_tags(w: &mut World, id: u32, kind: u8) {
    for t in w.scouting.film.tags.iter_mut().filter(|t| t.id == id && t.kind == kind) {
        t.used = true;
    }
}
