//! Front office: the high school seat (lane 10; people topic 12, first
//! playable topic 7). Your school's office (the principal, the athletic
//! director, the superintendent and the booster club, the year's goals,
//! the boosters' books, your teaching job and your hours), your squads
//! (varsity, JV and freshman ball, with the JV and freshman games), and
//! the school and its feeder area (the town, the feeders and the
//! freshmen coming up, what you've given each feeder, the athletes in your
//! building, the school's football history). `school_player` is one of
//! your players' School tab: his team, his grade and his eligibility.

use afm_engine::calendar::data::{GradeCheck, hs_calendar};
use afm_engine::world::World;
use afm_engine::world::front_office::school::{self, PROJECTS, SchoolPerson, Squad, WANTS};
use afm_engine::world::front_office::{Duty, Owner, dollars, index_of};
use afm_engine::world::hs_year::HsStat;
use afm_engine::world::scouting::own;
use afm_engine::world::{Person, Stage};
use serde_json::{Value, json};

use crate::views::{TeamRef, date_text, team_brief};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "school_office" => game.w().and_then(office),
        "school_squads" => game.w().and_then(squads),
        "school_area" => game.w().and_then(area),
        "school_player" => game.w().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
            player(w, id)
        }),
        "set_squad" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
            let to = args.get("squad").and_then(Value::as_str).and_then(Squad::from_key).ok_or("No such team.")?;
            let text = school::set_squad(w, id, to)?;
            let mut v = squads(w)?;
            v["text"] = json!(text);
            Ok(v)
        }),
        "reset_squads" => game.wm().and_then(|w| {
            school::reset_squads(w);
            squads(w)
        }),
        "squads_owner" => game.wm().and_then(|w| {
            school::your_school(w).ok_or("You don't coach a high school.")?;
            let staff = args.get("owner").and_then(Value::as_str) == Some("staff");
            w.front_office.set_owner(Duty::Squads, if staff { Owner::Staff } else { Owner::You });
            if staff {
                school::reset_squads(w);
            }
            squads(w)
        }),
        "squad_suggestion" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
            let take = args.get("take").and_then(Value::as_bool).unwrap_or(true);
            let s = w.front_office.school.suggestions.iter().find(|s| s.id == id).cloned().ok_or("That's done.")?;
            let text = if take {
                school::set_squad(w, id, s.to)?
            } else {
                w.front_office.school.suggestions.retain(|x| x.id != id);
                "Left where he is.".to_string()
            };
            let mut v = squads(w)?;
            v["text"] = json!(text);
            Ok(v)
        }),
        "booster_wish" => game.wm().and_then(|w| {
            school::your_school(w).ok_or("You don't coach a high school.")?;
            let order: Vec<u8> = args
                .get("order")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_u64)
                        .filter(|&k| (k as usize) < PROJECTS.len())
                        .map(|k| k as u8)
                        .collect()
                })
                .unwrap_or_default();
            let mut wish: Vec<u8> = Vec::new();
            for k in order.into_iter().chain(0..PROJECTS.len() as u8) {
                if !wish.contains(&k) {
                    wish.push(k);
                }
            }
            w.front_office.school.boosters.wish = wish;
            office(w)
        }),
        _ => return None,
    })
}

fn mine(w: &World) -> Result<usize, String> {
    if !w.seat_head() {
        return Err("You don't coach a high school.".into());
    }
    school::your_school(w).ok_or_else(|| "You don't coach a high school.".to_string())
}

// ---------------------------------------------------------------------------
// The school's office
// ---------------------------------------------------------------------------

