//! Your seat's side of flaws and tools (individual practice topics 4-9):
//! the points of emphasis on your players, what your staff has seen of
//! them, the tape book of every rep a flaw or tool decided, and the week's
//! work. AI staffs work theirs in the year's step on the same rule
//! (`craft::year`). Every size here is proposed, not confirmed.

use super::World;
use crate::calendar::Date;
use crate::craft::{Craft, GONE, Point, Stage, Teach};
use crate::game::stats::PlayRecord;
use crate::positions::Position;
use crate::world::coach::Group;
use crate::world::folk::Team;
use crate::world::season::key_uniform;
use std::collections::HashMap;

/// One game's or one practice week's reps a flaw or tool decided for one
/// of your players (topic 4.4: the tape tags).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tag {
    pub id: u32,
    /// The craft's index.
    pub code: u8,
    pub day: Date,
    /// True for practice snaps, false for a game.
    pub practice: bool,
    /// The reps it showed in, and those it lost him (a flaw that cost him,
    /// or a tool that didn't work).
    pub shows: u16,
    pub lost: u16,
}

/// What your staff has named (topic 4): a player, the craft's index, the
/// day they named it. A weak eye can name the wrong one (topic 4.3): the
/// entry stands until the tape talks them out of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Seen {
    pub id: u32,
    pub code: u8,
    pub day: Date,
}

/// How long the tape book keeps a rep (two seasons).
const BOOK_DAYS: i32 = 730;

impl World {
    /// Your players' persons now.
    pub(crate) fn seat_roster(&self) -> &[usize] {
        let Some(seat) = self.seat else { return &[] };
        match seat.level {
            crate::calendar::Level::Nfl => seat.nfl_team.and_then(|t| self.clubs.get(t)).map_or(&[], |c| &c.roster),
            crate::calendar::Level::HighSchool => seat.program.map_or(&[], |h| &self.high_schools[h].roster),
            crate::calendar::Level::Other => &[],
            _ => seat.program.map_or(&[], |p| &self.programs[p].roster),
        }
    }

    /// His points of emphasis as you set them, in order (one or two).
    pub fn points_of(&self, player: u32) -> Vec<Point> {
        self.seat_focus.iter().filter(|x| x.0 == player).filter_map(|x| Point::from_code(x.1)).take(2).collect()
    }

    /// Sets his points of emphasis (up to two; none gives them back to the
    /// staff). A veteran star may turn down work on a flaw he doesn't
    /// believe he has (topic 8.2): that point is left off, his trust in you
    /// drops a little, and the refused flaw comes back.
    pub fn set_points(&mut self, player: u32, points: &[Point]) -> Option<Craft> {
        let old = self.points_of(player);
        let mut keep: Vec<Point> = Vec::new();
        let mut refused = None;
        for &pt in points.iter().take(2) {
            if keep.contains(&pt) {
                continue;
            }
            if let (Point::Craft(c), false) = (pt, old.contains(&pt))
                && c.is_flaw()
                && let Some(i) = super::folk::person(self, player)
            {
                let star = super::folk::talks::is_starter(self, i);
                let p = &self.people[i];
                let chance = crate::craft::refuse_chance(&p.player, p.trust as f64, star);
                let u = key_uniform(self.today.0 as u64 ^ 0x5EF0, (player as u64) << 8 ^ c as u64);
                if u < chance {
                    refused = Some(c);
                    let pp = &mut self.people[i];
                    pp.trust = (pp.trust - 0.03).max(0.0);
                    super::folk::talks::mood_event(self, player, super::folk::talks::Reason::Practice, -0.03);
                    continue;
                }
            }
            keep.push(pt);
        }
        self.seat_focus.retain(|x| x.0 != player);
        self.seat_focus.extend(keep.iter().map(|p| (player, p.code())));
        refused
    }

