//! Ground zero (next update, lane 10, SK68): a start at a team that
//! doesn't exist yet, and its year before the first game. An NFL expansion
//! club (the other clubs to scout, your staff's guess at their open lists,
//! your queue for the expansion draft); a college in its build year (the
//! squad on the way, the tryouts, the first class); a new high school (the
//! zone's players, who must move, who chooses, your visits to the
//! juniors).

use afm_engine::world::World;
use afm_engine::world::new_teams::{ground_zero as gz, nfl, school};
use afm_engine::world::seat::Job;
use serde_json::{Value, json};

use crate::views::{TeamRef, date_text, team_brief};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "gz_jobs" => game.w().map(|w| jobs(w, args.get("level").and_then(Value::as_str).unwrap_or("college"))),
        "gz_page" => game.w().and_then(|w| {
            // Whether the team can dress for a game yet (the Plans pages
            // wait for a roster).
            let mut v = page(w)?;
            v["dress"] = json!(afm_engine::world::plans::our_team(w).is_some());
            Ok(v)
        }),
        "gz_queue" => game.wm().and_then(|w| {
            let t = my_new_club(w).ok_or("Your club has no expansion draft ahead.")?;
            let ids: Vec<u32> = args
                .get("ids")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_u64).map(|x| x as u32).collect())
                .unwrap_or_default();
            let _ = t;
            w.expansion.queue = ids.into_iter().take(60).collect();
            page(w)
        }),
        "gz_pitch" => game.wm().and_then(|w| {
            let h = my_new_school(w).ok_or("Your school isn't opening this year.")?;
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the player.")? as u32;
            let n = w.new_teams.schools.iter_mut().find(|n| n.school == h).ok_or("No such school.")?;
            if n.zoned {
                return Err("The families have chosen: the school has opened.".into());
            }
            if n.pitched.len() >= PITCHES && !n.pitched.contains(&id) {
                return Err(format!("You have made your {PITCHES} home visits."));
            }
            if !n.pitched.contains(&id) {
                n.pitched.push(id);
            }
            page(w)
        }),
        _ => return None,
    })
}

/// Home visits to zoned juniors before the first day (proposed, not
/// confirmed: a coach of a new school gets round about a dozen families).
const PITCHES: usize = 12;

// ---------------------------------------------------------------------------
// The setup's list
// ---------------------------------------------------------------------------

/// The teams you could build, by level.
pub fn jobs(w: &World, level: &str) -> Value {
    let year = w.year;
    let v: Vec<Value> = match level {
        "nfl" => gz::nfl_markets(w)
            .into_iter()
            .map(|k| {
                let (city, state, metro, nick) = nfl::MARKETS[k];
                json!({
                    "ref": { "kind": "gz_nfl", "id": k },
                    "name": format!("{city} {nick}"),
                    "city": city, "state": state, "metro": metro,
                    "line": format!("Expansion club · {city}, {state}"),
                    "sub": format!("{city}, {state} · {metro:.1} million people"),
                    "first": year + 1,
                    "timeline": [
                        format!("{year}: the owners' vote, a front office and a coaching staff; you scout every club all season"),
                        format!("February {}: the other clubs list their open players; the expansion draft two weeks later", year + 1),
                        format!("{}: free agency, then the draft with extra picks; the first season", year + 1),
                    ],
                })
            })
            .collect(),
        "hs" => gz::school_candidates(w)
            .into_iter()
            .take(600)
            .map(|h| {
                let s = &w.high_schools[h];
                let code = &w.states[s.state].code;
                let new = (s.enrollment as f64 * school::ZONE_SHARE) as u32;
                json!({
                    "ref": { "kind": "gz_hs", "id": h },
                    "name": format!("New school in {}", s.town),
                    "parent": w.school_name(h),
                    "town": s.town, "state": code,
                    "enrollment": new,
                    "parent_enrollment": s.enrollment,
                    "growth": (school::young_growth(s.state) * 10000.0).round() / 100.0,
                    "line": format!("Splits {}'s zone · {}, {code}", w.school_name(h), s.town),
                    "sub": format!("{}, {code}", s.town),
                    "first": year,
                    "timeline": [
                        format!("Spring {year}: the board redraws the lines; you hire your staff and scout the zone"),
                        format!("August {year}: freshmen and sophomores in the zone move; juniors choose; seniors stay"),
                        format!("Fall {year}: the first varsity schedule, with no district until the next realignment"),
                    ],
                })
            })
            .collect(),
        _ => gz::college_candidates(w)
            .into_iter()
            .map(|p| {
                let x = &w.programs[p];
                let level = gz::start_level(w, p);
                let code = &w.states[x.state].code;
                let restart = x.history.iter().any(|h| h.2.is_some()) || x.dark.iter().any(|d| d.0 != i32::MIN);
                json!({
                    "ref": { "kind": "gz_college", "id": p },
                    "name": w.program_name(p),
                    "division": level.name(),
                    "city": x.city, "state": code,
                    "enrollment": x.enrollment,
                    "restart": restart,
                    "line": format!("{} · {}, {code}", level.name(), x.city),
                    "sub": format!("{} · {}, {code}{}", level.name(), x.city, if restart { " · bringing football back" } else { "" }),
                    "first": year + 1,
                    "timeline": [
                        format!("{year}: the build year. Practices and no games; walk-on tryouts and junior college finds in the spring"),
                        format!("December {year}: the first class signs, with nothing on the field yet"),
                        format!("{}: the first games, as an independent", year + 1),
                    ],
                })
            })
            .collect(),
    };
    json!({ "level": level, "jobs": v })
}

