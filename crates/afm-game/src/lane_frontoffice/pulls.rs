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

use crate::views::{self, TeamRef};
use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::background::Decider;
use afm_engine::world::folk::claims::{self, Ear, Group, Kind, Point, Sheet, Tier, words};
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

/// A draft round as it reads in a line: "second round", or "a late pick".
fn round_line(r: &str) -> String {
    if r == "late" { "a late pick".to_string() } else { format!("{r} round") }
}

/// A spot's name in the plural ("tackles", "safeties").
fn plural(spot: &str) -> String {
    match spot.strip_suffix('y') {
        Some(stem) => format!("{stem}ies"),
        None => format!("{spot}s"),
    }
}

/// One of your own facts as a line for the file: what your school can say
/// to him on a point, in words and with no money figure (a number of
/// dollars is yours to say in a talk, not a line on his file). None for a
/// fact that has nothing to say plainly.
pub(crate) fn yours_words(p: &Point, spot: &str) -> Option<String> {
    let n = |h: &str| p.get(h).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    let t = |h: &str| p.get(h).unwrap_or("").to_string();
    let many = |c: i64, one: &str, more: &str| if c == 1 { format!("1 {one}") } else { format!("{c} {more}") };
    Some(match p.kind {
        Kind::Room => {
            let (stay, leave) = (n("staying"), n("leaving"));
            let all = stay + leave;
            let depth = match p.check("depth") {
                Some("thin") => "thin, so room to play early",
                Some("crowded") => "crowded, a long line ahead of him",
                _ => "a normal room",
            };
            let who = if all == 1 { spot.to_string() } else { plural(spot) };
            format!("{all} {who} on your roster now, {leave} leaving after this season: {depth}")
        }
        Kind::Role => match p.check("role") {
            Some("starter") => format!("You plan him as your starting {spot}"),
            Some("rotation") => format!("You plan him in the rotation at {spot}"),
            _ => format!("You plan him as a backup at {spot}"),
        },
        Kind::QbRivals => {
            if n("rivals") == 0 {
                "No blue-chip quarterback ahead of him".to_string()
            } else {
                format!(
                    "{} ahead of him, {} ({})",
                    many(n("rivals"), "blue-chip quarterback", "blue-chip quarterbacks"),
                    t("rival"),
                    t("rival_class")
                )
            }
        }
        Kind::Scheme => format!(
            "A {} at {spot}{}",
            t("system"),
            if p.check("new") == Some("yes") { ", new to your staff" } else { "" }
        ),
        Kind::OfferRate => match p.check("gap") {
            Some("over") => format!("Your offer is over what you usually pay a {spot}"),
            Some("under") => format!("Your offer is under what you usually pay a {spot}"),
            _ => format!("Your offer is about what you usually pay a {spot}"),
        },
        Kind::FamilyNeed => match p.check("need") {
            Some("high") => "His family needs the money, and your offer helps".to_string(),
            _ => "Your offer would help his family".to_string(),
        },
        Kind::Brand => "He has a following, and your program is seen enough to build on it".to_string(),
        Kind::Degree => match p.check("academics") {
            Some("top") => format!("{}: a top academic school", t("school")),
            Some("research") => format!("{}: a research university", t("school")),
            Some("solid") => format!("{}: solid academics", t("school")),
            _ => format!("{}: a plain academic name", t("school")),
        },
        Kind::Support => "Your academic support, with his grades near the line".to_string(),
        Kind::AfterFootball => format!("A degree and a life in {} after football", t("town")),
        Kind::Drive => {
            format!("{} miles from home, about {}", n("miles"), many(n("hours"), "hour's drive", "hours' drive"))
        }
        Kind::HomeState => format!("In his home state, {}", t("state")),
        Kind::ChildhoodTeam => format!("The team he grew up with: {}", t("school")),
        Kind::HsMates => format!("{} from {} already here: {}", n("mates"), t("hs"), t("names")),
        Kind::Brother => "His older brother is here".to_string(),
        Kind::Tenure => match p.check("seat") {
            Some("hot") => format!("Your seat is hot after {}", t("seasons")),
            Some("new") => format!("A new staff: {}", t("seasons")),
            Some("long") => format!("A settled staff: {}", t("seasons")),
            _ => format!("Your head coach, {}", t("seasons")),
        },
        Kind::PositionCoach => {
            let best =
                p.get("best").map(|b| format!(", best {b} ({})", round_line(&t("best_round")))).unwrap_or_default();
            let drafted =
                if n("picks") == 0 { "none drafted yet".to_string() } else { format!("{} drafted", n("picks")) };
            format!("{}, your {}: {drafted}{best}", t("pc"), t("title"))
        }
        Kind::HsCoachTie => format!("A pipeline from {}", t("hs")),
        Kind::YourWord => match p.check("record") {
            Some("clean") => format!("Your word: {} promises kept, none broken", n("kept")),
            _ => format!("Your word: {} kept, {} broken", n("kept"), n("broken")),
        },
        Kind::PicksAtPosition => {
            format!(
                "{} at his spot drafted in ten years; best {} ({}, {})",
                t("picks_words"),
                t("best"),
                round_line(&t("best_round")),
                t("best_club")
            )
        }
        Kind::PicksOverall => {
            format!(
                "{} drafted in the last three years{}",
                t("picks_words"),
                if n("first_round") > 0 { format!(", {} in round 1", n("first_round")) } else { String::new() }
            )
        }
        Kind::YourGuys => format!("{} drafted who played for your head coach: {}", n("guys"), t("names")),
        Kind::PositionU => format!("Your group is known as {}", t("u")),
        Kind::ProDay => format!("{} clubs at your pro day, {} sent their top people", n("clubs"), n("heads")),
        Kind::Titles => {
            let last = p.get("last_record").map(|r| format!("; last year {r}")).unwrap_or_default();
            format!("{} national and {} conference titles{last}", n("titles"), n("conference_titles"))
        }
        Kind::TitleOdds => match p.check("level") {
            Some("contender") => "A title contender this year".to_string(),
            Some("in_the_hunt") => "In the hunt this year".to_string(),
            _ => "A long shot for a title this year".to_string(),
        },
        Kind::Stadium => format!("A {} seat stadium in {}", n("seats"), t("town")),
        Kind::Exposure => {
            if p.check("power") == Some("yes") { "A power conference name" } else { "Less national exposure" }
                .to_string()
        }
        Kind::Conference => format!("Plays in the {}", t("conference")),
        Kind::Rival => format!("{} against {}, {}-{} in the series", t("game"), t("rival"), n("won"), n("lost")),
        Kind::Buildings => format!(
            "{}, {}",
            t("building"),
            if p.check("stage") == Some("open") { "open".to_string() } else { format!("opens {}", n("opens")) }
        ),
        _ => return None,
    })
}

