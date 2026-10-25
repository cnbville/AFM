//! Ties between people (people topic 2, the build 9 leftover): close
//! friends, mentors and family. Teammates and coaches come from history at
//! no cost; only close ties are stored, a few per player, as pairs of ids.
//!
//! Friends grow with time together, more in the same position room, the
//! same recruiting class or the same high school; leaders and loyal players
//! make more and deeper ties, a big ego or temper fewer. They fade by a
//! third a year apart. Mentors: a veteran at the same position with high
//! leadership and professionalism takes a young player under his wing, and
//! a bad veteran teaches bad habits the same way. Family: brothers in the
//! world, and coaches' sons on rosters.
//!
//! What ties do: friends on the roster lift a mood and losing one dips it;
//! a friend leaving through the portal pushes the lean; in free agency a
//! friend on a team pulls (`people::ties_q`). Sizes are Claude's picks
//! (proposed, not confirmed): up to 6 close friends, formed each fall camp.

use std::collections::HashMap;

use super::Team;
use super::talks::{self, Reason};
use crate::calendar::{Date, Weekday};
use crate::player::Trait;
use crate::world::{Stage, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Friend,
    /// `a` mentors `b`.
    Mentor,
    /// Brothers or cousins.
    Family,
}

/// One close tie: two player ids, its kind, strength (0-100) and the year it
/// began. Twelve bytes.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tie {
    pub a: u32,
    pub b: u32,
    pub kind: Kind,
    pub strength: u8,
    pub since: i16,
    /// Both on the same roster at the last look.
    pub together: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Ties {
    pub list: Vec<Tie>,
    /// The year the last fall's ties were drawn.
    pub drawn: i32,
    /// (player id, tie) sorted, built on first use after a change (never
    /// saved): free agency asks about friends for every offer.
    #[serde(skip)]
    index: std::sync::OnceLock<Vec<(u32, u32)>>,
}

impl Ties {
    /// Every tie a player is part of.
    pub fn of_id(&self, id: u32) -> impl Iterator<Item = &Tie> {
        let ix = self.index.get_or_init(|| {
            let mut v = Vec::with_capacity(self.list.len() * 2);
            for (k, t) in self.list.iter().enumerate() {
                v.push((t.a, k as u32));
                v.push((t.b, k as u32));
            }
            v.sort_unstable();
            v
        });
        let lo = ix.partition_point(|x| x.0 < id);
        ix[lo..].iter().take_while(move |x| x.0 == id).map(move |x| &self.list[x.1 as usize])
    }

    fn changed(&mut self) {
        self.index = std::sync::OnceLock::new();
    }
}

/// Close friends of a player (ids), strongest first.
pub fn friends(w: &World, id: u32) -> Vec<u32> {
    let mut v: Vec<(u32, u8)> = w
        .folk
        .ties
        .of_id(id)
        .filter(|t| t.kind == Kind::Friend && t.strength >= 35)
        .map(|t| (if t.a == id { t.b } else { t.a }, t.strength))
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    v.into_iter().map(|x| x.0).collect()
}

/// Every tie of a player, in words for his page: (other id, words).
pub fn of(w: &World, id: u32) -> Vec<(u32, String)> {
    w.folk
        .ties
        .of_id(id)
        .map(|t| {
            let other = if t.a == id { t.b } else { t.a };
            let close = if t.strength >= 70 {
                "very close"
            } else if t.strength >= 45 {
                "close"
            } else {
                "friendly"
            };
            let words = match t.kind {
                Kind::Friend => format!("{close} since {}", t.since),
                Kind::Mentor if t.a == id => format!("he mentors him (since {})", t.since),
                Kind::Mentor => format!("his mentor (since {})", t.since),
                Kind::Family => "family".to_string(),
            };
            (other, words)
        })
        .collect()
}

/// A player's mentor, when he has one on the roster.
pub fn mentor_of(w: &World, id: u32) -> Option<u32> {
    w.folk.ties.of_id(id).find(|t| t.kind == Kind::Mentor && t.b == id && t.together).map(|t| t.a)
}