/// The job a ground zero pick makes: the team is created now (the owners'
/// award, the school's word, the district's lines). None for any other job.
pub fn make_team(w: &mut World, job: &Value) -> Result<Option<Job>, String> {
    let kind = job.get("kind").and_then(Value::as_str).unwrap_or("");
    let id = job.get("id").and_then(Value::as_u64).map(|v| v as usize);
    Ok(Some(match (kind, id) {
        ("gz_nfl", Some(k)) => {
            if !gz::nfl_markets(w).contains(&k) {
                return Err("That market has a club.".into());
            }
            Job::Nfl(gz::nfl_club(w, k))
        }
        ("gz_college", Some(p)) => {
            if !gz::college_candidates(w).contains(&p) {
                return Err("That school can't start football now.".into());
            }
            gz::college(w, p);
            Job::College(p)
        }
        ("gz_hs", Some(h)) => {
            if h >= w.high_schools.len() || w.high_schools[h].private {
                return Err("No such school.".into());
            }
            Job::HighSchool(gz::school(w, h))
        }
        _ => return Ok(None),
    }))
}

// ---------------------------------------------------------------------------
// Your new team's page
// ---------------------------------------------------------------------------

fn my_new_club(w: &World) -> Option<usize> {
    match crate::views::mine(w)? {
        TeamRef::Nfl(t) => w.expansion.club(t).filter(|c| !c.drafted).map(|c| c.team),
        _ => None,
    }
}

fn my_new_school(w: &World) -> Option<usize> {
    match crate::views::mine(w)? {
        TeamRef::HighSchool(h) => w.new_teams.school(h).map(|n| n.school),
        _ => None,
    }
}

fn person(w: &World, i: usize) -> Value {
    let p = &w.people[i];
    json!({ "id": p.player.id, "name": p.name, "pos": format!("{:?}", p.player.position), "age": p.player.age })
}

