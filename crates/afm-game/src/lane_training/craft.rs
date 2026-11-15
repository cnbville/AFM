//! The Development tab's flaws and tools (individual practice topics 9,
//! 9b and 9c): his points of emphasis with their stage and tape, what the
//! staff has seen, the new skills he could learn, and the projection
//! before you pick one. Every number is your staff's read through its
//! eye: the gains, the weeks and the ratings are ranges, and a flaw the
//! staff named wrong is projected as the staff believes it.

use afm_engine::attributes::Attr;
use afm_engine::craft::{self, Craft, FULL, Kind, Point, Stage, Teach};
use afm_engine::world::World;
use afm_engine::world::craft_work::{self as cw, Tag};
use serde_json::{Value, json};

use crate::reads;
use crate::views::{TeamRef, date_text};

/// Up to this many weeks the projection looks ahead (three years).
const LOOK_WEEKS: u32 = 156;

/// A fixed lean in -1..1 for this player and key (the staff's own misread).
fn lean(reader: u64, id: u32, k: u64) -> f64 {
    let mut z = reader ^ (id as u64) << 22 ^ k.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xC7AF;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// How well his position coach spots flaws, in a word.
fn eye_words(eye: f64) -> &'static str {
    if eye >= 15.0 {
        "sharp"
    } else if eye >= 11.0 {
        "good"
    } else if eye >= 7.0 {
        "fair"
    } else {
        "weak"
    }
}

/// How sure the staff is of a flaw: its eye and the reps it has seen.
fn sure_words(eye: f64, shows: u32) -> &'static str {
    let s = eye / 20.0 + (shows as f64 / 30.0).min(1.0) * 0.5;
    if s >= 1.0 {
        "sure"
    } else if s >= 0.7 {
        "fairly sure"
    } else if s >= 0.45 {
        "not sure yet"
    } else {
        "only a guess"
    }
}

/// Is he on board (topic 8.1, in plain words)?
fn buy_in_words(b: f64) -> &'static str {
    if b >= 0.85 {
        "fully"
    } else if b >= 0.65 {
        "mostly"
    } else if b >= 0.45 {
        "half"
    } else {
        "not really"
    }
}

/// The size the staff believes a flaw or tool is (0-1): the truth when
/// he carries it, a believed one when the staff named it wrong.
fn believed(w: &World, i: usize, c: Craft, reader: u64) -> f64 {
    let p = &w.people[i].player;
    match p.craft.get(c) {
        Some(it) => it.sev(),
        _ if c.is_flaw() => 0.5 + 0.2 * lean(reader, p.id, 900 + c as u64),
        _ => 0.0,
    }
}

/// A gain in rating points in its own situation, as the staff reads it
/// (the middle of its range and the range).
fn gain_read(truth: f64, eye: f64, reader: u64, id: u32, k: u64) -> (f64, [i64; 2]) {
    let half = 0.5 + (20.0 - eye.clamp(1.0, 20.0)) / 12.0;
    let mid = (truth + lean(reader, id, k) * half * 0.6).max(0.0);
    (mid, [(mid - half).max(0.0).round() as i64, (mid + half).round() as i64])
}

/// The point's label: a flaw's or tool's name, or a rating's.
pub(super) fn point_label(pt: Point) -> String {
    match pt {
        Point::Craft(c) => c.name().to_string(),
        Point::Rating(a) => format!("{}, all round", a.label()),
    }
}

/// The points he works: yours, else the staff's (named flaws, else the
/// staff's rating focus), with who set them.
pub(super) fn points_now(w: &World, id: u32) -> (Vec<Point>, &'static str) {
    let mine = w.points_of(id);
    if !mine.is_empty() {
        return (mine, "you");
    }
    let staff: Vec<Point> = w.staff_points_of(id).into_iter().map(Point::Craft).collect();
    if !staff.is_empty() {
        return (staff, "staff");
    }
    (w.staff_focus(id).map(Point::Rating).into_iter().collect(), "staff")
}

/// One tape-book row for the screen.
fn tag_json(t: &Tag) -> Value {
    json!({ "date": date_text(t.day), "d": t.day.0, "shows": t.shows, "lost": t.lost })
}

/// The minutes one point gets: his week's (the film room's for a mental
/// one), split over his points.
fn point_minutes(w: &World, id: u32, c: Option<Craft>, n: usize) -> f64 {
    let (field, film) = w.minutes_of(id);
    let m = if c.is_some_and(|c| c.info().mental) { film } else { field };
    m / n.max(1) as f64
}

