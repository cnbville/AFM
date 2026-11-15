//! What people remember (lane N1, sections 3.6 and 10.2 to 10.5): the
//! stores in `Folk.live.persona` and what fills them.
//!
//! - His lies: logged with the day you could find out; on that day your
//!   read or your staff catches it, or not (some are never found).
//! - The grapevine: what you say travels through ties, a hop at a time, by
//!   each listener's loyalty, temper and professionalism, with his shade in
//!   the words. A request to keep quiet lowers the odds.
//! - Secrets, either way: kept by the keeper's loyalty and his trust in
//!   you; one you use where he can see it costs his trust.
//! - The colour notebook: small details only the voice reads.
//! - Unseen rule breaks: kept for your team when your staff didn't see
//!   them, with the teammates who were there.
//! - How you treated him.
//!
//! Every write is your seat's own (no AI team's world reads it), so no
//! world result changes. Every size here is Claude's (proposed, not
//! confirmed).

use super::Them;
use super::makeup::{self, Makeup};
use super::memory::{Colour, Heard, HisLie, Passed, Secret, Treated, Treatment, Unseen};
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::rules::RuleKind;
use crate::world::folk::{Team, roll, ties};

/// How long a row is kept, in days.
pub const KEEP_DAYS: i32 = 365;
/// His lies and the saved talks are kept two years: they can come back.
pub const KEEP_LIES_DAYS: i32 = 730;
/// The most rows a list keeps (the oldest go first).
pub const MAX_ROWS: usize = 600;
/// The most colour notes for one man.
pub const NOTES_EACH: usize = 12;
/// The most hops anything travels (10.5).
pub const MAX_HOPS: u8 = 3;
/// The most teammates who were there at an unseen break.
pub const THERE_MOST: usize = 3;

fn cap<T>(v: &mut Vec<T>) {
    if v.len() > MAX_ROWS {
        let cut = v.len() - MAX_ROWS;
        v.drain(..cut);
    }
}

// --- His lies (10.2) --------------------------------------------------------

/// Logs a lie he told you.
pub fn log_lie(w: &mut World, lie: HisLie) {
    let s = &mut w.folk.live.persona;
    s.his_lies.push(lie);
    cap(&mut s.his_lies);
}

/// How likely a lie is caught on its day, by what shows it: a case's day
/// in court is public, a hidden injury shows in practice, a signing shows
/// the offers; otherwise it takes your read.
fn catch_odds(w: &World, l: &HisLie) -> f64 {
    use super::knows::Topic as T;
    let sharp = crate::world::folk::talks::sharpness(w);
    match l.topic {
        T::Case => 0.9,
        T::Injury => 0.6 + 0.3 * sharp.min(1.0),
        T::Nil | T::Pitches | T::Market => 0.55 + 0.3 * sharp.min(1.0),
        _ => (0.2 + 0.5 * sharp).clamp(0.1, 0.8),
    }
}

/// On their day, your read or the facts catch his lies (or never do).
/// Returns the lies caught today.
pub fn catch_lies(w: &mut World) -> Vec<HisLie> {
    let today = w.today;
    let due: Vec<(usize, f64)> = w
        .folk
        .live
        .persona
        .his_lies
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.caught && l.shows.is_some_and(|d| d <= today))
        .map(|(k, l)| (k, catch_odds(w, l)))
        .collect();
    let mut out = Vec::new();
    for (k, odds) in due {
        let l = &w.folk.live.persona.his_lies[k];
        let u = roll(w, 0x9E70 ^ makeup::key(l.by), l.said.0 as u64 ^ (l.topic as u64) << 32);
        let l = &mut w.folk.live.persona.his_lies[k];
        if u < odds {
            l.caught = true;
            out.push(l.clone());
        } else {
            // Missed on its day: it stays hidden.
            l.shows = None;
        }
    }
    out
}

/// His lies you caught (what his word is worth to you now).
pub fn caught(w: &World, him: Them) -> usize {
    w.folk.live.persona.his_lies.iter().filter(|l| l.by == him && l.caught).count()
}

// --- The grapevine (10.5) ---------------------------------------------------