/// The best-known player at his spot who is still on your roster when he
/// arrives: a blue-chip (4 stars or more) by name, class and stars, the
/// things a family can see and never a rating. "A young star ahead of
/// him" is the reason a recruit at that spot looks elsewhere. With none,
/// the line says so.
pub(crate) fn ahead_line(w: &World, i: usize, j: usize) -> Option<String> {
    use afm_engine::world::Stage;
    use afm_engine::world::folk::facts;
    let pos = w.people[i].player.position;
    let best = w
        .programs
        .get(j)?
        .roster
        .iter()
        .copied()
        .filter(|&k| {
            let q = &w.people[k];
            k != i
                && q.player.position == pos
                && q.stage == Stage::College
                && q.cal.portal.is_none()
                && !facts::leaving(w, k)
        })
        .max_by_key(|&k| {
            (crate::lane_find::fields::rstars(&w.people[k]), std::cmp::Reverse(w.people[k].college_seasons))
        })?;
    let q = &w.people[best];
    let stars = crate::lane_find::fields::rstars(q);
    Some(if stars >= 4 {
        format!("Ahead of him: {}, {}, {stars} stars, still here when he arrives", q.name, facts::class_words(w, best))
    } else {
        "No blue-chip at his spot who will still be here when he arrives".to_string()
    })
}

/// What your school can say on a group, from your sheet for him: your best
/// fact with words, and how strong your proof is next to his other top
/// schools (`Tier`). None when you have nothing on it yet.
fn yours(sheet: &Sheet, g: Group, spot: &str) -> Option<Value> {
    let mut pts: Vec<&Point> = sheet.group(g).filter(|p| p.known && p.proof.is_some()).collect();
    pts.sort_by(|a, b| {
        let (x, y) = (a.proof.unwrap(), b.proof.unwrap());
        y.tier.cmp(&x.tier).then(y.strength.total_cmp(&x.strength))
    });
    pts.into_iter().find_map(|p| {
        let text = yours_words(p, spot)?;
        let tier = p.proof?.tier;
        let word = match tier {
            Tier::Elite => "best on his list",
            Tier::Strong => "strong",
            Tier::Fair => "fair",
            Tier::Weak => "weak",
        };
        Some(json!({ "text": text, "tier": tier.key(), "tier_word": word }))
    })
}