    /// What your staff has named on him, oldest first: (craft, the day).
    pub fn seen_of(&self, player: u32) -> Vec<(Craft, Date)> {
        self.seat_seen
            .iter()
            .filter(|s| s.id == player)
            .filter_map(|s| Craft::from_index(s.code as usize).map(|c| (c, s.day)))
            .collect()
    }

    /// The tape book's rows for him, oldest first.
    pub fn tags_of(&self, player: u32) -> impl Iterator<Item = &Tag> {
        self.seat_book.iter().filter(move |t| t.id == player)
    }

    /// The points his staff works when you set none (topic 5.2): of what
    /// they've named, the two the tape shows costing him most lately (a
    /// moulding staff weighs what its system leans on at his spot).
    pub fn staff_points_of(&self, player: u32) -> Vec<Craft> {
        let Some(i) = super::folk::person(self, player) else { return Vec::new() };
        let pos = self.people[i].player.position;
        let since = self.today.add(-56);
        let mould = seat_mould(self);
        let top = crate::attributes::Attr::ALL.iter().map(|&a| crate::positions::weight(pos, a)).fold(1e-9, f64::max);
        let mut v: Vec<(Craft, f64)> = self
            .seen_of(player)
            .into_iter()
            .filter(|(c, _)| c.is_flaw() && c.fits(pos))
            .map(|(c, _)| {
                let lost: u16 = self
                    .tags_of(player)
                    .filter(|t| t.code == c as u8 && t.day.days_to(since) <= 0)
                    .map(|t| t.lost)
                    .sum();
                let m = ((mould - 1.0) / 19.0).clamp(0.0, 1.0);
                let w = crate::positions::weight(pos, c.owner()) / top;
                (c, (1.0 + lost as f64) * (1.0 - m + m * (0.3 + 0.7 * w)))
            })
            .collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1).then((a.0 as u8).cmp(&(b.0 as u8))));
        v.into_iter().take(2).map(|x| x.0).collect()
    }

    /// His individual minutes last week (on the field, in the film room);
    /// the usual 30 and 30 before his first week with you.
    pub fn minutes_of(&self, player: u32) -> (f64, f64) {
        let usual = crate::development::USUAL_FOCUS_MINUTES;
        self.seat_minutes.iter().find(|m| m.0 == player).map_or((usual, usual), |m| (m.1 as f64, m.2 as f64))
    }

    /// The flaws and tools he works this week: yours if you set any, else
    /// the staff's; a plain rating point works no flaw.
    pub fn worked_on(&self, player: u32) -> Vec<Craft> {
        let mine = self.points_of(player);
        if mine.is_empty() { self.staff_points_of(player) } else { mine.iter().filter_map(|p| p.craft()).collect() }
    }
}

/// Your staff's style (1 adapting to 20 moulding), as the year's step reads
/// it.
fn seat_mould(w: &World) -> f64 {
    let coach = match Team::seat(w) {
        Some(Team::College(p)) => w.programs[p].head_coach,
        Some(Team::High(h)) => w.high_schools[h].coach,
        Some(Team::Nfl(t)) => w.nfl.get(t).and_then(|c| c.head_coach),
        _ => None,
    };
    coach.and_then(|c| w.coaches.get(c)).map_or(10.0, |c| c.style().mould)
}

/// Who teaches his group, and his eye (topic 7.1): the position coach at
/// a college, the coordinator of his side at an NFL club or a high school
/// (the world keeps no NFL position coaches), else the head coach; an
/// empty room reads as a fair teacher (8).
pub fn room(w: &World, pos: Position) -> (f64, f64) {
    let g = Group::of(pos);
    let side = |oc: Option<usize>, dc: Option<usize>| if g.offense() { oc } else { dc };
    let c = match Team::seat(w) {
        Some(Team::College(p)) => {
            let pr = &w.programs[p];
            pr.position_coaches[g as usize].or(side(pr.oc, pr.dc)).or(pr.head_coach)
        }
        Some(Team::Nfl(t)) => w.nfl.get(t).and_then(|c| side(c.oc, c.dc).or(c.head_coach)),
        Some(Team::High(h)) => {
            let s = &w.high_schools[h];
            side(s.oc, s.dc).or(s.coach)
        }
        _ => None,
    };
    c.and_then(|c| w.coaches.get(c)).map_or((8.0, 8.0), |c| (c.teaching(g), c.evaluation()))
}

