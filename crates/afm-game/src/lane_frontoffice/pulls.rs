//! "What pulls him" on a recruit's or a transfer's file (talk facts design,
//! section 7; lane L7). The head coach finds out by talking to him, and
//! the file keeps what was seen: what he brought up, how he took each
//! point, what he told you when you asked, what your read caught, and your
//! area recruiter's free guess before you spoke. Every reader here reads
//! that file (`afm_engine::world::folk::pulls`), never his true wants: the
//! file's section, the one line on where he leans ("why you're behind"),
//! who decides at his home, and the race page's weights.
//!
//! Every size here is Claude's (proposed, not confirmed).

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::background::Decider;
use afm_engine::world::folk::claims::Group;
use afm_engine::world::folk::pulls::{self, Read};
use serde_json::{Value, json};

/// The race page's seven reasons, in its order (`lane_find::race`,
/// `REASONS`), as the file's groups.
pub(crate) const RACE_GROUPS: [Group; 7] =
    [Group::Home, Group::PlayingTime, Group::Money, Group::League, Group::Winning, Group::School, Group::People];

/// The race page's weight for a reason the file has nothing on: the whole
/// range, 1 to 5 around 3.
pub(crate) const WEIGHT_HALF_NONE: f64 = 2.0;
/// Each row on the file narrows it this much ...
pub(crate) const WEIGHT_HALF_ROW: f64 = 0.5;
/// ... down to this.
pub(crate) const WEIGHT_HALF_MIN: f64 = 0.5;
/// His words and your read disagree: the range stays this much wider.
pub(crate) const WEIGHT_HALF_CONFLICT: f64 = 0.5;
/// The file's score that puts a reason at either end (5, or 1 below).
pub(crate) const WEIGHT_SCORE_END: f64 = 2.0;
/// A group the file reads this high says "matters most" when it leads
/// (his top two told, or as much in what he asked and how he took it).
pub(crate) const FIRM_SCORE: f32 = 2.0;
/// The rows that make a read firm, when they agree.
pub(crate) const FIRM_ROWS: u8 = 3;
/// Rows shown for one group on the file, newest first.
pub(crate) const ROWS_SHOWN: usize = 6;

/// His file as it reads today: the order of the seven groups with the free
/// guess counted in (`pulls::file`).
pub(crate) fn file(w: &World, i: usize) -> Vec<Read> {
    pulls::file(w, i)
}

/// Whether the file puts a group among what pulls him: in its first two,
/// with something for it.
pub(crate) fn leans(file: &[Read], g: Group) -> bool {
    file.iter().take(2).any(|r| r.group == g && r.rows > 0 && r.score > 0.0)
}

/// Who decides at his home, as far as you know: what he or his family told
/// you when you asked, else the truth once your staff has been in touch
/// (as "Who decides" on Find recruits and the race page read it), else
/// nothing yet.
pub(crate) fn decider(w: &World, i: usize) -> Option<Decider> {
    let p = &w.people[i];
    if let Some((code, _, _)) = w.folk.pulls.decides(p.player.id) {
        return Some(pulls::decider_of(code));
    }
    crate::lane_find::staff::contacted(w, p.player.id)
        .then_some(afm_engine::world::front_office::college::family_read(w, i).decider)
}

/// "He", or whoever decides for him when you know it.
pub(crate) fn subject(w: &World, i: usize) -> &'static str {
    match decider(w, i) {
        Some(Decider::Parent) => "His parents",
        Some(Decider::Grandparent) => "His grandparents",
        Some(Decider::Handler) => "The trainer around him",
        _ => "He",
    }
}

/// The file's line on who decides.
pub(crate) fn decides_words(w: &World, i: usize) -> &'static str {
    match decider(w, i) {
        Some(Decider::Himself) => "He decides for himself.",
        Some(Decider::Parent) => "A parent has the final say.",
        Some(Decider::Grandparent) => "His grandparents raised him; they decide with him.",
        Some(Decider::Handler) => "A trainer close to the family steers it.",
        None => "Not known yet: your staff hasn't been in touch.",
    }
}