/// The rosters ties are drawn on: every college program and NFL club, and
/// your high school.
fn rosters(w: &World) -> Vec<Vec<usize>> {
    let mut v: Vec<Vec<usize>> = w.programs.iter().map(|p| p.roster.clone()).collect();
    v.extend(w.clubs.iter().map(|c| c.roster.clone()));
    if let Some(Team::High(h)) = Team::seat(w) {
        v.push(w.high_schools[h].roster.clone());
    }
    v
}

/// Each fall camp: friendships form and deepen inside each roster, mentors
/// pair up, family ties are found; ties apart fade.
fn draw(w: &mut World) {
    let year = w.year;
    let seed = w.folk.base as u64 ^ 0x7E5 ^ (year as u64) << 32;
    let mut idx: HashMap<(u32, u32), usize> = HashMap::new();
    for (k, t) in w.folk.ties.list.iter().enumerate() {
        idx.insert((t.a.min(t.b), t.a.max(t.b)), k);
    }
    let mut count: HashMap<u32, u8> = HashMap::new();
    for t in &w.folk.ties.list {
        if t.kind == Kind::Friend {
            *count.entry(t.a).or_default() += 1;
            *count.entry(t.b).or_default() += 1;
        }
    }
    let mut on_team: HashMap<u32, usize> = HashMap::new();
    let mut new: Vec<Tie> = Vec::new();
    let mut deepen: Vec<usize> = Vec::new();
    for (r, roster) in rosters(w).iter().enumerate() {
        let live: Vec<usize> =
            roster.iter().copied().filter(|&i| w.people.get(i).is_some_and(|p| p.stage != Stage::Done)).collect();
        for &i in &live {
            on_team.insert(w.people[i].player.id, r);
        }
        // Friendship: each player looks at a handful of teammates near him
        // (same room, same class, same high school).
        for &i in &live {
            let p = &w.people[i];
            let id = p.player.id;
            let t = &p.player.personality.traits;
            let social = (1.0 + 0.35 * t.z(Trait::Leadership) + 0.25 * t.z(Trait::Loyalty)
                - 0.25 * t.z(Trait::Ego)
                - 0.2 * t.z(Trait::Temper))
            .clamp(0.3, 1.8);
            let cap = (2.0 + 4.0 * (social - 0.3) / 1.5).round() as u8;
            if count.get(&id).copied().unwrap_or(0) >= cap.min(6) {
                continue;
            }
            for (k, &j) in live.iter().enumerate() {
                if j == i {
                    continue;
                }
                let q = &w.people[j];
                let jd = q.player.id;
                if jd < id {
                    continue;
                }
                let same_room = crate::world::coach::Group::of(p.player.position)
                    == crate::world::coach::Group::of(q.player.position);
                let same_class = p.class_of == q.class_of;
                let same_hs = p.hs == q.hs;
                let near =
                    same_room as u8 as f64 * 0.04 + same_class as u8 as f64 * 0.025 + same_hs as u8 as f64 * 0.35;
                if near == 0.0 {
                    continue;
                }
                let tq = &q.player.personality.traits;
                let social_q = (1.0 + 0.35 * tq.z(Trait::Leadership) + 0.25 * tq.z(Trait::Loyalty)
                    - 0.25 * tq.z(Trait::Ego))
                .clamp(0.3, 1.8);
                let chance = near * social * social_q;
                let u = crate::world::season::key_uniform(seed ^ id as u64, k as u64 ^ (jd as u64) << 20);
                if u >= chance {
                    continue;
                }
                match idx.get(&(id.min(jd), id.max(jd))) {
                    Some(&e) => deepen.push(e),
                    None => {
                        if count.get(&jd).copied().unwrap_or(0) >= 6 {
                            continue;
                        }
                        new.push(Tie {
                            a: id,
                            b: jd,
                            kind: Kind::Friend,
                            strength: if same_hs { 55 } else { 38 },
                            since: year as i16,
                            together: true,
                        });
                        *count.entry(id).or_default() += 1;
                        *count.entry(jd).or_default() += 1;
                        idx.insert((id.min(jd), id.max(jd)), usize::MAX);
                    }
                }
            }
        }
        // Mentors: the best veteran mind at a position takes the youngest.
        // By position in a fixed order, so a world and its save draw alike.
        let mut by_pos: std::collections::BTreeMap<crate::positions::Position, Vec<usize>> = Default::default();
        for &i in &live {
            by_pos.entry(w.people[i].player.position).or_default().push(i);
        }
        for (_, group) in by_pos {
            let vet = group
                .iter()
                .copied()
                .filter(|&i| w.people[i].player.age >= if w.people[i].stage == Stage::Pro { 26 } else { 21 })
                .max_by(|&a, &b| {
                    let s = |i: usize| {
                        let t = &w.people[i].player.personality.traits;
                        t.get(Trait::Leadership) + 0.6 * t.get(Trait::Professionalism)
                    };
                    s(a).total_cmp(&s(b))
                });
            let young =
                group.iter().copied().filter(|&i| w.people[i].player.age <= 21).min_by_key(|&i| w.people[i].player.age);
            if let (Some(v), Some(y)) = (vet, young) {
                let lead = w.people[v].player.personality.traits.get(Trait::Leadership);
                let (a, b) = (w.people[v].player.id, w.people[y].player.id);
                if lead >= 13.0 && !w.folk.ties.of_id(b).any(|t| t.kind == Kind::Mentor && t.b == b) {
                    new.push(Tie { a, b, kind: Kind::Mentor, strength: 50, since: year as i16, together: true });
                }
            }
        }
        // Family: an older brother on the same roster.
        for &i in &live {
            let p = &w.people[i];
            if p.family.older_brother_at.is_none() || p.class_of < year {
                continue;
            }
            let last = p.name.split_whitespace().last().unwrap_or("");
            if let Some(&j) = live.iter().find(|&&j| {
                j != i
                    && w.people[j].hs == p.hs
                    && w.people[j].name.ends_with(last)
                    && w.people[j].class_of < p.class_of
            }) {
                let (a, b) = (w.people[j].player.id, p.player.id);
                if !w.folk.ties.of_id(b).any(|t| t.kind == Kind::Family && t.b == b) {
                    new.push(Tie { a, b, kind: Kind::Family, strength: 90, since: year as i16, together: true });
                }
            }
        }
    }
    for e in deepen {
        if let Some(t) = w.folk.ties.list.get_mut(e) {
            t.strength = (t.strength as u16 + 12).min(100) as u8;
        }
    }
    // Ties apart fade by a third a year; the gone are dropped.
    for t in w.folk.ties.list.iter_mut() {
        let a = on_team.get(&t.a);
        let together = a.is_some() && a == on_team.get(&t.b);
        t.together = together;
        if !together && t.kind != Kind::Family {
            t.strength = (t.strength as f32 * 0.67) as u8;
        }
    }
    w.folk.ties.list.retain(|t| {
        t.strength >= 15 && (on_team.contains_key(&t.a) || on_team.contains_key(&t.b) || t.kind == Kind::Family)
    });
    w.folk.ties.list.extend(new);
    w.folk.ties.changed();
    w.folk.ties.drawn = year;
}