/// Who a man passes things on to: his close friends, his family, his agent,
/// a recruit's high school coach.
pub fn listeners(w: &World, him: Them) -> Vec<(Them, f64)> {
    let mut v = Vec::new();
    match him {
        Them::Player(id) => {
            for t in w.folk.ties.of_id(id) {
                let other = if t.a == id { t.b } else { t.a };
                if t.strength >= 35 || t.kind == ties::Kind::Family {
                    v.push((Them::Player(other), t.strength as f64 / 100.0));
                }
            }
            v.push((Them::Kin(id, 0), 0.9));
            if let Some(i) = crate::world::folk::person(w, id) {
                let p = &w.people[i];
                if let Some(a) = p.pro.as_ref().and_then(|c| c.agent) {
                    v.push((Them::Agent(a), 0.7));
                }
                if matches!(p.stage, crate::world::Stage::HighSchool | crate::world::Stage::PrepSchool)
                    && let Some(c) = w.high_schools.get(p.hs).and_then(|h| h.coach)
                {
                    v.push((Them::Coach(c), 0.6));
                }
            }
        }
        Them::Kin(id, _) => v.push((Them::Player(id), 0.95)),
        // An agent talks to clubs, not on the grapevine; others keep it.
        _ => {}
    }
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.truncate(6);
    v
}

/// The odds he passes something on (10.5): a loose, hot or unprofessional
/// man more, a loyal one less when it's a secret or you asked him to keep
/// it quiet; a closer tie hears more.
pub fn pass_odds(m: &Makeup, what: Passed, quiet: bool, tie: f64) -> f64 {
    let z = Makeup::z;
    let mut p = 0.35 - 0.25 * z(m.professionalism) + 0.15 * z(m.temper) - 0.05 * z(m.warmth);
    if matches!(what, Passed::Secret(_)) || quiet {
        p -= 0.25 * z(m.loyalty) + 0.1;
    }
    if quiet {
        p *= 0.5;
    }
    (p * (0.5 + tie)).clamp(0.02, 0.85)
}

/// Shade added at each hop: the words drift as they travel.
fn shaded(words: &str, hops: u8, from: &str) -> String {
    match hops {
        1 => format!("{from} said: {words}"),
        2 => format!("word is: {words}"),
        _ => format!("people are saying something like: {words}"),
    }
}

/// Who a man is, in a few words for the grapevine's lines.
fn who(w: &World, him: Them) -> String {
    match him {
        Them::Player(id) => crate::world::folk::person(w, id).map_or("a teammate".into(), |i| w.people[i].name.clone()),
        Them::Kin(..) => "his family".into(),
        Them::Coach(c) => w.coaches.get(c).map_or("a coach".into(), |x| x.name.clone()),
        Them::Agent(_) => "his agent".into(),
        _ => "someone".into(),
    }
}

/// What you said to `first` travels (10.5): each listener passes it on by
/// his own makeup, a hop at a time (one to four days a hop). Returns the
/// rows it wrote, each dated the day it reaches him.
pub fn spread(w: &mut World, first: Them, what: Passed, about: Option<Them>, words: &str, quiet: bool) -> Vec<Heard> {
    let today = w.today;
    let salt = words.bytes().fold(0u64, |h, b| h.wrapping_mul(131).wrapping_add(b as u64));
    let mut heard: Vec<Them> = vec![first];
    let mut wave: Vec<(Them, Date)> = vec![(first, today)];
    let mut out = Vec::new();
    for hops in 1..=MAX_HOPS {
        let mut next = Vec::new();
        for (teller, day) in &wave {
            let m = makeup::makeup(w, *teller);
            let from = if *teller == first && hops == 1 { "you".to_string() } else { who(w, *teller) };
            for (to, tie) in listeners(w, *teller) {
                if heard.contains(&to) {
                    continue;
                }
                let u =
                    roll(w, 0x9E71 ^ salt ^ makeup::key(*teller).rotate_left(7), makeup::key(to) ^ (hops as u64) << 56);
                if u >= pass_odds(&m, what, quiet, tie) {
                    continue;
                }
                let lag = 1 + (roll(w, 0x9E72 ^ salt, makeup::key(to)) * 4.0) as i32;
                let on = day.add(lag.min(4));
                heard.push(to);
                let h = Heard {
                    from: *teller,
                    to,
                    what,
                    day: on,
                    hops,
                    about,
                    words: shaded(words, hops, if hops == 1 { &from } else { "someone" }),
                };
                out.push(h.clone());
                next.push((to, on));
            }
        }
        if next.is_empty() {
            break;
        }
        wave = next;
    }
    let s = &mut w.folk.live.persona;
    s.grapevine.extend(out.iter().cloned());
    cap(&mut s.grapevine);
    out
}

/// What has reached `him` by today.
pub fn heard_by(w: &World, him: Them) -> Vec<&Heard> {
    w.folk.live.persona.grapevine.iter().filter(|h| h.to == him && h.day <= w.today).collect()
}

