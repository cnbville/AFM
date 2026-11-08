//! Your own read beside your staff's (scouting update SK23, SK24, SK27):
//! every player has two reads, your staff's from all of its looks and
//! yours from your own looks only.
//!
//! Until you have seen him yourself, your read is a first impression from
//! what reaches your desk (your staff's word on him, his stars, a clip):
//! wrong in its centre by a lasting misjudgment of yours, not just wide.
//! Each look of your own narrows it and pulls it toward what you saw, by
//! your evaluation eye: a practice or a game in person most, film next, a
//! camp for what it measures, a visit only the talk attributes (the base
//! misreads of `file::sees`). Your own players are in your practices every
//! week (`readers::own_roster`), so your read of them is your own.

use super::Kind;
use super::file::{self, FileRead, Link};
use super::readers::{self, Staff};
use crate::rng::clamp;
use crate::world::World;
use crate::world::season::key_normal;

/// A first impression's misjudgment for an average eye, rating points of
/// overall (proposed, not confirmed: sd 3.5, so up to about 7 points off).
pub const IMPRESSION_SD: f64 = 3.5;

/// Your tape's own draws in your file (minor bug 352): read apart from
/// your other looks, it errs on its own.
const TAPE_KEY: u64 = 0x7A9E_5EED_0000;

/// Your own read of a player.
#[derive(Clone, Debug)]
pub struct OwnRead {
    /// His overall now: centre and sd.
    pub now: (f64, f64),
    /// Each attribute your own looks read: centre and sd.
    pub attrs: Vec<Option<(f64, f64)>>,
    /// Your own looks written up (0: a first impression only).
    pub looks: usize,
}

impl OwnRead {
    /// The overall as a range on 1-99, the way the file shows one.
    pub fn range(&self) -> (u8, u8) {
        range(self.now)
    }
}

/// A (centre, sd) as a 1-99 range, the file's way (`AttrRead::range`).
pub fn range(r: (f64, f64)) -> (u8, u8) {
    // Exact (lane 9, SK57): tape with a top analyst can pin him down.
    if r.1 < 0.5 {
        let v = r.0.round().clamp(1.0, 99.0) as u8;
        return (v, v);
    }
    let half = r.1.max(1.0);
    let lo = (r.0 - half).floor().clamp(1.0, 99.0) as u8;
    let hi = (r.0 + half).ceil().clamp(1.0, 99.0) as u8;
    (lo, hi.max(lo))
}

/// Your evaluation eye (1-20), or None without a career.
pub fn your_eye(w: &World) -> Option<f64> {
    let c = readers::you(w)?;
    w.coaches.get(c).map(|c| c.evaluation())
}

/// Your first impression of a player from your staff's read of him: their
/// centre, moved by your own lasting misjudgment (keyed by you and him, so
/// it holds), and wider than theirs.
pub fn impression(w: &World, id: u32, staff: &FileRead) -> Option<(f64, f64)> {
    let you = readers::you(w)?;
    let eye = your_eye(w)?;
    let miss = IMPRESSION_SD * file::sharp(eye);
    let lean = key_normal(0x1F57_1A55 ^ (w.coaches[you].id as u64) << 24, id as u64);
    let centre = clamp(staff.now.0 + miss * lean, 1.0, 99.0);
    Some((centre, (staff.now.1 * staff.now.1 + miss * miss).sqrt().max(3.0)))
}

/// Your own read: your looks only, read as your file reads them, combined
/// with your first impression where your staff has one. None while
/// nothing of him has reached you.
pub fn own_read(w: &World, staff: Staff, id: u32, staff_read: Option<&FileRead>) -> Option<OwnRead> {
    own_read_with(w, staff, id, staff_read, &[])
}