/// A mentor's year (topic 2): a good one raises his young player's
/// professionalism; a bad one teaches bad habits.
fn mentor_year(w: &mut World) {
    let pairs: Vec<(u32, u32)> =
        w.folk.ties.list.iter().filter(|t| t.kind == Kind::Mentor && t.together).map(|t| (t.a, t.b)).collect();
    if pairs.is_empty() {
        return;
    }
    let index: HashMap<u32, usize> = w.people.iter().enumerate().map(|(k, p)| (p.player.id, k)).collect();
    for (a, b) in pairs {
        let (Some(&i), Some(&j)) = (index.get(&a), index.get(&b)) else { continue };
        let pro = w.people[i].player.personality.traits.get(Trait::Professionalism);
        let d = if pro >= 14.0 {
            1.0
        } else if pro <= 7.0 {
            -1.0
        } else {
            0.0
        };
        if d != 0.0 {
            w.people[j].player.personality.traits.shift(Trait::Professionalism, d);
        }
    }
}

/// Weekly for your roster: a close friend gone (cut, traded, transferred)
/// dips a mood; close friends on the roster lift it once a month.
fn your_week(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let roster: Vec<u32> = team.roster(w).iter().map(|&i| w.people[i].player.id).collect();
    let mut gone: Vec<(u32, u8)> = Vec::new();
    for t in w.folk.ties.list.iter_mut() {
        if t.kind == Kind::Mentor {
            continue;
        }
        let ina = roster.contains(&t.a);
        let inb = roster.contains(&t.b);
        if t.together && ina != inb {
            t.together = false;
            let stay = if ina { t.a } else { t.b };
            gone.push((stay, t.strength));
        } else if ina && inb {
            t.together = true;
        }
    }
    for (id, s) in gone {
        talks::mood_event(w, id, Reason::FriendLeft, -0.12 * s as f32 / 70.0);
    }
    if d.day() <= 7 {
        for &id in &roster {
            let n = w.folk.ties.of_id(id).filter(|t| t.kind == Kind::Friend && t.together && t.strength >= 45).count();
            if n >= 2 {
                talks::mood_event(w, id, Reason::FriendsHere, 0.015 * n.min(4) as f32);
            }
        }
    }
}