/// Your new team's year: where it stands, and the tools of its level.
fn page(w: &World) -> Result<Value, String> {
    let me = crate::views::mine(w).ok_or("You have no team yet.")?;
    let brief = team_brief(w, me);
    let year = w.year;
    Ok(match me {
        TeamRef::Nfl(t) => {
            let c = w.expansion.club(t).ok_or("Your club isn't new.")?;
            let lists_day = if c.first == year { nfl::lists_day(w) } else { w.dates.start.add(365 + 3) };
            let draft_day = if c.first == year { nfl::draft_day(w) } else { w.dates.start.add(365 + 14) };
            let lists_in = !w.expansion.lists.is_empty();
            let by_id = |id: u32| w.people.iter().position(|p| p.player.id == id);
            let open: Vec<Value> = if lists_in {
                w.expansion
                    .lists
                    .iter()
                    .filter_map(|&(club, id)| by_id(id).map(|i| (club, i)))
                    .map(|(club, i)| {
                        let mut v = person(w, i);
                        v["club"] = json!(w.nfl_team_name(club));
                        v["grade"] = json!(gz::staff_grade(w, t, i).round());
                        v
                    })
                    .collect()
            } else if !c.drafted {
                gz::open_guess(w, t)
                    .into_iter()
                    .map(|(club, i, g)| {
                        let mut v = person(w, i);
                        v["club"] = json!(w.nfl_team_name(club));
                        v["grade"] = json!(g.round());
                        v
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let taken: Vec<Value> = w
                .expansion
                .taken
                .iter()
                .filter(|x| x.0 == t)
                .filter_map(|&(_, id)| by_id(id))
                .map(|i| person(w, i))
                .collect();
            let partner: Vec<Value> = w
                .expansion
                .clubs
                .iter()
                .filter(|x| x.team != t && x.first == c.first)
                .map(|x| team_brief(w, TeamRef::Nfl(x.team)))
                .collect();
            json!({
                "kind": "nfl", "team": brief, "first": c.first, "awarded": c.awarded,
                "fee": c.fee, "metro": c.metro, "partner": partner,
                "drafted": c.drafted, "lists_in": lists_in,
                "lists_day": date_text(lists_day), "draft_day": date_text(draft_day),
                "open": open, "queue": w.expansion.queue, "taken": taken,
                "steps": [
                    { "when": format!("{}", c.awarded), "what": "The owners award the club", "done": true },
                    { "when": date_text(lists_day), "what": "The other clubs list their open players", "done": lists_in || c.drafted },
                    { "when": date_text(draft_day), "what": "The expansion draft", "done": c.drafted },
                    { "when": date_text(w.dates.draft[0].date), "what": if c.first == year { "The draft, with your extra picks" } else { "This year's draft: next year's is yours" }, "done": false },
                    { "when": format!("{}", c.first), "what": "The first season", "done": year > c.first },
                ],
            })
        }
        TeamRef::College(p) => {
            let n = w.new_teams.program(p).ok_or("Your program isn't new.")?;
            let squad = w.programs[p].roster.len();
            let coming = w.people.iter().filter(|x| x.cal.signed.is_some_and(|s| s.program == p)).count();
            let committed = w.people.iter().filter(|x| x.cal.commit.is_some_and(|c| c.0 == p)).count();
            json!({
                "kind": "college", "team": brief, "build": n.build, "first": n.first, "restart": n.restart,
                "building": w.new_teams.build_year(p, year),
                "squad": squad, "coming": coming, "committed": committed,
                "tryouts": date_text(w.dates.summer_enrollment),
                "steps": [
                    { "when": format!("{}", n.announced), "what": if n.restart { "The school brings football back" } else { "The school starts football" }, "done": true },
                    { "when": date_text(w.dates.summer_enrollment), "what": "Walk-on tryouts and junior college finds", "done": year > n.build || w.today > w.dates.summer_enrollment },
                    { "when": format!("Fall {}", n.build), "what": "The build year: practices, no games", "done": year > n.build },
                    { "when": date_text(w.dates.signing_day), "what": "The first class signs", "done": year >= n.first },
                    { "when": format!("{}", n.first), "what": "The first games", "done": year > n.first },
                ],
            })
        }
        TeamRef::HighSchool(h) => {
            let n = w.new_teams.school(h).ok_or("Your school isn't new.")?;
            let parents: Vec<Value> = n.parents.iter().map(|&q| team_brief(w, TeamRef::HighSchool(q))).collect();
            let zone: Vec<Value> = if n.zoned {
                Vec::new()
            } else {
                let mut z = school::zone(w, h);
                z.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
                // This spring's seniors graduate before the new school opens.
                z.into_iter()
                    .filter(|&(i, _)| w.people[i].class_of > year)
                    .map(|(i, chance)| {
                        let p = &w.people[i];
                        let mut v = person(w, i);
                        v["grade"] = json!(match p.class_of - year {
                            3.. => "Sophomore",
                            2 => "Junior",
                            _ => "Senior",
                        });
                        v["chance"] = json!((chance * 100.0).round());
                        v["pitched"] = json!(n.pitched.contains(&p.player.id));
                        v
                    })
                    .collect()
            };
            // The season on hand can still be last year's before the year turns.
            let class_day = w.hs_class_day().filter(|d| d.year() >= n.opens).map(date_text);
            json!({
                "kind": "hs", "team": brief, "opens": n.opens, "parents": parents,
                "zoned": n.zoned, "moved": n.moved, "stayed": n.stayed,
                "zone": zone, "pitches": PITCHES, "pitched": n.pitched.len(),
                "class_day": class_day,
                "steps": [
                    { "when": format!("{}", n.announced), "what": "The district votes to build", "done": true },
                    { "when": format!("Spring {}", n.opens), "what": "The lines are redrawn; you scout the zone", "done": true },
                    { "when": class_day.clone().unwrap_or_else(|| format!("August {}", n.opens)), "what": "The first day: the zone's families choose", "done": n.zoned },
                    { "when": format!("Fall {}", n.opens), "what": "The first varsity schedule", "done": year > n.opens },
                ],
            })
        }
        TeamRef::Other(_) => return Err("Ground zero is for a new NFL club, college or high school.".into()),
    })
}

/// Whether your team is a new one still in its year before the first game
/// (the sidebar shows its page then).
pub fn status(w: &World) -> Value {
    let year = w.year;
    let new = match crate::views::mine(w) {
        Some(TeamRef::Nfl(t)) => w.expansion.club(t).is_some_and(|c| year <= c.first),
        Some(TeamRef::College(p)) => w.new_teams.program(p).is_some_and(|n| year <= n.first),
        Some(TeamRef::HighSchool(h)) => w.new_teams.school(h).is_some_and(|n| year <= n.opens),
        _ => false,
    };
    json!(new)
}