/// Books the reps a flaw or tool decided for your players (topic 4.4):
/// one row a player and craft a game (or a practice week).
pub(crate) fn book_plays(w: &mut World, plays: &[PlayRecord], practice: bool) {
    if w.seat.is_none() || plays.is_empty() {
        return;
    }
    let mine: std::collections::HashSet<u32> = w.seat_roster().iter().map(|&i| w.people[i].player.id).collect();
    let mut rows: HashMap<(u32, u8), (u16, u16)> = HashMap::new();
    for r in plays {
        for (id, c, won) in r.craft_tags() {
            if mine.contains(&id) {
                let e = rows.entry((id, c as u8)).or_default();
                e.0 = e.0.saturating_add(1);
                e.1 = e.1.saturating_add((!won) as u16);
            }
        }
    }
    let day = w.today;
    let mut add: Vec<Tag> =
        rows.into_iter().map(|((id, code), (shows, lost))| Tag { id, code, day, practice, shows, lost }).collect();
    add.sort_by_key(|t| (t.id, t.code));
    w.seat_book.retain(|t| t.day.days_to(day) <= BOOK_DAYS);
    w.seat_book.extend(add);
}

/// Your team's week for flaws and tools (topics 4, 6-8): your staff's eye
/// finds (or misnames) what the tape and practice show, each player works
/// his points with the minutes his week gives them (`minutes`: plain and
/// film-room, by person), and a fix that lands says so. Returns the notes.
pub(crate) fn week(w: &mut World, roster: &[usize], minutes: &HashMap<usize, (f64, f64)>) -> Vec<(u32, String)> {
    let day = w.today;
    let key = day.0 as u64 ^ 0xC2A7;
    let mut notes = Vec::new();
    let week_ago = day.add(-7);
    for &i in roster {
        let (id, pos) = (w.people[i].player.id, w.people[i].player.position);
        let (teaching, eye) = room(w, pos);
        let eyef = (eye / 10.0).clamp(0.3, 1.8);
        // The eye (topic 4.2-4.3): each held one not yet named may be found
        // this week, sooner with a better eye, a bigger habit and more reps
        // of it on tape; a weak eye can name a neighbour instead.
        let shows = |w: &World, c: Craft| -> u16 {
            w.tags_of(id).filter(|t| t.code == c as u8 && t.day.days_to(week_ago) <= 0).map(|t| t.shows).sum()
        };
        let held: Vec<(Craft, f64)> =
            w.people[i].player.craft.iter().filter(|(_, it)| it.sev() >= GONE).map(|(c, it)| (c, it.sev())).collect();
        let named: Vec<u8> = w.seat_seen.iter().filter(|s| s.id == id).map(|s| s.code).collect();
        for (c, sev) in held {
            if named.contains(&(c as u8)) {
                continue;
            }
            let chance = 0.06 * eyef * (0.3 + sev) * (1.0 + 0.5 * shows(w, c) as f64);
            if key_uniform(key, (id as u64) << 8 ^ c as u64) >= chance {
                continue;
            }
            let wrong = (0.32 - 0.016 * eye).clamp(0.04, 0.3);
            let mut code = c as u8;
            if key_uniform(key ^ 0x3B, (id as u64) << 8 ^ c as u64) < wrong {
                let pc = &w.people[i].player.craft;
                let near: Vec<Craft> = Craft::for_position(pos)
                    .filter(|&x| x != c && x.is_flaw() == c.is_flaw() && !pc.has(x) && !named.contains(&(x as u8)))
                    .filter(|&x| x.owner().profile().group == c.owner().profile().group)
                    .collect();
                if !near.is_empty() {
                    let k = (key_uniform(key ^ 0x3C, id as u64) * near.len() as f64) as usize % near.len();
                    code = near[k] as u8;
                }
            }
            w.seat_seen.push(Seen { id, code, day });
        }
        // A wrong name fades as the tape never shows it.
        let pc = w.people[i].player.craft;
        w.seat_seen.retain(|s| {
            s.id != id
                || pc.has_code(s.code)
                || s.day.days_to(day) < 28
                || key_uniform(key ^ 0x3D, (id as u64) << 8 ^ s.code as u64) >= 0.05 * eyef
        });
        // The work (topics 5-7).
        // A week with no individual time (his own time, winter's weight
        // room, the summer program) moves nothing and keeps the minutes of
        // his last week with it for the screens' projection.
        let (plain, film) = minutes.get(&i).copied().unwrap_or((0.0, 0.0));
        if plain <= 0.0 && film <= 0.0 {
            continue;
        }
        let kept = (id, plain.round().clamp(0.0, 255.0) as u8, film.round().clamp(0.0, 255.0) as u8);
        match w.seat_minutes.iter_mut().find(|m| m.0 == id) {
            Some(m) => *m = kept,
            None => w.seat_minutes.push(kept),
        }
        let work = w.worked_on(id);
        if work.is_empty() {
            continue;
        }
        let share = work.len() as f64;
        let trust = w.people[i].trust as f64;
        let buy = crate::craft::buy_in(&w.people[i].player, trust);
        for c in work {
            let m = if c.info().mental { film } else { plain } / share;
            if m <= 0.0 {
                continue;
            }
            let t = Teach { teaching, minutes: m, buy_in: buy };
            let Some((before, after)) = crate::craft::work_week(&mut w.people[i].player, c, &t) else { continue };
            if after > before {
                // He feels it (topic 8.4).
                let what = c.name().to_lowercase();
                let line = match after {
                    Stage::Games => format!("says the work on {what} is holding up in games"),
                    _ => format!("is showing the work on {what} in practice"),
                };
                notes.push((id, line));
                if after == Stage::Games {
                    super::folk::talks::mood_event(w, id, super::folk::talks::Reason::Practice, 0.03);
                }
            }
        }
    }
    notes
}