/// What a person weighs most, in words: his two biggest wants.
fn wants_words(p: &SchoolPerson) -> String {
    let mut k: Vec<usize> = (0..4).collect();
    k.sort_by(|&a, &b| p.wants[b].total_cmp(&p.wants[a]));
    let say = |i: usize| match WANTS[i] {
        "wins" => "winning",
        "grades" => "grades",
        "numbers" => "kids coming out for football",
        _ => "behavior",
    };
    if p.wants[k[0]] >= 0.55 {
        format!("Mostly {}", say(k[0]))
    } else {
        format!("{} and {}", capital(say(k[0])), say(k[1]))
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn patience_words(p: f32) -> &'static str {
    match p {
        p if p >= 0.6 => "Patient",
        p if p >= 0.4 => "Gives a coach time",
        _ => "Wants results now",
    }
}

fn person(w: &World, key: &str, p: &SchoolPerson) -> Value {
    use afm_engine::world::folk::{
        Team,
        cast::{CastRef, Role},
    };
    // His page in the people screens (`people_cast`).
    let role = match key {
        "principal" => Role::Principal,
        "ad" => Role::SchoolAd,
        "superintendent" => Role::Superintendent,
        _ => Role::ClubPresident,
    };
    let tok =
        (p.cast != 0).then(|| school::your_school(w)).flatten().map(|h| CastRef::new(Team::High(h), role, 0).token());
    let child = p
        .child
        .and_then(|id| index_of(w, id))
        .map(|i| json!({ "id": w.people[i].player.id, "name": w.people[i].name }));
    json!({
        "key": key, "name": p.name, "title": p.title, "age": p.age,
        "since": p.since, "years": (w.year - p.since).max(0), "about": p.about,
        "wants": wants_words(p), "patience": patience_words(p.patience),
        "trust": school::trust_words(school::trust_of(w, p)), "child": child, "she": p.she,
        "tok": tok,
    })
}

fn office(w: &World) -> Result<Value, String> {
    let h = mine(w)?;
    let d = &w.front_office.school;
    // The principal's goals from the start-of-year meeting, then the
    // athletic director's and the booster club's.
    let (season, set) = school::principal_goals(w);
    let mut goals: Vec<Value> = set
        .iter()
        .map(|g| {
            let (prog, ok) = school::principal_goal_now(w, h, g);
            let mut text = g.kind.words(afm_engine::calendar::Level::HighSchool);
            text[..1].make_ascii_uppercase();
            json!({
                "text": text, "by": format!("{}, principal", d.principal.name),
                "prog": prog, "on_track": ok, "met": g.met,
                "deal": match g.deal { d if d < 0 => "you argued it down", 0 => "", _ => "you raised it" },
            })
        })
        .collect();
    goals.extend(d.goals.iter().filter(|_| d.goals_year == season || set.is_empty()).map(|g| {
        let (now, ok) = school::goal_now(w, h, g);
        let by = match g.by {
            0 => format!("{}, principal", d.principal.name),
            1 => format!("{}, athletic director", d.ad.name),
            _ => "The booster club".to_string(),
        };
        json!({
            "text": school::goal_text(g), "by": by,
            "prog": school::goal_progress(g, now), "on_track": ok, "met": g.met, "deal": "",
        })
    }));
    let b = &d.boosters;
    const IN: [&str; 5] = ["Membership", "Golf tournament", "Ads and banners", "Concessions", "The banquet"];
    const OUT: [&str; 4] = ["Equipment", "Meals", "Travel", "Projects"];
    let year = w.today.year();
    let projects: Vec<Value> = b
        .wish
        .iter()
        .enumerate()
        .map(|(rank, &k)| {
            let (name, cost, _) = PROJECTS[k as usize];
            let done = b.done.iter().find(|x| x.1 == k).map(|x| x.0);
            json!({ "k": k, "rank": rank + 1, "name": name, "cost": cost * 1.03f64.powi(year - 2026), "done": done })
        })
        .collect();
    let t = &d.teaching;
    let (teach, coach, what) = school::hours(w, w.today);
    let reviews: Vec<Value> =
        d.reviews.iter().rev().map(|r| json!({ "year": r.year, "met": r.met, "of": r.of, "words": r.words })).collect();
    // The principal is your boss: a meeting you ask for waits a month
    // after the last one (`folk::boss::request_meeting`).
    let can_meet = !w.folk.scenes.iter().any(|s| {
        s.kind == afm_engine::world::folk::scene::Kind::BossMeeting && s.c == 3 && s.date.days_to(w.today) < 30
    });
    Ok(json!({
        "school": team_brief(w, TeamRef::HighSchool(h)),
        "can_meet": can_meet,
        "mascot": d.mascot,
        "year": if set.is_empty() { d.goals_year } else { season },
        "people": [
            person(w, "principal", &d.principal),
            person(w, "ad", &d.ad),
            person(w, "superintendent", &d.superintendent),
            person(w, "boosters", &b.president),
        ],
        "goals": goals,
        "boosters": {
            "members": b.members,
            "balance": b.balance,
            "raised": IN
                .iter()
                .zip(b.raised)
                .chain(std::iter::once((&"Team fundraisers", b.fundraised)))
                .map(|(n, x)| json!({ "what": n, "x": x }))
                .collect::<Vec<_>>(),
            "spent": OUT.iter().zip(b.spent).map(|(n, x)| json!({ "what": n, "x": x })).collect::<Vec<_>>(),
            "ledger": b.ledger.iter().rev().take(40).map(|(dt, t, x)| json!({ "date": date_text(*dt), "what": t, "x": x })).collect::<Vec<_>>(),
            "projects": projects,
        },
        "teaching": {
            "subject": t.subject, "periods": t.periods, "of_periods": t.of_periods,
            "coordinator": t.coordinator, "salary": t.salary, "stipend": t.stipend,
            "total": t.salary + t.stipend,
            "line": if t.coordinator {
                format!("Campus athletic coordinator: {} of {} periods are athletics, no classroom of your own.", t.periods, t.of_periods)
            } else {
                format!("You teach {} {} periods a day of {}; the rest is your planning period and athletics.", t.periods, t.subject.to_lowercase(), t.of_periods)
            },
        },
        "hours": { "teach": teach, "coach": coach, "total": teach + coach, "what": what },
        "reviews": reviews,
        "money": dollars(b.balance),
    }))
}

// ---------------------------------------------------------------------------
// Squads
// ---------------------------------------------------------------------------

/// A JV or freshman season line in words.
fn line_words(s: &[i32]) -> String {
    let g = |k: HsStat| s[k as usize];
    let mut parts = Vec::new();
    if g(HsStat::Att) > 0 {
        parts.push(format!(
            "{}-{}, {} yds, {} TD, {} INT",
            g(HsStat::Cmp),
            g(HsStat::Att),
            g(HsStat::PassYds),
            g(HsStat::PassTd),
            g(HsStat::Int)
        ));
    }
    if g(HsStat::Rush) > 0 {
        parts.push(format!("{} car, {} yds, {} TD", g(HsStat::Rush), g(HsStat::RushYds), g(HsStat::RushTd)));
    }
    if g(HsStat::Rec) > 0 {
        parts.push(format!("{} rec, {} yds, {} TD", g(HsStat::Rec), g(HsStat::RecYds), g(HsStat::RecTd)));
    }
    if g(HsStat::Tackles) > 0 {
        let sacks = g(HsStat::Sacks) as f64 / 2.0;
        parts.push(format!(
            "{} tkl{}{}{}",
            g(HsStat::Tackles),
            if g(HsStat::Tfl) > 0 { format!(", {:.1} TFL", g(HsStat::Tfl) as f64 / 2.0) } else { String::new() },
            if sacks > 0.0 { format!(", {sacks:.1} sk") } else { String::new() },
            if g(HsStat::Ints) > 0 { format!(", {} INT", g(HsStat::Ints)) } else { String::new() },
        ));
    }
    if g(HsStat::FgAtt) > 0 || g(HsStat::XpMade) > 0 {
        parts.push(format!("{}-{} FG, {} XP", g(HsStat::FgMade), g(HsStat::FgAtt), g(HsStat::XpMade)));
    }
    parts.join(" · ")
}

/// The teams a player on `sq` can move to: varsity always, JV if the school
/// fields one and he isn't a senior, the freshman team only for a freshman
/// (`school::set_squad` holds the same rules).
fn can_move(sq: Squad, grade: u32, jv: bool, fr: bool) -> Vec<&'static str> {
    Squad::ALL
        .into_iter()
        .filter(|&to| {
            to != sq
                && match to {
                    Squad::Varsity => true,
                    Squad::Jv => jv && grade < 12,
                    Squad::Freshman => fr && grade == 9,
                }
        })
        .map(Squad::key)
        .collect()
}