/// The file's summary in one line: where his words and your read part,
/// else his top two as the file reads them. None while nothing speaks.
pub(crate) fn summary(file: &[Read], first: &str) -> Option<String> {
    if let Some(f) = pulls::flags(file).into_iter().next() {
        return Some(f);
    }
    let seen: Vec<&str> =
        file.iter().filter(|r| r.rows > 0 && r.score > 0.0).take(2).map(|r| pulls::word(r.group)).collect();
    match seen.as_slice() {
        [a, b] => Some(format!("From what you've seen, {first} seems after {a} first, then {b}.")),
        [a] => Some(format!("From what you've seen, {first} seems after {a} most.")),
        _ => None,
    }
}

/// How the file reads one group, in words.
fn group_word(r: &Read, lead: bool) -> &'static str {
    if r.rows == 0 {
        "Nothing yet"
    } else if r.under {
        "More than he lets on"
    } else if r.over {
        "Says so; your read doubts it"
    } else if r.score >= FIRM_SCORE && lead {
        "Matters most"
    } else if r.score >= FIRM_SCORE {
        "Matters a lot"
    } else if r.score > 0.0 {
        "Seems to matter"
    } else if r.score < 0.0 {
        "Not his thing"
    } else {
        "Hard to say"
    }
}

/// A row's day: "3 Mar", with the year when it isn't this one.
fn day(d: Date, today: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, dd) = d.parts();
    let mon = M[(m as usize).clamp(1, 12) - 1];
    if y == today.parts().0 { format!("{dd} {mon}") } else { format!("{dd} {mon} {y}") }
}

/// "What pulls him" for the file's screen: the seven groups in the file's
/// order, each with a word for how it reads and the rows behind it (what
/// he did or said, where and when, and whose read when it was a coach of
/// yours); the lines where his words and your read part; your area
/// recruiter's free guess; who decides as far as you know, and whose wants
/// the rows speak for. No number of his.
pub(crate) fn section(w: &World, i: usize) -> Value {
    let p = &w.people[i];
    let id = p.player.id;
    let today = w.today;
    let family = pulls::decider_rows(w, i);
    let reads = file(w, i);
    let guess = pulls::guess(w, i);
    let rows_of = |g: Group| -> Vec<Value> {
        let mut v: Vec<Value> = w
            .folk
            .pulls
            .of(id)
            .filter(|r| r.ev != pulls::Ev::Decides && r.decider == family && r.group() == g)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(ROWS_SHOWN)
            .map(|r| {
                let by = r.by.and_then(|c| w.coaches.get(c)).map(|c| c.name.clone());
                let from = match &by {
                    Some(n) => format!("{n}, {}", day(r.on, today)),
                    None => format!("{}, {}", r.from.words(), day(r.on, today)),
                };
                json!({ "text": r.says(), "from": from, "staff": by.is_some() })
            })
            .collect();
        if let Some((c, gg)) = guess
            && gg == g
        {
            let name = w.coaches.get(c).map_or_else(|| "Your area recruiter".to_string(), |c| c.name.clone());
            v.push(json!({ "text": format!("Guessed {} pulls him most", pulls::word(g)), "from": format!("{name}, before you spoke"), "staff": true, "guess": true }));
        }
        v
    };
    let lead = reads.first().map(|r| r.group);
    let groups: Vec<Value> = reads
        .iter()
        .map(|r| {
            let firm = r.rows >= FIRM_ROWS && !r.conflict;
            json!({
                "key": r.group.key(), "name": r.group.name(),
                "word": group_word(r, Some(r.group) == lead),
                "firm": firm, "conflict": r.conflict,
                "rows": rows_of(r.group),
            })
        })
        .collect();
    let told = w.folk.pulls.decides(id).map(|(_, from, on)| format!("{}, {}", from.words(), day(on, today)));
    let transfer = p.cal.portal.is_some() || p.stage != afm_engine::world::Stage::HighSchool;
    json!({
        "groups": groups,
        "flags": pulls::flags(&reads),
        "empty": reads.iter().all(|r| r.rows == 0),
        "for": if family { "his family" } else { "him" },
        "decides": if transfer { Value::Null } else { json!({ "words": decides_words(w, i), "told": told }) },
    })
}