/// Whether the man it's about has heard it by today ("call a teammate soft
/// and he hears it").
pub fn reached(w: &World, about: Them, what: Passed) -> bool {
    w.folk.live.persona.grapevine.iter().any(|h| h.to == about && h.what == what && h.day <= w.today)
}

// --- Secrets (10.3) ---------------------------------------------------------

/// Logs something said "between us", either way. Returns its place.
pub fn confide(w: &mut World, by: Them, to: Them, yours: bool, words: &str, asked_quiet: bool) -> u32 {
    let s = &mut w.folk.live.persona;
    // Secrets are few and keep their places (the grapevine names them by
    // place), so they are never trimmed.
    s.secrets.push(Secret { by, yours, to, day: w.today, words: words.into(), asked_quiet, out: false });
    (s.secrets.len() - 1) as u32
}

/// The odds a man keeps a secret you told him (10.3): his loyalty and his
/// trust in you keep it, a loose tongue doesn't.
pub fn keep_odds(w: &World, keeper: Them, asked_quiet: bool) -> f64 {
    let m = makeup::makeup(w, keeper);
    let z = Makeup::z;
    let t = makeup::trust(w, keeper);
    let k = 0.6 + 0.3 * z(m.loyalty) + 0.5 * (t - 0.5) + 0.2 * z(m.professionalism) - 0.1 * z(m.temper)
        + if asked_quiet { 0.1 } else { 0.0 };
    k.clamp(0.1, 0.97)
}

/// Whether a secret you told gets out: the keeper passes it on the
/// grapevine if he doesn't keep it. Returns the rows it wrote.
pub fn test_secret(w: &mut World, idx: u32) -> Vec<Heard> {
    let Some(s) = w.folk.live.persona.secrets.get(idx as usize).cloned() else { return Vec::new() };
    if !s.yours || s.out {
        return Vec::new();
    }
    let odds = keep_odds(w, s.to, s.asked_quiet);
    let u = roll(w, 0x9E73 ^ makeup::key(s.to), s.day.0 as u64 ^ idx as u64);
    if u < odds {
        return Vec::new();
    }
    w.folk.live.persona.secrets[idx as usize].out = true;
    // It goes out from the keeper.
    let words = s.words.clone();
    let mut rows = Vec::new();
    for (to, _) in listeners(w, s.to).into_iter().take(2) {
        rows.push(Heard {
            from: s.to,
            to,
            what: Passed::Secret(idx),
            day: s.day.add(2),
            hops: 1,
            about: None,
            words: format!("the coach told him in confidence: {words}"),
        });
    }
    let p = &mut w.folk.live.persona;
    p.grapevine.extend(rows.iter().cloned());
    cap(&mut p.grapevine);
    rows
}

/// You used something he told you "between us" where he can see it (10.3):
/// it costs his trust, more with his loyalty. Returns the trust lost.
pub fn used_secret(w: &mut World, idx: u32) -> f32 {
    let Some(s) = w.folk.live.persona.secrets.get(idx as usize).cloned() else { return 0.0 };
    if s.yours || s.out {
        return 0.0;
    }
    w.folk.live.persona.secrets[idx as usize].out = true;
    let m = makeup::makeup(w, s.by);
    let cost = (0.08 + 0.06 * Makeup::z(m.loyalty).max(0.0) + 0.04 * Makeup::z(m.ego).max(0.0)) as f32;
    nudge_trust(w, s.by, -cost);
    treat(w, s.by, Treated::Dodged);
    cost
}

// --- Trust ------------------------------------------------------------------

/// Moves his trust in you where the world keeps it: your player's trust in
/// his head coach, an agent's trust in your word, everyone else's in
/// `Mine.trust`.
pub fn nudge_trust(w: &mut World, him: Them, by: f32) {
    let key = makeup::key(him);
    match him {
        Them::Player(_) => {
            let you = makeup::your_coach(w);
            if let Some(i) = makeup::index(w, him)
                && you.is_some_and(|c| w.people[i].trust_in as usize == c)
                && makeup::team_of(w, i) == Team::seat(w)
            {
                let p = &mut w.people[i];
                p.trust = (p.trust + by).clamp(0.0, 1.0);
                return;
            }
            let t = makeup::trust(w, him) as f32;
            w.folk.mine.set_trust(key, (t + by).clamp(0.0, 1.0));
        }
        Them::Agent(_) => {
            let t = w.folk.live.word_of(key);
            w.folk.live.set_word(key, t + by);
        }
        _ => {
            let t = makeup::trust(w, him) as f32;
            w.folk.mine.set_trust(key, (t + by).clamp(0.0, 1.0));
        }
    }
}