/// The flaws and tools part of the Development tab.
pub(super) fn tab(w: &World, t: TeamRef, i: usize, eye: f64) -> Value {
    let reader = crate::views::reader_key(w);
    let p = &w.people[i];
    let id = p.player.id;
    let pos = p.player.position;
    let (points, by) = points_now(w, id);
    let named = w.seen_of(id);
    let tags: Vec<&Tag> = w.tags_of(id).collect();
    let notes: Vec<&afm_engine::training::seat::Note> =
        w.training.notes.iter().rev().filter(|n| n.id == id && n.date <= w.today).collect();
    let (field, film) = w.minutes_of(id);
    let buy = craft::buy_in(&p.player, p.trust as f64);
    let teacher = super::teacher(w, t, i);
    let pts: Vec<Value> = points
        .iter()
        .enumerate()
        .map(|(k, &pt)| {
            let c = pt.craft();
            let mins = point_minutes(w, id, c, points.len());
            let mut v = json!({
                "n": k + 1,
                "code": pt.code(),
                "kind": match c { Some(c) if c.is_flaw() => "flaw", Some(_) => "tool", None => "rating" },
                "name": point_label(pt),
                "rating": pt.rating().label(),
                "by": by,
                "minutes": mins.round(),
            });
            if let Some(c) = c {
                let it = p.player.craft.get(c);
                let st = it.map_or(Stage::Learning, |it| craft::stage(c, it));
                let mine: Vec<&&Tag> = tags.iter().filter(|t| t.code == c as u8).collect();
                let games: Vec<Value> = mine.iter().filter(|t| !t.practice).map(|t| tag_json(t)).collect();
                let practice: Vec<Value> = mine.iter().filter(|t| t.practice).map(|t| tag_json(t)).collect();
                let sum = |v: &[&&Tag]| v.iter().fold((0u32, 0u32), |a, t| (a.0 + t.shows as u32, a.1 + t.lost as u32));
                let g: Vec<&&Tag> = mine.iter().filter(|t| !t.practice).copied().collect();
                let first = sum(&g[..g.len().min(4)]);
                let last = sum(&g[g.len().saturating_sub(4)..]);
                let what = c.name().to_lowercase();
                let note = notes.iter().find(|n| n.text.to_lowercase().contains(&what));
                v["what"] = json!(c.info().what);
                v["tell"] = json!(c.info().tell);
                v["stage"] = json!(st as u8);
                v["stage_words"] = json!(st.words());
                v["weeks"] = json!(it.map_or(0, |it| it.weeks()));
                v["games"] = json!(games);
                v["practice"] = json!(practice);
                v["before"] = json!({ "shows": first.0, "lost": first.1 });
                v["now"] = json!({ "shows": last.0, "lost": last.1 });
                v["game_rows"] = json!(g.len());
                v["note"] = note.map_or(Value::Null, |n| json!({ "date": date_text(n.date), "text": n.text }));
                v["since"] = json!(named.iter().find(|x| x.0 == c).map(|x| date_text(x.1)));
            }
            v
        })
        .collect();
    // What the staff has seen (topic 4): flaws and tools it named.
    let since = w.today.add(-56);
    let seen: Vec<Value> = named
        .iter()
        .map(|&(c, day)| {
            let mine: Vec<&&Tag> = tags.iter().filter(|t| t.code == c as u8).collect();
            let shows: u32 = mine.iter().map(|t| t.shows as u32).sum();
            // His games lately: the reps in its situation, and those lost.
            let late = mine.iter().filter(|t| !t.practice && t.day.days_to(since) <= 0);
            let (g_shows, lost) = late.fold((0u32, 0u32), |a, t| (a.0 + t.shows as u32, a.1 + t.lost as u32));
            let held = p.player.craft.get(c).map(|it| craft::stage(c, it));
            let (mid, range) = gain_read(FULL * believed(w, i, c, reader), eye, reader, id, c as u64);
            json!({
                "code": Point::Craft(c).code(),
                "kind": if c.is_flaw() { "flaw" } else { "tool" },
                "name": c.name(),
                "what": c.info().what,
                "tell": c.info().tell,
                "rating": c.owner().label(),
                "gain": mid.round(),
                "gain_range": range,
                "named": date_text(day),
                "shows": g_shows,
                "lost": lost,
                "stage": held.map(|st| st as u8),
                "stage_words": held.map(|st| st.words()),
                "sure": sure_words(eye, shows),
                "on": points.contains(&Point::Craft(c)),
            })
        })
        .collect();
    // Losses on tape the staff can't put a name to yet (topic 4.4).
    let unexplained: u32 = tags
        .iter()
        .filter(|t| t.day.days_to(since) <= 0 && !named.iter().any(|x| x.0 as u8 == t.code))
        .filter(|t| Craft::from_index(t.code as usize).is_some_and(|c| c.is_flaw()))
        .map(|t| t.lost as u32)
        .sum();
    // New skills he could learn (topic 2's tools at his spot, not yet his):
    // the staff's read of what each would add, biggest first.
    let mut tools: Vec<(f64, Value)> = Craft::for_position(pos)
        .filter(|c| c.info().kind == Kind::Tool && !named.iter().any(|x| x.0 == *c))
        .filter(|&c| p.player.craft.get(c).is_none_or(|it| craft::stage(c, it) != Stage::Games))
        .map(|c| {
            let have = p.player.craft.sev(c);
            let (mid, range) = gain_read(FULL * (0.6 - have).max(0.1), eye, reader, id, 300 + c as u64);
            let w8 = afm_engine::positions::weight(pos, c.owner());
            (
                mid * (0.5 + w8),
                json!({
                    "code": Point::Craft(c).code(),
                    "kind": "tool",
                    "name": c.name(),
                    "what": c.info().what,
                    "rating": c.owner().label(),
                    "gain": mid.round(),
                    "gain_range": range,
                    "started": have > 0.0,
                    "on": points.contains(&Point::Craft(c)),
                }),
            )
        })
        .collect();
    tools.sort_by(|a, b| b.0.total_cmp(&a.0));
    json!({
        "by": by,
        "set": w.points_of(id).len(),
        "points": pts,
        "seen": seen,
        "unexplained": unexplained,
        "tools": tools.into_iter().take(4).map(|x| x.1).collect::<Vec<_>>(),
        "eye": eye_words(eye),
        "teacher": teacher,
        "minutes": { "field": field.round(), "film": film.round() },
        "today": w.today.0,
        "buy_in": buy_in_words(buy),
        "ratings": super::focus_options(pos),
    })
}

