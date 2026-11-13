//! NFL alumni by program and position: the one index the fact sheet adds
//! (talk facts design, section 3). Every drafted player a program sent to
//! the league, with the coaches he had there, so "your draft picks at his
//! position, by name" and "the players you coached yourself" are facts a
//! talk can stand on.
//!
//! Nothing is saved: the index is built from `World.people` (pros still
//! playing) and `World.alumni` (pros who retired) the first time a sheet
//! asks, and again once the alumni list has grown, the draft is held or
//! the year has turned. A man who retires in between reads as still
//! playing until that rebuild.

use crate::positions::Position;
use crate::world::World;
use crate::world::coach::Group;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A program's picks count for this many drafts back (as the position
/// names read them, `legacy::college::position_counts`).
/// Proposed, not confirmed.
pub const PICK_YEARS: i32 = 10;

/// One drafted player.
#[derive(Clone, Debug, PartialEq)]
pub struct Alum {
    pub name: String,
    /// His player id while he is still in the world's people.
    pub id: Option<u32>,
    /// The program he was drafted from.
    pub program: u32,
    pub position: Position,
    /// His draft: the year, the round and the overall pick.
    pub year: i32,
    pub round: u8,
    pub pick: u16,
    /// The club that drafted him.
    pub club: String,
    /// Still playing.
    pub active: bool,
    /// His head coach and his position coach in his last season at that
    /// program (coach indices).
    pub head: Option<usize>,
    pub position_coach: Option<usize>,
}

impl Alum {
    /// Whether a coach had him: as his head coach or his position coach in
    /// his last season before the draft.
    pub fn coached_by(&self, c: usize) -> bool {
        self.head == Some(c) || self.position_coach == Some(c)
    }
}

/// The index: every drafted player, and where to find a program's and a
/// coach's.
#[derive(Debug, Default)]
pub struct Index {
    stamp: (i32, usize, bool),
    all: Vec<Alum>,
    /// (program, position group) to places in `all`, newest draft first,
    /// then the earlier pick.
    by_room: HashMap<(u32, u8), Vec<u32>>,
    by_program: HashMap<u32, Vec<u32>>,
    by_coach: HashMap<usize, Vec<u32>>,
}

fn group_index(g: Group) -> u8 {
    Group::ALL.iter().position(|&x| x == g).unwrap_or(0) as u8
}

impl Index {
    fn build(w: &World) -> Index {
        let mut all: Vec<Alum> = Vec::new();
        let coaches = |staff: &[(i32, usize, Option<usize>, Option<usize>)], prog: usize| {
            staff.iter().rev().find(|s| s.1 == prog).map_or((None, None), |s| (s.2, s.3))
        };
        // Pros still playing. A man who retired keeps his place in the
        // people until the world tidies them; once his career is written
        // to the alumni list he is counted there, below, and not here.
        for p in &w.people {
            let Some((year, round, pick, club)) = p.pro.as_ref().and_then(|c| c.draft) else { continue };
            if p.pro.as_ref().and_then(|c| c.alumnus).and_then(|k| w.alumni.get(k)).is_some_and(|a| a.pro.is_some()) {
                continue;
            }
            let Some(&prog) = p.path.last().or(p.college.as_ref()) else { continue };
            let (head, position_coach) = coaches(&p.college_staff, prog);
            all.push(Alum {
                name: p.name.clone(),
                id: Some(p.player.id),
                program: prog as u32,
                position: p.player.position,
                year,
                round: round.min(255) as u8,
                pick: pick.min(u16::MAX as u32) as u16,
                club: w.nfl_team_name(club),
                active: p.stage == crate::world::Stage::Pro,
                head,
                position_coach,
            });
        }
        // Pros who retired: the alumni list keeps their careers. A drafted
        // man with no career written yet is still in the people above.
        for a in &w.alumni {
            let (Some(pick), Some(&prog), true) = (a.pick, a.colleges.last(), a.pro.is_some()) else { continue };
            let (head, position_coach) = coaches(&a.college_staff, prog);
            // His career keeps the round and the club; the alumni row has
            // the pick and the club's name that day.
            let drafted = a.pro.as_ref().and_then(|c| c.draft);
            all.push(Alum {
                name: a.name.clone(),
                id: None,
                program: prog as u32,
                position: a.position,
                year: a.year,
                round: drafted.map_or(super::words::round_of(pick), |d| d.1).min(255) as u8,
                pick: pick.min(u16::MAX as u32) as u16,
                club: a
                    .nfl_team
                    .map(str::to_string)
                    .or_else(|| drafted.map(|d| w.nfl_team_name(d.3)))
                    .unwrap_or_default(),
                active: false,
                head,
                position_coach,
            });
        }
        all.sort_by(|a, b| b.year.cmp(&a.year).then(a.pick.cmp(&b.pick)).then(a.name.cmp(&b.name)));
        let mut ix = Index { stamp: stamp(w), ..Default::default() };
        for (k, a) in all.iter().enumerate() {
            let k = k as u32;
            ix.by_room.entry((a.program, group_index(Group::of(a.position)))).or_default().push(k);
            ix.by_program.entry(a.program).or_default().push(k);
            for c in [a.head, a.position_coach].into_iter().flatten() {
                let v = ix.by_coach.entry(c).or_default();
                if v.last() != Some(&k) {
                    v.push(k);
                }
            }
        }
        ix.all = all;
        ix
    }