fn squads(w: &World) -> Result<Value, String> {
    let h = mine(w)?;
    let t = TeamRef::HighSchool(h);
    let (jv, fr) = school::fields(w, h);
    let d = &w.front_office.school;
    let reader = crate::views::reader_key(w);
    let list = school::squads(w);
    let staff = school::staff_squads(w, h);
    let by_staff = |i: usize| staff.iter().find(|x| x.0 == i).map(|x| x.1);
    let line_of = |id: u32| d.lines.iter().find(|l| l.id == id);
    let mut teams: Vec<Value> = Vec::new();
    for sq in Squad::ALL {
        if (sq == Squad::Jv && !jv) || (sq == Squad::Freshman && !fr) {
            continue;
        }
        let mut rows: Vec<(usize, Value)> = list
            .iter()
            .filter(|x| x.1 == sq)
            .map(|&(i, _)| {
                let p = &w.people[i];
                let mut v = crate::views::row(w, i, t, true, reader);
                let grade = school::grade(w, p.class_of);
                v["grade"] = json!(grade);
                v["moved"] = json!(by_staff(i).is_some_and(|s| s != sq));
                v["staff"] = json!(by_staff(i).map(|s| s.name()));
                v["can"] = json!(can_move(sq, grade, jv, fr));
                v["line"] = json!(
                    line_of(p.player.id)
                        .filter(|l| l.squad == sq)
                        .map(|l| json!({ "games": l.games, "text": line_words(&l.s) }))
                );
                (p.player.position.index(), v)
            })
            .collect();
        rows.sort_by_key(|x| x.0);
        let games: Vec<Value> = d
            .games
            .iter()
            .filter(|g| g.squad == sq)
            .map(|g| {
                let top: Vec<Value> = g
                    .top
                    .iter()
                    .map(|(id, text)| {
                        let name = index_of(w, *id).map(|i| w.people[i].name.clone()).unwrap_or_default();
                        json!({ "id": id, "name": name, "text": text })
                    })
                    .collect();
                json!({
                    "date": date_text(g.date), "opp": team_brief(w, TeamRef::HighSchool(g.opp)), "home": g.home,
                    "score": g.score, "won": g.score.map(|s| s[0] > s[1]), "top": top, "note": g.note,
                })
            })
            .collect();
        let (wn, l) = d
            .games
            .iter()
            .filter(|g| g.squad == sq)
            .filter_map(|g| g.score)
            .fold((0, 0), |a, s| (a.0 + (s[0] > s[1]) as u32, a.1 + (s[0] < s[1]) as u32));
        teams.push(json!({
            "key": sq.key(), "name": sq.name(), "count": rows.len(),
            "record": if sq == Squad::Varsity { None } else { Some(format!("{wn}-{l}")) },
            "players": rows.into_iter().map(|x| x.1).collect::<Vec<_>>(),
            "games": games,
        }));
    }
    let suggestions: Vec<Value> = d
        .suggestions
        .iter()
        .filter_map(|s| {
            let i = index_of(w, s.id)?;
            let p = &w.people[i];
            (p.stage == Stage::HighSchool).then(|| {
                json!({ "id": s.id, "name": p.name, "pos": p.player.position.name(), "to": s.to.name(), "why": s.why, "date": date_text(s.date) })
            })
        })
        .collect();
    let owner = w.front_office.owner(Duty::Squads);
    let s = &w.high_schools[h];
    // What the school fields this year: a team short of 22 to dress is folded into the one above.
    let has = |sq: Squad| list.iter().any(|x| x.1 == sq);
    let fields_line = match (jv, fr, has(Squad::Jv), has(Squad::Freshman)) {
        (true, true, true, true) => format!("{} students: a varsity, a JV and a freshman team.", s.enrollment),
        (true, true, true, false) => {
            format!(
                "{} students: a varsity and a JV. Too few freshmen to dress a freshman team, so they play JV.",
                s.enrollment
            )
        }
        (true, _, false, _) => format!(
            "{} students: one team this year. Too few players to dress a JV (22), so everyone is on varsity.",
            s.enrollment
        ),
        (true, false, true, _) => format!("{} students: a varsity and a JV team. Freshmen play JV.", s.enrollment),
        _ => format!("{} students: one team. Everyone plays varsity, as at most small schools.", s.enrollment),
    };
    Ok(json!({
        "school": team_brief(w, t),
        "jv": jv, "freshman": fr,
        "fields": fields_line,
        "owner": if owner == Owner::You { "you" } else { "staff" },
        "staff_does": Duty::Squads.staff_does(),
        "set_on": d.set_on.map(date_text),
        "moves": d.moves.len(),
        "teams": teams,
        "suggestions": suggestions,
        "rules": "Seniors can't play JV, and only freshmen play on the freshman team. JV and freshman games are on Thursdays, at the other team's field when the varsity is home, with 10-minute quarters. A player who plays in a JV game that week can still dress for Friday.",
    }))
}