/// The seasons he has left with you, this one included (None for a pro).
fn seasons_left(w: &World, i: usize) -> Option<u32> {
    use afm_engine::world::Stage as S;
    let p = &w.people[i];
    match p.stage {
        S::College => {
            use afm_engine::world::eligibility;
            Some(eligibility::seasons_left(p, eligibility::division_of(p, &w.programs), w.year).max(1))
        }
        S::HighSchool | S::PrepSchool => Some(((p.class_of - w.year).max(0) as u32 + 1).min(4)),
        _ => None,
    }
}

/// The projection before you pick (topic 9c): what this point gets him and
/// when, against his clock, through the staff's eye.
pub(super) fn project(w: &World, i: usize, eye: f64, code: u8) -> Result<Value, String> {
    let pt = Point::from_code(code).ok_or("No such focus.")?;
    let reader = crate::views::reader_key(w);
    let p = &w.people[i];
    let id = p.player.id;
    let pos = p.player.position;
    if let Some(c) = pt.craft()
        && !c.fits(pos)
    {
        return Err("That isn't a skill at his position.".into());
    }
    let rating = pt.rating();
    let attrs = reads::attributes(w, i);
    let read = attrs.iter().find(|x| x.0 == rating).map(|x| x.1).unwrap_or(reads::Read::NotSeen);
    let (points, _) = points_now(w, id);
    let n = if points.contains(&pt) { points.len() } else { (points.len() + 1).min(2) };
    let mins = point_minutes(w, id, pt.craft(), n);
    let half_eye = 0.15 + (20.0 - eye.clamp(1.0, 20.0)) / 40.0;
    let (teaching, _) = cw::room(w, pos);
    let buy = craft::buy_in(&p.player, p.trust as f64);
    let mut v = json!({
        "code": code,
        "name": point_label(pt),
        "rating": rating.label(),
        "kind": match pt.craft() { Some(c) if c.is_flaw() => "flaw", Some(_) => "tool", None => "rating" },
        "minutes": mins.round(),
        "buy_in": buy_in_words(buy),
        "on": points.contains(&pt),
    });
    // His skill in that situation now and once it holds (topic 9c.2).
    if let (Some(c), reads::Read::Range(lo, hi)) = (pt.craft(), read) {
        let size = FULL * believed(w, i, c, reader);
        let (gain, _) = gain_read(
            if c.is_flaw() { size } else { FULL * (0.6 - p.player.craft.sev(c)).max(0.1) },
            eye,
            reader,
            id,
            if c.is_flaw() { c as u64 } else { 300 + c as u64 },
        );
        let (now_lo, now_hi) = if c.is_flaw() {
            (lo as f64 - gain, hi as f64 - gain)
        } else {
            (lo as f64 + FULL * p.player.craft.sev(c), hi as f64 + FULL * p.player.craft.sev(c))
        };
        let fixed = if c.is_flaw() { (lo as f64, hi as f64) } else { (now_lo + gain, now_hi + gain) };
        v["situation"] = json!({
            "now": [now_lo.round().max(1.0), now_hi.round().min(99.0)],
            "fixed": [fixed.0.round().max(1.0), fixed.1.round().min(99.0)],
            "tell": c.info().tell,
        });
        // When it holds in games (topic 9c.4), on a copy of him as the
        // staff believes him (a wrong name is projected as if it were his).
        let mut q = p.player.clone();
        if c.is_flaw() && !q.craft.has(c) {
            q.craft.set(c, believed(w, i, c, reader));
        }
        let weeks = |m: f64| craft::weeks_to_hold(&q, c, &Teach { teaching, minutes: m, buy_in: buy }, LOOK_WEEKS);
        let left = seasons_left(w, i);
        let at = |wk: u32| w.today.add(7 * wk as i32);
        let hold = weeks(mins).map(|wk| {
            let lo = ((wk as f64) * (1.0 - half_eye)).round().max(0.0) as u32;
            let hi = ((wk as f64) * (1.0 + half_eye)).round() as u32;
            // The seasons of payoff after it holds: a season's games run
            // from September to December.
            let pay = left.map(|l| {
                (0..l)
                    .filter(|&k| {
                        let y = w.year + k as i32 + if w.today.month() >= 12 || w.today.month() <= 1 { 1 } else { 0 };
                        afm_engine::calendar::Date::ymd(y, 10, 15) >= at(wk)
                    })
                    .count()
            });
            json!({ "weeks": wk, "weeks_range": [lo, hi], "from": date_text(at(lo)), "to": date_text(at(hi)), "payoff": pay })
        });
        let more = weeks(mins + 15.0);
        v["hold"] = hold.unwrap_or(Value::Null);
        v["sooner"] = json!(match (weeks(mins), more) {
            (Some(a), Some(b)) if a > b => Some(a - b),
            _ => None,
        });
        v["seasons_left"] = json!(left);
        v["sure"] = json!(sure_words(eye, w.tags_of(id).filter(|t| t.code == c as u8).map(|t| t.shows as u32).sum()));
        v["mine"] = json!(!c.is_flaw() || p.player.craft.has(c));
    }
    // The rating it grows, by the year's end, with it and without (topic
    // 5b.6), and by his last season (9c.5), grown on copies of him.
    let years = seasons_left(w, i).unwrap_or(2).clamp(1, 4);
    let grown = cw::focus_projection(w, id, rating, mins, years);
    // Each read moves the staff's own read of him now by what the copies
    // grew, so the projection and today's range never disagree.
    let truth = p.player.attributes.get(rating);
    let wide = |x: f64, k: f64| -> Value {
        let grow = (2.0 + (20.0 - eye.clamp(1.0, 20.0)) / 5.0) * (k - 1.0);
        match read {
            reads::Read::Range(lo, hi) => {
                let d = x - truth;
                json!([
                    (lo as f64 + d - grow).round().clamp(1.0, 99.0),
                    (hi as f64 + d + grow).round().clamp(1.0, 99.0)
                ])
            }
            reads::Read::NotSeen => Value::Null,
        }
    };
    if let Some(&(without, with)) = grown.first() {
        v["year_end"] =
            json!({ "without": wide(without, 1.0), "with": wide(with, 1.0), "gap": (with - without).round() });
    }
    if let Some(&(without, with)) = grown.last() {
        v["last_season"] = json!({ "years": years, "without": wide(without, 1.5), "with": wide(with, 1.5) });
    }
    v["now"] = read.json();
    v["eye"] = json!(eye_words(eye));
    Ok(v)
}

/// Sets his points of emphasis from the screen's codes (topic 5): at most
/// two, each a rating, or a flaw or tool at his spot. Returns the flaw he
/// turned down, if any.
pub(super) fn set(w: &mut World, id: u32, codes: &[u8]) -> Result<Option<String>, String> {
    let i = crate::views::person_index(w, id).ok_or("No such player.")?;
    let pos = w.people[i].player.position;
    let mut pts = Vec::new();
    for &c in codes.iter().take(2) {
        let pt = Point::from_code(c).ok_or("No such focus.")?;
        if let Point::Craft(k) = pt
            && !k.fits(pos)
        {
            return Err("That isn't a skill at his position.".into());
        }
        if let Point::Rating(a) = pt
            && !Attr::ALL.contains(&a)
        {
            return Err("No such skill.".into());
        }
        pts.push(pt);
    }
    Ok(w.set_points(id, &pts).map(|c| c.name().to_string()))
}