/// A friend's portal entry pushes the lean of his close friends still on
/// the roster (topic 2), by the tie and his loyalty.
fn portal_pull(w: &mut World, d: Date) {
    let entered: Vec<u32> = w
        .people
        .iter()
        .filter(|p| p.cal.portal.as_ref().is_some_and(|e| e.entered == d.add(-1)))
        .map(|p| p.player.id)
        .collect();
    if entered.is_empty() {
        return;
    }
    let pushes: Vec<(u32, f64)> = w
        .folk
        .ties
        .list
        .iter()
        .filter(|t| t.kind == Kind::Friend && t.together && t.strength >= 45)
        .filter_map(|t| {
            if entered.contains(&t.a) {
                Some((t.b, t.strength as f64))
            } else if entered.contains(&t.b) {
                Some((t.a, t.strength as f64))
            } else {
                None
            }
        })
        .collect();
    for (id, s) in pushes {
        let Some(i) = super::person(w, id) else { continue };
        let p = &mut w.people[i];
        if p.stage != Stage::College || p.cal.lean <= 0.0 {
            continue;
        }
        let f = 1.0 + 0.25 * s / 100.0 * (1.0 + 0.04 * (p.player.personality.traits.get(Trait::Loyalty) - 10.0));
        p.cal.lean *= f;
        p.cal.lean_events.push((d, "a close friend entered the portal", f));
    }
}

/// How many close friends a player has on an NFL club's roster (free
/// agency's pull, `people::ties_q`).
pub fn friends_on(w: &World, id: u32, t: usize) -> usize {
    let fr = friends(w, id);
    if fr.is_empty() {
        return 0;
    }
    let Some(club) = w.clubs.get(t) else { return 0 };
    fr.iter().filter(|&&f| super::person(w, f).is_some_and(|j| club.roster.contains(&j))).count()
}

pub(crate) fn day(w: &mut World, d: Date) {
    if d == w.dates.fall_camp && w.folk.ties.drawn < w.year {
        draw(w);
        mentor_year(w);
    }
    if d.weekday() == Weekday::Mon && w.seat.is_some() {
        your_week(w, d);
    }
    if d >= w.dates.portal_open.add(-40) && d <= w.dates.portal_close.add(130) {
        portal_pull(w, d);
    }
}