/// A high school grade in words.
fn grade_words(g: u32) -> &'static str {
    match g {
        9 => "Freshman",
        10 => "Sophomore",
        11 => "Junior",
        _ => "Senior",
    }
}

/// How his state checks grades for eligibility (A29), in words.
fn check_words(c: GradeCheck) -> &'static str {
    match c {
        GradeCheck::SixWeek => "No pass, no play: a failing grade on a six-week report card sits him about three weeks",
        GradeCheck::Weekly => "Grades are checked every week: a failing week sits him the next",
        GradeCheck::NineWeek => "Grades are checked each grading period, about every nine weeks",
        GradeCheck::Semester => "Last semester's grades decide this one",
    }
}

/// One of your players' School tab (screens topic 5, tab 8): the team he
/// plays on and the staff's list he was on, his grade, what keeps him
/// eligible (his grades under the state's check, a transfer's sit-out, a
/// suspension), his JV or freshman line, and the teams he can move to.
fn player(w: &World, id: u32) -> Result<Value, String> {
    let h = mine(w)?;
    let i = index_of(w, id)
        .filter(|i| w.high_schools[h].roster.contains(i) && w.people[*i].stage == Stage::HighSchool)
        .ok_or("He isn't on your roster.")?;
    let p = &w.people[i];
    let sq = school::squads(w).into_iter().find(|x| x.0 == i).map_or(Squad::Varsity, |x| x.1);
    let staff = school::staff_squads(w, h).into_iter().find(|x| x.0 == i).map(|x| x.1);
    let (jv, fr) = school::fields(w, h);
    let grade = school::grade(w, p.class_of);
    let st = &w.states[w.high_schools[h].state];
    let out = p.cal.hs_grade_out;
    // His transcript's GPA, as a college recruiter reads it too.
    let gpa = afm_engine::world::front_office::college::family_read(w, i).gpa;
    let mut eligibility = vec![json!({
        "what": "Grades",
        "words": if p.cal.academic_sit {
            "Academically ineligible".to_string()
        } else if out > 0 {
            format!("Ruled out of {out} {} for grades this season", afm_engine::world::folk::noun(out, "game"))
        } else {
            "Eligible".to_string()
        },
        "ok": !p.cal.academic_sit && out == 0,
        "note": format!("GPA {gpa:.2}. {}.", check_words(hs_calendar(&st.code).grades)),
    })];
    if p.sit_out > 0.0 || p.cal.sitting_transfer {
        eligibility.push(json!({
            "what": "Transfer",
            "words": if p.sit_out > 0.0 { format!("Sits {:.0}% of the varsity season", p.sit_out * 100.0) } else { "Sitting out as a transfer".to_string() },
            "ok": false,
            "note": "The state's rule for a transfer without a family move.",
        }));
    }
    if p.cal.trouble {
        eligibility.push(json!({ "what": "Conduct", "words": "Suspended", "ok": false, "note": "" }));
    }
    let line = w
        .front_office
        .school
        .lines
        .iter()
        .find(|l| l.id == id && l.squad == sq && sq != Squad::Varsity)
        .map(|l| json!({ "games": l.games, "text": line_words(&l.s) }));
    Ok(json!({
        "id": id,
        "name": p.name,
        "school": team_brief(w, TeamRef::HighSchool(h)),
        "squad": sq.key(),
        "squad_name": sq.name(),
        "staff": staff.map(|s| s.name()),
        "moved": staff.is_some_and(|s| s != sq),
        "can": can_move(sq, grade, jv, fr),
        "owner": if w.front_office.owner(Duty::Squads) == Owner::You { "you" } else { "staff" },
        "grade": grade,
        "grade_words": grade_words(grade),
        "class_of": p.class_of,
        "seasons_left": (p.class_of - w.year).max(0),
        "eligibility": eligibility,
        "line": line,
        "rules": "Seniors can't play JV, and only freshmen play on the freshman team.",
    }))
}