/// The race page's weights from his file: a range on 1 to 5 for each of
/// its seven reasons, the whole range where the file has nothing, narrower
/// with each row, wider where his words and your read part, around where
/// the file's score puts it. The file's score, never his own wants.
pub(crate) fn race_weights(w: &World, i: usize) -> Vec<[u8; 2]> {
    let reads = file(w, i);
    RACE_GROUPS.iter().map(|&g| weight_of(reads.iter().find(|r| r.group == g && r.rows > 0))).collect()
}

/// One reason's range from the file's read of its group (None: nothing on
/// the file).
fn weight_of(r: Option<&Read>) -> [u8; 2] {
    let Some(r) = r.filter(|r| r.rows > 0) else {
        return [1, 5];
    };
    let centre = 3.0 + (r.score as f64).clamp(-WEIGHT_SCORE_END, WEIGHT_SCORE_END) * 2.0 / WEIGHT_SCORE_END;
    let mut half = (WEIGHT_HALF_NONE - WEIGHT_HALF_ROW * r.rows as f64).max(WEIGHT_HALF_MIN);
    if r.conflict {
        half += WEIGHT_HALF_CONFLICT;
    }
    let lo = (centre - half).round().clamp(1.0, 5.0) as u8;
    let hi = (centre + half).round().clamp(1.0, 5.0) as u8;
    [lo, hi.max(lo)]
}

/// The newest row behind a race reason, in words with where and when: the
/// first of the race page's sources.
pub(crate) fn race_source(w: &World, i: usize, k: usize) -> Option<String> {
    let g = *RACE_GROUPS.get(k)?;
    let p = &w.people[i];
    let family = pulls::decider_rows(w, i);
    w.folk
        .pulls
        .of(p.player.id)
        .filter(|r| r.ev != pulls::Ev::Decides && r.decider == family && r.group() == g)
        .last()
        .map(|r| format!("{} ({}, {})", r.says(), r.from.words(), day(r.on, w.today)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(group: Group, score: f32, rows: u8, conflict: bool) -> Read {
        Read { group, score, rows, conflict, under: false, over: false, talked: rows > 0 }
    }

    #[test]
    fn the_race_weights_come_from_the_file() {
        // Nothing on the file: the whole range.
        assert_eq!(weight_of(None), [1, 5]);
        assert_eq!(weight_of(Some(&read(Group::Money, 0.0, 0, false))), [1, 5]);
        // Told it matters most (one row): high, still open.
        assert_eq!(weight_of(Some(&read(Group::Money, 2.0, 1, false))), [4, 5]);
        // Asked first, took it well, told you: firm at the top.
        assert_eq!(weight_of(Some(&read(Group::Money, 4.0, 3, false))), [5, 5]);
        // Said it isn't his thing: low.
        assert_eq!(weight_of(Some(&read(Group::Money, -1.0, 1, false))), [1, 4]);
        // Only the free guess: a lean, the range barely narrower.
        assert_eq!(weight_of(Some(&read(Group::Money, 0.75, 1, false))), [2, 5]);
        // His words and your read part: wider than the same rows agreeing.
        let a = weight_of(Some(&read(Group::Money, 1.0, 3, false)));
        let b = weight_of(Some(&read(Group::Money, 1.0, 3, true)));
        assert!(b[1] - b[0] > a[1] - a[0], "{a:?} {b:?}");
    }

    #[test]
    fn the_lean_line_reads_the_file() {
        let mut f: Vec<Read> = Group::ALL.iter().map(|&g| read(g, 0.0, 0, false)).collect();
        assert!(summary(&f, "Jalen").is_none());
        assert!(!leans(&f, Group::Money));
        f[0] = read(Group::League, 3.0, 2, false);
        f[1] = read(Group::Money, 1.0, 1, false);
        assert_eq!(
            summary(&f, "Jalen").as_deref(),
            Some("From what you've seen, Jalen seems after development first, then money.")
        );
        assert!(leans(&f, Group::League) && leans(&f, Group::Money) && !leans(&f, Group::Home));
        // Where his words and your read part, that leads.
        f[1].under = true;
        f[1].conflict = true;
        assert_eq!(summary(&f, "Jalen").as_deref(), Some("Says development, lights up at money."));
        assert_eq!(group_word(&f[0], true), "Matters most");
        assert_eq!(group_word(&f[1], false), "More than he lets on");
        assert_eq!(group_word(&f[2], false), "Nothing yet");
    }
}