/// Your own read with film catches not yet saved (a session in progress,
/// so the film room shows your read narrowing as you watch).
pub fn own_read_with(
    w: &World,
    staff: Staff,
    id: u32,
    staff_read: Option<&FileRead>,
    extra: &[super::film::Catch],
) -> Option<OwnRead> {
    let i = super::place(w, id)?;
    let you = readers::you(w)?;
    let (rds, looks, codes) = readers::evidence(w, staff, id);
    // Your looks: every reader that is you, read as one man's file (minor
    // bug 352). An NFL head coach is also the head coach in his club's
    // file, which holds looks of his own (the board's spring film study)
    // and, for the board, a copy of each watch, trip and asked workout of
    // his the same day: one act counts once. A coordinator's code is yours
    // only while you hold the job (a vacant one's notes read as the head
    // coach's for the board).
    use crate::world::pro::scouts::{DC, HC, OC};
    let job = |code: u32| {
        let tm = match staff {
            Staff::Club(t) => w.nfl.get(t),
            _ => None,
        };
        code == readers::COACH + you as u32
            || tm.is_some_and(|tm| match u8::try_from(code) {
                Ok(HC) => tm.head_coach == Some(you),
                Ok(OC) => tm.oc == Some(you),
                Ok(DC) => tm.dc == Some(you),
                _ => false,
            })
    };
    let mine: Vec<usize> = (0..rds.len()).filter(|&r| rds[r].link == Link::You && job(codes[r])).collect();
    let me = mine.iter().copied().find(|&r| codes[r] == readers::COACH + you as u32).or(mine.first().copied());
    let seat = w.scouting.dept.looks.get(&id).map_or(&[][..], Vec::as_slice);
    let club = match staff {
        Staff::Club(t) => w.clubs.get(t).and_then(|c| c.files.get(&id)).map_or(&[][..], Vec::as_slice),
        _ => &[][..],
    };
    // The club's copies: under the head coach's code, one for each look of
    // yours that day it copies (each new look files its own,
    // `dept::club_copy`, so the board's own looks that day count: its film
    // study, a specialist's pro day). In a save from before this, a watch
    // from the stands filed a film copy on the day you closed it: its
    // report is the look it copies.
    let stands = |d: crate::calendar::Date| {
        use super::film::{Target, session::Mode};
        let fr = &w.scouting.film;
        fr.reports.iter().filter(|r| r.mode == Mode::Live && r.target == Target::Player(id) && r.date == d).count()
    };
    let mut left: Vec<(Kind, crate::calendar::Date, usize)> = Vec::new();
    let mut copy = |l: &file::Evidence| {
        let code = codes[l.reader];
        if code != HC as u32
            || !club.iter().any(|c| c.who == HC && Kind::from_nfl(c.kind) == l.kind && c.date == l.date)
            || seat.iter().any(|s| s.who == code && s.kind == l.kind && s.date == l.date)
        {
            return false;
        }
        let k = match left.iter().position(|x| x.0 == l.kind && x.1 == l.date) {
            Some(k) => k,
            None => {
                let acts = seat.iter().filter(|s| s.date == l.date && job(s.who) && copies(l.kind, s.kind)).count()
                    + if l.kind == Kind::Film { stands(l.date) } else { 0 };
                left.push((l.kind, l.date, acts));
                left.len() - 1
            }
        };
        let is_copy = left[k].2 > 0;
        left[k].2 = left[k].2.saturating_sub(1);
        is_copy
    };
    let own_looks: Vec<file::Evidence> =
        looks.iter().filter(|l| mine.contains(&l.reader) && !copy(l)).copied().collect();
    // Your film of him (Cain 2026-11-04: six hours of tape and no read):
    // each cut-up you watched is a look of its own, so more tape keeps
    // narrowing what film shows, and a cut-up still open counts from its
    // first snap, before anything is in his file.
    let (cut_ups, last) = cut_ups(w, id);
    let group = crate::world::coach::Group::of(w.people[i].player.position) as usize;
    let rd = match me {
        Some(r) => Some(rds[r].clone()),
        None if cut_ups > 0 => readers::reader(w, staff, readers::COACH + you as u32, group),
        None => None,
    };
    let mine = rd.and_then(|mut rd| {
        // Alone in the room, your say is your weight.
        rd.say = 1.0;
        let mut lk: Vec<file::Evidence> = own_looks
            .iter()
            .filter(|l| l.date.add(rd.lag(l.kind, id, l.date)) <= w.today)
            .map(|l| file::Evidence { reader: 0, ..*l })
            .collect();
        let kind = super::film::session::film_kind(w, i);
        let filed = lk.iter().filter(|l| l.kind == kind).count();
        lk.extend((filed..cut_ups).map(|_| file::Evidence { reader: 0, kind, date: last }));
        if lk.is_empty() {
            return None;
        }
        let n = lk.len();
        let read = |rd: &file::Reader, some: &[file::Evidence]| {
            file::read_in(
                w,
                i,
                std::slice::from_ref(rd),
                some,
                Some(lk.as_slice()),
                &readers::norms(w, i),
                readers::medical_rating(w, staff),
                Some(w.today),
            )
        };
        let f = read(&rd, &lk);
        // Only what your own eye saw counts as yours: a shared measurement
        // isn't in your looks. What your tape can't show is a wide guess.
        let yours_of = |f: &file::FileRead, guesses: bool| {
            let mut v = f.per_reader.first().cloned().unwrap_or_default();
            v.resize(crate::attributes::ATTRIBUTE_COUNT, None);
            for (k, a) in f.attrs.iter().enumerate() {
                if guesses && a.guess && v[k].is_none() {
                    v[k] = Some((a.centre, a.sd));
                }
            }
            v
        };
        let attrs = yours_of(&f, true);
        // Your tape apart from your other looks (minor bug 352): your eye's
        // lasting misjudgment rides on your tape, not on what you saw in
        // person. The two are read on draws of their own, so what one gets
        // wrong isn't what the other does and they join as two reads.
        let (mut tape, mut others) = (Vec::new(), Vec::new());
        if lk.iter().any(|l| on_tape(l.kind)) {
            let (tapes, rest): (Vec<file::Evidence>, Vec<file::Evidence>) =
                lk.iter().copied().partition(|l| on_tape(l.kind));
            let mut on_film = rd.clone();
            on_film.key ^= TAPE_KEY;
            tape = yours_of(&read(&on_film, &tapes), false);
            others = if rest.is_empty() {
                vec![None; crate::attributes::ATTRIBUTE_COUNT]
            } else {
                yours_of(&read(&rd, &rest), true)
            };
            // What your other looks can't read keeps the file's wide guess,
            // so a trait your tape reads for the first time keeps it too.
            for (k, o) in others.iter_mut().enumerate() {
                if o.is_none() {
                    *o = Some(file::guess(w, i, crate::attributes::Attr::ALL[k]));
                }
            }
        }
        Some((f.now, attrs, n, tape, others))
    });
    let first = staff_read.and_then(|f| impression(w, id, f));
    let (mut tape, mut others) = (Vec::new(), Vec::new());
    let mut out = match (mine, first) {
        (Some((now, attrs, n, t, o)), Some(imp)) => {
            (tape, others) = (t, o);
            let (a, b) = (1.0 / (imp.1 * imp.1), 1.0 / (now.1 * now.1).max(0.25));
            Some(OwnRead { now: ((imp.0 * a + now.0 * b) / (a + b), (1.0 / (a + b)).sqrt()), attrs, looks: n })
        }
        (Some((now, attrs, n, t, o)), None) => {
            (tape, others) = (t, o);
            Some(OwnRead { now, attrs, looks: n })
        }
        (None, Some(imp)) => {
            Some(OwnRead { now: imp, attrs: vec![None; crate::attributes::ATTRIBUTE_COUNT], looks: 0 })
        }
        (None, None) => None,
    };
    // Lane 9 (SK43): what your own eye caught on his film.
    if let Some(r) = out.as_mut() {
        super::film::sharpen_with(w, id, w.people[i].player.position, &mut r.now, &mut r.attrs, &tape, &others, extra);
    }
    out
}