// ---------------------------------------------------------------------------
// The school and its feeder area
// ---------------------------------------------------------------------------

fn level_words(x: f64, words: [&'static str; 5]) -> &'static str {
    words[((x.clamp(0.0, 0.999)) * 5.0) as usize]
}

/// How much of your system a feeder runs, in words (half or more counts
/// toward the L14 chase).
fn adopt_words(a: f32) -> &'static str {
    match a {
        a if a >= 0.5 => "Runs your system",
        a if a >= 0.25 => "Runs some of your system",
        a if a > 0.0 => "Has started on your system",
        _ => "Runs his own system",
    }
}

fn area(w: &World) -> Result<Value, String> {
    let h = mine(w)?;
    let s = &w.high_schools[h];
    let st = &w.states[s.state];
    let feeders: Vec<Value> = school::feeders(w, h)
        .iter()
        .enumerate()
        .map(|(k, f)| {
            let from: Vec<Value> = s
                .roster
                .iter()
                .filter(|&&i| w.people[i].stage == Stage::HighSchool && !f.youth && school::feeder_of(w, h, i) == Some(k))
                .take(80)
                .map(|&i| json!({ "id": w.people[i].player.id, "name": w.people[i].name, "pos": w.people[i].player.position.name(), "grade": school::grade(w, w.people[i].class_of) }))
                .collect();
            let eighth = (f.boys as f64 * if f.youth { 0.5 } else { 0.48 }).round();
            // What you've given a feeder middle school (your system's chase,
            // L14); a youth league takes none of it.
            let plan = w.legacy.hs.feeders.iter().find(|x| x.name == f.name);
            let help = (!f.youth).then(|| {
                json!({
                    "playbook": plan.is_some_and(|x| x.playbook), "clinic": plan.is_some_and(|x| x.clinic),
                    "assistant": plan.is_some_and(|x| x.assistant), "equipment": plan.is_some_and(|x| x.equipment),
                    "runs": plan.map_or("Runs his own system", |x| adopt_words(x.adopt)),
                })
            });
            json!({
                "name": f.name, "youth": f.youth, "coach": f.coach, "tok": f.tok, "boys": f.boys,
                "share": (f.share * 100.0).round(), "last": f.last,
                "next": if f.youth { format!("About {eighth} seventh and eighth graders play youth tackle") } else { format!("About {eighth} eighth graders: next fall's freshmen") },
                "players": from,
                "help": help,
            })
        })
        .collect();
    // The athletes in your building who don't play football (SK22), as the
    // evaluation desk lists them, to talk to from here too.
    let athletes: Vec<Value> = own::athletes(w)
        .into_iter()
        .map(|a| {
            json!({
                "key": a.key, "name": a.name, "sport": own::SPORTS[a.sport].0, "pos": a.pos.name(),
                "class_of": a.class_of, "tools": [a.tools.0, a.tools.1], "spotted_by": a.spotted_by,
            })
        })
        .collect();
    let came_in = s
        .roster
        .iter()
        .filter(|&&i| w.people[i].stage == Stage::HighSchool && school::feeder_of(w, h, i).is_none())
        .count();
    let seasons: Vec<Value> = s
        .seasons
        .iter()
        .rev()
        .map(|x| {
            json!({
                "year": x.year, "record": format!("{}-{}", x.wins, x.losses),
                "coach": x.coach.and_then(|c| w.coaches.get(c)).map(|c| c.name.clone()),
                "coach_id": x.coach, "title": x.state_title,
                "points": if x.points[0] + x.points[1] > 0 { Some(format!("{}-{}", x.points[0], x.points[1])) } else { None },
            })
        })
        .collect();
    let mut coaches: Vec<(usize, i32, i32, u32, u32)> = Vec::new();
    for x in &s.seasons {
        let Some(c) = x.coach else { continue };
        match coaches.iter_mut().find(|k| k.0 == c) {
            Some(k) => {
                k.2 = x.year;
                k.3 += x.wins;
                k.4 += x.losses;
            }
            None => coaches.push((c, x.year, x.year, x.wins, x.losses)),
        }
    }
    // Everyone from the school still playing: the NFL first, then the
    // draft pool, then college by division, the newest class first.
    let mut playing: Vec<(u8, i32, &Person)> = w
        .people
        .iter()
        .filter(|p| p.hs == h && matches!(p.stage, Stage::College | Stage::Pro | Stage::Declared))
        .map(|p| {
            let rank = match p.stage {
                Stage::Pro if p.pro.as_ref().is_some_and(|c| c.team.is_some()) => 0,
                Stage::Pro => 1,
                Stage::Declared => 2,
                _ => 3 + p.college.map_or(6, |c| w.programs[c].division as u8),
            };
            (rank, p.class_of, p)
        })
        .collect();
    playing.sort_by_key(|&(r, c, p)| (r, -c, p.name.clone()));
    let alumni: Vec<Value> = playing
        .iter()
        .map(|&(_, _, p)| {
            let level = match p.stage {
                Stage::Pro if p.pro.as_ref().is_some_and(|c| c.team.is_some()) => "NFL".to_string(),
                Stage::Pro => "Free agent".to_string(),
                Stage::Declared => "Draft pool".to_string(),
                _ => p.college.map_or("College".into(), |c| w.programs[c].division.name().to_string()),
            };
            json!({
                "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "class": p.class_of,
                "level": level, "where": crate::views::where_now(w, p),
                "team": crate::views::team_of(w, p).map(|t| team_brief(w, t)),
            })
        })
        .collect();
    let (wins, losses) = s.seasons.iter().fold((0, 0), |a, x| (a.0 + x.wins, a.1 + x.losses));
    let bracket = st.schools.iter().filter(|&&k| w.high_schools[k].classification == s.classification).count();
    Ok(json!({
        "school": team_brief(w, TeamRef::HighSchool(h)),
        "town": s.town,
        "state": crate::views::state_name(&st.code),
        "enrollment": s.enrollment,
        "class": crate::views::hs_class(w, h),
        "line": crate::views::team_line(w, TeamRef::HighSchool(h)),
        "private": s.private,
        "mascot": w.front_office.school.mascot,
        "prestige": crate::views::prestige_words(s.prestige),
        "resources": level_words(s.resources, ["Bare bones", "Thin", "Decent", "Well equipped", "Among the best in the state"]),
        "money": level_words(s.area_money, ["A poor area", "Working class", "Middle class", "Comfortable", "Wealthy"]),
        "safety": level_words(s.area_safety, ["Rough", "Some trouble", "Mostly quiet", "Safe", "Very safe"]),
        "district": format!("{bracket} school{} in the {} {} playoff bracket", if bracket == 1 { "" } else { "s" }, st.code, crate::views::hs_class(w, h)),
        "transfer": st.transfer.sit_out,
        "feeders": feeders,
        "boosters": w.front_office.school.boosters.balance,
        "athletes": athletes,
        "came_in": came_in,
        "seasons": seasons,
        "record": format!("{wins}-{losses}"),
        "titles": s.state_titles,
        "coaches": coaches.iter().rev().map(|&(c, a, b, wn, l)| json!({ "id": c, "name": w.coaches[c].name, "years": if a == b { a.to_string() } else { format!("{a}-{b}") }, "record": format!("{wn}-{l}") })).collect::<Vec<_>>(),
        "alumni": alumni,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_reads_in_words() {
        let mut s = [0i32; afm_engine::world::hs_year::HS_STAT_COUNT];
        s[HsStat::Rush as usize] = 12;
        s[HsStat::RushYds as usize] = 88;
        s[HsStat::RushTd as usize] = 1;
        assert_eq!(line_words(&s), "12 car, 88 yds, 1 TD");
    }

    #[test]
    fn moves_follow_the_squad_rules() {
        // A freshman on varsity can go down to JV or the freshman team.
        assert_eq!(can_move(Squad::Varsity, 9, true, true), vec!["jv", "freshman"]);
        // A senior plays varsity only; a JV junior can only move up.
        assert!(can_move(Squad::Varsity, 12, true, true).is_empty());
        assert_eq!(can_move(Squad::Jv, 11, true, true), vec!["varsity"]);
        // A school without a JV or freshman team offers neither.
        assert!(can_move(Squad::Varsity, 10, false, false).is_empty());
    }

    #[test]
    fn wants_read_in_words() {
        let p = SchoolPerson { wants: [0.6, 0.2, 0.1, 0.1], ..Default::default() };
        assert_eq!(wants_words(&p), "Mostly winning");
        let q = SchoolPerson { wants: [0.3, 0.4, 0.2, 0.1], ..Default::default() };
        assert_eq!(wants_words(&q), "Grades and winning");
    }
}
