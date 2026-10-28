//! V9 at a college or a high school: your players' draft outlook. At a
//! college, your players three years or more out of high school (and next
//! year's); at a high school, your former players in college. Each with
//! where he stands (declared, deciding, back for another year, drafted),
//! the advisory committee's grade sent to his school, what the NFL scouts
//! who came through say, the public's rank and best mock, and your staff's
//! read against what it took to go in last year's rounds.

use afm_engine::calendar::{Date, YearDates};
use afm_engine::world::eligibility::{division_of, exhausted_after};
use afm_engine::world::scouting::public;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::Reply;
use crate::lane_squadboards::{any_read, mid, own_read};
use crate::reads::Read;
use crate::views::{self, TeamRef, team_brief};

/// A round in words, as scouts and coaches say it.
fn round_talk(r: u32) -> &'static str {
    match r {
        1 => "Round 1",
        2..=3 => "Day two",
        4..=7 => "Day three",
        _ => "Free agent",
    }
}

fn range(r: Read) -> Value {
    match r {
        Read::Range(lo, hi) => json!([lo, hi]),
        Read::NotSeen => Value::Null,
    }
}

pub fn view(w: &World) -> Reply {
    let me = match views::mine(w) {
        Some(t @ (TeamRef::College(_) | TeamRef::HighSchool(_))) => t,
        _ => return Err("The draft outlook is for a college or a high school seat.".into()),
    };
    let college = matches!(me, TeamRef::College(_));
    let dy = public::draft_year(w);
    // The draft last held: this April once it's done, else last year's.
    let last = if w.today > w.dates.draft[2].date { w.year } else { w.year - 1 };
    let deadline = YearDates::new(dy - 1).declare_deadline;
    let draft_day = YearDates::new(dy).draft[0].date;
    // The scouts' December reads and the committee's grades (from 16
    // December) are this cycle's from the day they come out to the draft;
    // until then they are last year's and stay off the page.
    let before_draft = w.today <= w.dates.draft[2].date;
    let reads_out = before_draft || w.today > w.dates.fbs_regular_end;
    let advisory_out = before_draft || w.today >= Date::ymd(w.year, 12, 16);
    let consensus = public::consensus(w).unwrap_or_default();
    let best_mock = &w.scouting.public.best_mock;
    let bars = w.draft_bars();
    let ours = |p: &afm_engine::world::Person| match me {
        TeamRef::College(c) => p.college == Some(c),
        TeamRef::HighSchool(h) => p.hs == h,
        _ => false,
    };
    let mut rows: Vec<(u32, f64, Value)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if !ours(p) {
            continue;
        }
        let id = p.player.id;
        let drafted = p.pro.as_ref().and_then(|c| c.draft).filter(|d| d.0 == last);
        let undrafted = p.stage == Stage::Pro
            && p.pro.as_ref().is_some_and(|c| c.draft.is_none() && c.first_year == last)
            && p.class_of + 3 <= last;
        let eligible = matches!(p.stage, Stage::College | Stage::Declared) && p.class_of + 3 <= dy + 1;
        if drafted.is_none() && !undrafted && !eligible {
            continue;
        }
        let school = p.college.map(|c| w.program_name(c).to_string());
        let (status, group) = if let Some(d) = drafted {
            (format!("Drafted {}: round {}, pick {}", d.0, d.1, d.2), 0)
        } else if undrafted {
            (format!("Undrafted in {last}"), 0)
        } else if p.stage == Stage::Declared {
            ("Declared".to_string(), 1)
        } else if p.class_of + 3 > dy {
            (format!("Eligible in {}", dy + 1), 3)
        } else if w.today > deadline {
            ("Back for another year".to_string(), 2)
        } else if exhausted_after(p, division_of(p, &w.programs), dy - 1) {
            ("Final season: enters the draft after it".to_string(), 1)
        } else {
            (format!("Eligible: decides by {}", views::date_text(deadline)), 1)
        };
        // What his school hears and what's public.
        let in_class = group != 0;
        let this_cycle = matches!(group, 1 | 2);
        let read_now = p.cal.draft_read.filter(|_| this_cycle && reads_out);
        // The committee grades underclassmen who ask (a round-8 read only if
        // he's thinking of going), so an older grade never shows.
        let asked = read_now.is_some_and(|r| r.round <= 7 || p.cal.declare_plan == Some(true));
        let advisory = if college && advisory_out && asked && !exhausted_after(p, division_of(p, &w.programs), dy - 1) {
            p.cal.advisory
        } else {
            None
        };
        let scouts = read_now.map(|d| d.round);
        let pub_rank = if this_cycle { consensus.iter().position(|&x| x == id).map(|x| x + 1) } else { None };
        let mock = if this_cycle { best_mock.get(&id).copied() } else { None };
        // Your staff's read: your own player's (narrow) at a college, the
        // public's read (wide) of a former player from a high school.
        let read = if college && p.stage == Stage::College { own_read(w, me, i) } else { any_read(w, i).0 };
        let staff = match (mid(read), bars) {
            (Some(m), Some((b12, b34))) if in_class => Some(if m >= b12 {
                "Grades like a round 1-2 pick"
            } else if m >= b34 {
                "Grades like a round 3-4 pick"
            } else {
                "Grades below last year's round 4"
            }),
            _ => None,
        };
        let outlook = match (drafted, advisory, scouts, pub_rank) {
            (Some(d), ..) => round_talk(d.1).to_string(),
            _ if undrafted => "Free agent".into(),
            (_, Some(a), ..) if a >= 8 => "Return to school".into(),
            (_, Some(a), ..) => round_talk(a as u32).into(),
            (_, _, Some(r), _) => round_talk(r).into(),
            (_, _, _, Some(_)) => "Round 1".into(),
            _ => "Too early to say".into(),
        };
        // Sorted: last draft by pick, then the class by the best word.
        let key = drafted.map_or(0.0, |d| d.2 as f64)
            + if undrafted { 999.0 } else { 0.0 }
            + if in_class {
                pub_rank.map_or(0.0, |r| r as f64)
                    + advisory.map_or(scouts.map_or(400.0, |r| r as f64 * 40.0), |a| a as f64 * 40.0)
            } else {
                0.0
            };
        let draft = drafted
            .map(|d| json!({ "year": d.0, "round": d.1, "pick": d.2, "team": team_brief(w, TeamRef::Nfl(d.3)) }));
        rows.push((
            group,
            key,
            json!({
                "id": id, "name": p.name, "pos": p.player.position.name(),
                "school": school, "class_of": p.class_of,
                "class": if p.stage == Stage::College { format!("{} {} played", p.college_seasons, afm_engine::world::folk::noun(p.college_seasons, "season")) } else { String::new() },
                "status": status, "group": group, "outlook": outlook, "draft": draft,
                "advisory": advisory.map(|a| if a >= 8 { "Return to school".to_string() } else { format!("Round {a}") }),
                "scouts": scouts.map(|r| {
                    let (lo, hi) = (r.saturating_sub(1).max(1), (r + 1).min(8));
                    if r >= 8 { "Free agent".to_string() } else if hi >= 8 { format!("Round {lo} or later") } else { format!("Rounds {lo}-{hi}") }
                }),
                "public": pub_rank, "mock": mock, "read": range(read), "staff": staff,
                "team": views::team_of(w, p).map(|t| team_brief(w, t)),
            }),
        ));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let n = |g: u32| rows.iter().filter(|r| r.0 == g).count();
    let counts = json!({ "drafted": n(0), "class": n(1), "back": n(2), "next": n(3) });
    Ok(json!({
        "seat": if college { "college" } else { "hs" },
        "team": team_brief(w, me),
        "draft_year": dy,
        "last": last,
        "deadline": views::date_text(deadline),
        "draft_day": views::date_text(draft_day),
        "bars": bars.map(|b| [b.0.round(), b.1.round()]),
        "counts": counts,
        "rows": rows.into_iter().map(|r| r.2).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::round_talk;

    #[test]
    fn rounds_read_as_days() {
        assert_eq!(round_talk(1), "Round 1");
        assert_eq!(round_talk(3), "Day two");
        assert_eq!(round_talk(7), "Day three");
        assert_eq!(round_talk(8), "Free agent");
    }
}