/// His rating by each coming year's end with a focus on it and without
/// (topic 5b.6, 9c.5 and 9c.7), grown on copies of him with the same dice
/// under his room's teacher and his role: `years` pairs of (without, with).
/// The truth: the screens read it through the staff's eye.
pub fn focus_projection(
    w: &World,
    player: u32,
    rating: crate::attributes::Attr,
    minutes: f64,
    years: u32,
) -> Vec<(f64, f64)> {
    use crate::development::{SeasonContext, develop_season};
    let Some(i) = super::folk::person(w, player) else { return Vec::new() };
    let p = &w.people[i];
    let level = match p.stage {
        super::Stage::HighSchool | super::Stage::PrepSchool => crate::player::Level::HS,
        super::Stage::College => p
            .college
            .and_then(|c| w.programs.get(c))
            .map_or(crate::player::Level::FBS, |g| super::division_level(g.division)),
        _ => crate::player::Level::NFL,
    };
    let (teaching, _) = room(w, p.player.position);
    let role = p.last_role.unwrap_or(crate::player::PlayingTime::Backup);
    let arm = |focus: bool| -> Vec<f64> {
        let mut q = p.player.clone();
        let mut rng = crate::Rng::new(player ^ (w.year as u32).wrapping_mul(0x9E37_79B9) ^ 0x0F0C);
        (0..years)
            .map(|_| {
                let mut ctx = SeasonContext::new(level, teaching.clamp(1.0, 20.0), role);
                ctx.seat_plan = true;
                if focus {
                    ctx.focus = Some(rating);
                    ctx.focus_minutes = minutes;
                }
                develop_season(&mut q, &ctx, &mut rng);
                q.attributes.get(rating)
            })
            .collect()
    };
    arm(false).into_iter().zip(arm(true)).collect()
}