// --- The colour notebook (3.6, question 5) ----------------------------------

/// A detail the voice added. The same note twice is kept once; each man
/// keeps his newest few.
pub fn note(w: &mut World, who: Them, note: &str) {
    let note = note.trim();
    if note.is_empty() {
        return;
    }
    let day = w.today;
    let s = &mut w.folk.live.persona;
    if s.notebook.iter().any(|c| c.who == who && c.note.eq_ignore_ascii_case(note)) {
        return;
    }
    s.notebook.push(Colour { who, day, note: note.chars().take(160).collect() });
    let mine: Vec<usize> = s.notebook.iter().enumerate().filter(|(_, c)| c.who == who).map(|(k, _)| k).collect();
    if mine.len() > NOTES_EACH {
        s.notebook.remove(mine[0]);
    }
    cap(&mut s.notebook);
}

/// His notes, oldest first: only the voice reads them.
pub fn notes(w: &World, who: Them) -> Vec<&str> {
    w.folk.live.persona.notebook.iter().filter(|c| c.who == who).map(|c| c.note.as_str()).collect()
}

// --- Unseen rule breaks (3.1) -----------------------------------------------

/// A break your staff didn't see: kept with the teammates who were there
/// (his close friends on the roster first, then men from his unit; keyed,
/// never the world's stream).
pub fn record_unseen(w: &mut World, i: usize, rule: RuleKind, day: Date) {
    let id = w.people[i].player.id;
    let team = makeup::team_of(w, i);
    let roster: Vec<usize> = team.map(|t| t.roster(w)).unwrap_or_default();
    let on: Vec<u32> = roster.iter().map(|&j| w.people[j].player.id).collect();
    let mut there: Vec<u32> = ties::friends(w, id).into_iter().filter(|f| on.contains(f)).collect();
    there.truncate(2);
    let g = crate::world::coach::Group::of(w.people[i].player.position);
    let mut unit: Vec<(f64, u32)> = roster
        .iter()
        .filter(|&&j| j != i && crate::world::coach::Group::of(w.people[j].player.position) == g)
        .map(|&j| (roll(w, 0x9E74 ^ day.0 as u64, w.people[j].player.id as u64), w.people[j].player.id))
        .filter(|x| !there.contains(&x.1))
        .collect();
    unit.sort_by(|a, b| a.0.total_cmp(&b.0));
    let n = 1 + (roll(w, 0x9E75 ^ day.0 as u64, id as u64) * 2.0) as usize;
    there.extend(unit.into_iter().take(n).map(|x| x.1));
    there.truncate(THERE_MOST);
    let s = &mut w.folk.live.persona;
    s.unseen.push(Unseen { player: id, rule, day, there, found: false });
    cap(&mut s.unseen);
}

/// An unseen break reached your staff (a teammate told, you asked).
pub fn found(w: &mut World, player: u32, day: Date) {
    for u in w.folk.live.persona.unseen.iter_mut().filter(|u| u.player == player && u.day == day) {
        u.found = true;
    }
}

// --- How you treated him (3.6) ----------------------------------------------

pub fn treat(w: &mut World, who: Them, how: Treated) {
    let day = w.today;
    let s = &mut w.folk.live.persona;
    s.treated.push(Treatment { who, how, day });
    cap(&mut s.treated);
}

/// How you treated him in the last `days`.
pub fn treated(w: &World, who: Them, days: i32) -> Vec<Treated> {
    w.folk
        .live
        .persona
        .treated
        .iter()
        .filter(|t| t.who == who && t.day.days_to(w.today) <= days)
        .map(|t| t.how)
        .collect()
}

// --- Upkeep -----------------------------------------------------------------

/// Weekly: old rows go (a year for most, two for lies and saved talks;
/// secrets keep their places); lies due today are checked.
pub fn week(w: &mut World) {
    if w.folk.live.persona.is_empty() {
        return;
    }
    let _ = catch_lies(w);
    let today = w.today;
    let s = &mut w.folk.live.persona;
    let young = |d: Date, keep: i32| d.days_to(today) <= keep;
    s.grapevine.retain(|h| young(h.day, KEEP_DAYS));
    s.treated.retain(|t| young(t.day, KEEP_DAYS));
    s.unseen.retain(|u| young(u.day, KEEP_DAYS));
    s.his_lies.retain(|l| young(l.said, KEEP_LIES_DAYS));
    s.talks.retain(|t| young(t.day, KEEP_LIES_DAYS));
}