    fn pick<'a>(&'a self, places: Option<&'a Vec<u32>>) -> impl Iterator<Item = &'a Alum> + 'a {
        places.into_iter().flatten().map(|&k| &self.all[k as usize])
    }

    /// Everyone drafted, newest draft first.
    pub fn all(&self) -> &[Alum] {
        &self.all
    }

    /// A program's picks at a position group, newest draft first.
    pub fn at(&self, program: usize, g: Group) -> impl Iterator<Item = &Alum> + '_ {
        self.pick(self.by_room.get(&(program as u32, group_index(g))))
    }

    /// A program's picks at the group a position belongs to, in the last
    /// `PICK_YEARS` drafts up to `year`.
    pub fn recent(&self, program: usize, pos: Position, year: i32) -> impl Iterator<Item = &Alum> + '_ {
        self.at(program, Group::of(pos)).filter(move |a| a.year <= year && a.year > year - PICK_YEARS)
    }

    /// Every pick of a program, newest draft first.
    pub fn of(&self, program: usize) -> impl Iterator<Item = &Alum> + '_ {
        self.pick(self.by_program.get(&(program as u32)))
    }

    /// A program's picks in one draft, the earliest pick first.
    pub fn class(&self, program: usize, year: i32) -> impl Iterator<Item = &Alum> + '_ {
        self.of(program).filter(move |a| a.year == year)
    }

    /// The drafted players a coach had, at any program, newest first.
    pub fn coached(&self, c: usize) -> impl Iterator<Item = &Alum> + '_ {
        self.pick(self.by_coach.get(&c))
    }
}

/// The day a round of this year's draft is held: round 1's night, rounds
/// 2 and 3, then the rest.
pub(super) fn draft_day(w: &World, round: u8) -> crate::calendar::Date {
    let d = &w.dates.draft;
    match round {
        0 | 1 => d[0].date,
        2 | 3 => d[1].date,
        _ => d[2].date,
    }
}

/// What the index was built from: the year, the alumni list's length and
/// whether this year's draft has been held.
fn stamp(w: &World) -> (i32, usize, bool) {
    (w.year, w.alumni.len(), w.today >= w.dates.draft[2].date)
}

/// The index's place in the world (`Folk.alumni`): never saved, built when
/// first asked for.
#[derive(Debug, Default)]
pub struct Alumni(Mutex<Option<Arc<Index>>>);

impl Clone for Alumni {
    fn clone(&self) -> Alumni {
        Alumni(Mutex::new(self.0.lock().ok().and_then(|g| g.clone())))
    }
}

impl Alumni {
    /// The index for the world as it is today.
    pub fn get(w: &World) -> Arc<Index> {
        let slot = &w.folk.alumni.0;
        if let Ok(mut g) = slot.lock() {
            if let Some(ix) = g.as_ref().filter(|ix| ix.stamp == stamp(w)) {
                return ix.clone();
            }
            let ix = Arc::new(Index::build(w));
            *g = Some(ix.clone());
            return ix;
        }
        Arc::new(Index::build(w))
    }
}