/// The club file's copy of a look of yours (minor bug 352): the kind the
/// club's file gives it beside the look under your name the same day (the
/// film room's watches and the hunt's games as film, a trip or an asked
/// workout as a coach's or an all-star look: `dept::club_copy`; a watch
/// from the stands at a game or an all-star week also as film in a save
/// from before this).
fn copies(club: Kind, yours: Kind) -> bool {
    match club {
        Kind::Film => matches!(yours, Kind::ProFilm | Kind::CollegeFilm | Kind::HsFilm | Kind::Game | Kind::AllStar),
        Kind::Coach => matches!(yours, Kind::Game | Kind::Practice | Kind::ProDay | Kind::Workout | Kind::HsFilm),
        Kind::AllStar => yours == Kind::AllStar,
        _ => false,
    }
}

/// Your eye on his tape (minor bug 352): a film look of yours (the film
/// room files one under your name for each watch; the hunt's games; old
/// film), a cross-check, a director's or a GM's look: the looks the file
/// sorts as film (`Kind::family`, but the analysts' numbers). They carry
/// your eye's lasting misjudgment of him, as your catches do
/// (`film::sharpen_with`). A coach's film and drills at a pro day or a
/// workout (`Kind::Coach`) reads as in person, as the file sorts it.
fn on_tape(k: Kind) -> bool {
    matches!(
        k,
        Kind::Film | Kind::CrossCheck | Kind::Director | Kind::Gm | Kind::HsFilm | Kind::CollegeFilm | Kind::ProFilm
    )
}

/// Your film looks of a player: one a game of his you have watched
/// (minor bug 338: a game is one look of him, however many of its snaps
/// and however often you watch it; reports saved before count one a six
/// snaps watched; not from the stands, which the trip filed), with the
/// day of the last: the reports kept and a session open on him.
pub fn cut_ups(w: &World, id: u32) -> (usize, crate::calendar::Date) {
    use super::film::{Target, session::Mode};
    let fr = &w.scouting.film;
    let of = |watched: usize| watched.div_ceil(6);
    let mut n = 0;
    let mut games: Vec<u64> = Vec::new();
    let mut last = w.today;
    let mut latest = None;
    for r in fr.reports.iter().filter(|r| r.target == Target::Player(id) && r.mode != Mode::Live) {
        if r.games.is_empty() {
            n += of(r.watched as usize);
        } else {
            games.extend_from_slice(&r.games);
        }
        latest = latest.max(Some(r.date));
    }
    if let Some(s) = fr.open.as_ref().filter(|s| s.target == Target::Player(id) && s.mode != Mode::Live) {
        // A snap stopped for your call is on the screen too.
        let started = s.step > 0 || s.at > 0;
        let seen = super::film::session::seen_games(w, s);
        if !seen.is_empty() || started {
            n += usize::from(seen.is_empty());
            games.extend(seen);
            latest = Some(w.today);
        }
    }
    games.sort_unstable();
    games.dedup();
    if let Some(d) = latest {
        last = d;
    }
    (n + games.len(), last)
}