/// Your fact for a group with its extra line from the roster: the extra
/// line alone still shows when your sheet has nothing to say yet.
fn yours_with(y: Option<Value>, also: Option<String>) -> Value {
    match (y, also) {
        (Some(mut y), Some(a)) => {
            y["also"] = json!(a);
            y
        }
        (Some(y), None) => y,
        (None, Some(a)) => json!({ "text": a, "tier": "", "tier_word": "" }),
        (None, None) => Value::Null,
    }
}

/// A group's own extra line from your roster: at Playing time, who is ahead
/// of him at his spot.
fn also(w: &World, i: usize, g: Group, prog: Option<usize>) -> Option<String> {
    (g == Group::PlayingTime).then(|| ahead_line(w, i, prog?)).flatten()
}

/// "What pulls him" for the file's screen: the seven groups in the file's
/// order, each with a word for how it reads and the rows behind it (what
/// he did or said, where and when, and whose read when it was a coach of
/// yours); the lines where his words and your read part; your area
/// recruiter's free guess; who decides as far as you know, and whose wants
/// the rows speak for (`for`: him, or whoever decides at his home as far
/// as you know). No number of his.
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
    // Your school's own side of each group, for a college seat.
    let spot = words::spot(p.player.position);
    let prog = match views::mine(w) {
        Some(TeamRef::College(j)) => Some(j),
        _ => None,
    };
    let sheet = prog.map(|j| claims::recruit(w, i, j, if family { Ear::Decider } else { Ear::Himself }));
    let groups: Vec<Value> = reads
        .iter()
        .map(|r| {
            let firm = r.rows >= FIRM_ROWS && !r.conflict;
            json!({
                "key": r.group.key(), "name": r.group.name(),
                "word": group_word(r, Some(r.group) == lead),
                "firm": firm, "conflict": r.conflict,
                "rows": rows_of(r.group),
                "yours": yours_with(sheet.as_ref().and_then(|s| yours(s, r.group, spot)), also(w, i, r.group, prog)),
            })
        })
        .collect();
    let told = w.folk.pulls.decides(id).map(|(_, from, on)| format!("{}, {}", from.words(), day(on, today)));
    let transfer = p.cal.portal.is_some() || p.stage != afm_engine::world::Stage::HighSchool;
    json!({
        "groups": groups,
        "flags": pulls::flags(&reads),
        "empty": reads.iter().all(|r| r.rows == 0),
        "for": match (family, decider(w, i)) {
            (false, _) => "him",
            (true, Some(Decider::Parent)) => "his parents",
            (true, Some(Decider::Grandparent)) => "his grandparents",
            (true, Some(Decider::Handler)) => "the trainer around him",
            (true, _) => "his family",
        },
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
    fn your_school_says_its_facts_in_words() {
        assert_eq!(plural("safety"), "safeties");
        assert_eq!(plural("tackle"), "tackles");
        let room = Point::new(Kind::Room).count("staying", 4).count("leaving", 2).state("depth", "crowded");
        assert_eq!(
            yours_words(&room, "tackle").as_deref(),
            Some("6 tackles on your roster now, 2 leaving after this season: crowded, a long line ahead of him")
        );
        let picks = Point::new(Kind::PicksAtPosition)
            .text("picks_words", "3 picks")
            .text("best", "Marcus Hale")
            .text("best_round", "second")
            .text("best_club", "Bears");
        assert_eq!(
            yours_words(&picks, "tackle").as_deref(),
            Some("3 picks at his spot drafted in ten years; best Marcus Hale (second round, Bears)")
        );
        // Money never shows a figure: the offer's size is yours to say in a talk.
        let offer = Point::new(Kind::OfferRate).text("offer", "$400k").state("gap", "under");
        let line = yours_words(&offer, "tackle").unwrap();
        assert!(!line.contains('$') && line.contains("under"), "{line}");
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
