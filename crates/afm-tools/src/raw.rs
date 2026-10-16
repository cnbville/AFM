//! The full raw export of a world trial (`AFM_RAW=dir`): everything the world
//! recorded, as JSON lines, so dashboards can be rebuilt without re-running.
//! Hidden values included: debug only. It only reads the world.

use std::io::Write;

use afm_engine::debug;
use afm_engine::world::coach::{Employer, Group};
use afm_engine::world::geo::STATE_CENTERS;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

fn r1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn employer(w: &World, e: Employer) -> Value {
    match e {
        Employer::College(p) => json!({"college": w.programs[p].real_name, "div": w.programs[p].division.name()}),
        Employer::HighSchool(h) => {
            json!({"hs": w.high_schools[h].name, "state": STATE_CENTERS[w.high_schools[h].state].0})
        }
        Employer::Nfl(_) => json!({"nfl": true}),
    }
}

struct Lines {
    out: std::io::BufWriter<std::fs::File>,
    n: usize,
}

impl Lines {
    fn new(dir: &str, name: &str) -> Lines {
        Lines { out: std::io::BufWriter::new(std::fs::File::create(format!("{dir}/{name}")).unwrap()), n: 0 }
    }
    fn put(&mut self, v: Value) {
        serde_json::to_writer(&mut self.out, &v).unwrap();
        self.out.write_all(b"\n").unwrap();
        self.n += 1;
    }
}

fn team_stats(t: &afm_engine::game::stats::TeamStats) -> Value {
    json!({
        "points": t.points, "plays": t.plays, "yards": r1(t.yards), "pass_yards": r1(t.pass_yards),
        "rush_yards": r1(t.rush_yards), "first_downs": t.first_downs, "third_att": t.third_att,
        "third_conv": t.third_conv, "fourth_att": t.fourth_att, "fourth_conv": t.fourth_conv,
        "penalties": t.penalties, "penalty_yds": r1(t.penalty_yds), "turnovers": t.turnovers,
        "top": r1(t.time_of_possession), "sacks": t.sacks, "rz_trips": t.red_zone_trips, "rz_tds": t.red_zone_tds,
        "epa": r2(t.epa), "successes": t.successes, "explosive": t.explosive, "dropbacks": t.dropbacks,
        "pressures": t.pressures, "drives": t.drives, "start_sum": r1(t.start_sum), "punts": t.punts,
        "fg_att": t.fg_att, "fg_made": t.fg_made, "att": t.att, "cmp": t.cmp, "carries": t.carries,
        "ints": t.ints, "fumbles_lost": t.fumbles_lost
    })
}

fn player_detail(p: &afm_engine::player::Player) -> Value {
    let ceil = debug::ceilings(p);
    json!({
        "attrs": afm_engine::attributes::Attr::ALL.iter().map(|&t| json!([format!("{t:?}"), r1(p.attributes[t]), r1(ceil[t])])).collect::<Vec<_>>(),
        "seasons": p.history.iter().map(|h| json!({
            "age": h.age, "level": h.level.name(), "pos": h.position.name(), "role": h.playing_time.name(),
            "start": r1(h.overall_start), "end": r1(h.overall), "perf": h.performance.map(r1),
            "lr": h.level_rating, "lr_end": h.level_rating_end, "pot": r1(debug::season_potential(h)),
            "ev": h.events.iter().map(|e| e.name()).collect::<Vec<_>>(), "missed": h.games_missed, "coach": r1(h.coaching),
            "dip": h.adjustment, "spring": h.spring_gain.map(r1)
        })).collect::<Vec<_>>(),
        "positions": p.position_history.iter().map(|(age, pos)| json!([age, pos.name()])).collect::<Vec<_>>(),
        "personality": {"work_ethic": r1(p.personality.work_ethic), "coachability": r1(p.personality.coachability),
            "competitiveness": r1(p.personality.competitiveness), "love_of_game": r1(p.personality.love_of_game)},
        "bloom": r1(p.bloom), "injury": r1(p.injury_proneness),
    })
}

/// Writes every file of the raw export into `dir` and returns a one-line summary.
pub fn write_raw(w: &World, dir: &str, seed: u32, years: u32) -> String {
    std::fs::create_dir_all(dir).unwrap();
    let first = w.year - years as i32;
    let log = &w.game_log;
    let mut counts: Vec<(String, usize)> = Vec::new();

    let mut f = Lines::new(dir, "games.jsonl");
    for g in &log.games {
        f.put(json!({
            "year": g.year, "div": g.division.name(), "home": g.teams[0], "away": g.teams[1],
            "score": g.score, "ot": g.overtime, "kr": [[g.kr[0].0, r1(g.kr[0].1)], [g.kr[1].0, r1(g.kr[1].1)]],
            "pr": [[g.pr[0].0, r1(g.pr[0].1)], [g.pr[1].0, r1(g.pr[1].1)]],
            "stats": [team_stats(&g.stats[0]), team_stats(&g.stats[1])]
        }));
    }
    counts.push(("games".into(), f.n));

    let mut f = Lines::new(dir, "player_seasons.jsonl");
    let mut keys: Vec<&(i32, u32)> = log.players.keys().collect();
    keys.sort();
    for k in keys {
        let s = &log.players[k];
        let (name, pos0, stars, class) =
            log.names.get(&k.1).map_or(("?".to_string(), None, 0, 0), |n| (n.0.clone(), Some(n.1.name()), n.2, n.3));
        f.put(json!({
            "year": k.0, "id": k.1, "name": name, "first_pos": pos0, "stars": stars, "class_of": class,
            "program": s.program, "pos": s.pos.map(|p| p.name()), "g": s.games, "att": s.att, "cmp": s.cmp,
            "pass_yds": s.pass_yds.round(), "pass_td": s.pass_td, "int": s.int_thrown, "sacked": s.sacked,
            "rush": s.rush, "rush_yds": s.rush_yds.round(), "rush_td": s.rush_td, "rec": s.rec,
            "rec_yds": s.rec_yds.round(), "rec_td": s.rec_td, "tackles": s.tackles, "tfl": r1(s.tfl as f64),
            "sacks": r1(s.sacks as f64), "ints": s.ints, "pbus": s.pbus, "pressures": s.pressures, "kr": s.kr,
            "kr_yds": s.kr_yds.round(), "fg_made": s.fg_made, "fg_att": s.fg_att,
            "grade": if s.graded > 0 { r1(s.grade_sum as f64 / s.graded as f64) } else { 0.0 }
        }));
    }
    counts.push(("player seasons".into(), f.n));

    let mut f = Lines::new(dir, "situations.jsonl");
    let mut keys: Vec<&(i32, afm_engine::world::data::Division, String)> = log.situations.keys().collect();
    keys.sort();
    for k in keys {
        let v = log.situations[k];
        f.put(json!({"year": k.0, "div": k.1.name(), "key": k.2, "v": v.iter().map(|x| r2(*x)).collect::<Vec<_>>()}));
    }
    counts.push(("situation rows".into(), f.n));
    let mut f = Lines::new(dir, "team_situations.jsonl");
    let mut keys: Vec<&(i32, usize, String)> = log.team_situations.keys().collect();
    keys.sort();
    for k in keys {
        let v = log.team_situations[k];
        f.put(json!({"year": k.0, "program": k.1, "key": k.2, "v": v.iter().map(|x| r2(*x)).collect::<Vec<_>>()}));
    }
    counts.push(("team situation rows".into(), f.n));

    let mut f = Lines::new(dir, "lineups.jsonl");
    let mut keys: Vec<&(i32, usize)> = log.lineups.keys().collect();
    keys.sort();
    for k in keys {
        let (sum, n) = &log.lineups[k];
        f.put(json!({
            "year": k.0, "program": k.1,
            "pos": (0..sum.len()).map(|i| json!([afm_engine::positions::Position::ALL[i].name(), if n[i] > 0 { r1(sum[i] / n[i] as f64) } else { 0.0 }, n[i]])).collect::<Vec<_>>(),
            "extras": log.extras.get(k).map(|e| e.iter().map(|v| r2(*v)).collect::<Vec<_>>())
        }));
    }
    counts.push(("lineups".into(), f.n));

    let mut f = Lines::new(dir, "programs.jsonl");
    for p in &w.programs {
        f.put(json!({
            "id": p.id, "name": p.real_name, "fictional": p.fictional_name, "div": p.division.name(), "conf": p.conference,
            "city": p.city, "state": STATE_CENTERS[p.state].0, "enrollment": p.enrollment, "academic": format!("{:?}", p.academic_tier),
            "stadium": p.stadium, "tradition": r2(p.tradition), "facilities": r2(p.facilities), "fans": r2(p.fans),
            "prestige_start": r2(p.prestige_start), "prestige": r2(p.prestige),
            "prestige_history": p.prestige_history.iter().map(|h| json!([h.0, r2(h.1)])).collect::<Vec<_>>(),
            "grade_names": afm_engine::world::program::GradeCat::ALL.iter().map(|g| g.name()).collect::<Vec<_>>(),
            "grade_history": p.grade_history.iter().map(|h| json!([h.0, h.1.iter().map(|v| r2(*v)).collect::<Vec<_>>()])).collect::<Vec<_>>(),
            "nil_budget": p.collective.budget.round(), "nil_base": p.collective.base.round(), "big_booster": p.collective.big_booster.map(|b| b.round()),
            "pipeline": (0..51).filter(|&k| p.pipeline[k] > 0.01).map(|k| json!([STATE_CENTERS[k].0, r2(p.pipeline[k])])).collect::<Vec<_>>(),
            "footprint": (0..51).filter(|&k| p.footprint[k] > 0.01).map(|k| json!([STATE_CENTERS[k].0, r2(p.footprint[k])])).collect::<Vec<_>>(),
            "titles": p.titles, "conf_titles": p.conference_titles, "draft_picks": p.draft_picks,
            "signing_cap": afm_engine::world::ncaa::roster_limits(p, w.year).1, "admissions_bar": r2(p.admissions_bar), "special_admits": p.special_admits,
            "roster_size": p.roster.len(), "story": p.story,
            "seasons": p.seasons.iter().map(|s| { let t = &s.totals; json!({
                "year": s.year, "w": s.wins, "l": s.losses, "conf": s.conference_title, "nat": s.national_title,
                "strength": r2(s.strength), "coach": s.coach, "g": t.games, "pf": t.points_for, "pa": t.points_against,
                "plays": t.plays, "yds": t.yards.round(), "pass": t.pass_yards.round(), "rush": t.rush_yards.round(),
                "yds_allowed": t.yards_allowed.round(), "att": t.att, "cmp": t.cmp, "carries": t.carries,
                "to": t.turnovers, "takeaways": t.takeaways, "sacks": t.sacks, "sacks_allowed": t.sacks_allowed }) }).collect::<Vec<_>>(),
        }));
    }
    counts.push(("programs".into(), f.n));

    let mut f = Lines::new(dir, "coaches.jsonl");
    // College and NFL coaches, and anyone who ever held such a job.
    let above_hs = |e: Employer| !matches!(e, Employer::HighSchool(_));
    for c in w
        .coaches
        .iter()
        .filter(|c| c.job.is_some_and(|j| above_hs(j.employer)) || c.history.iter().any(|s| above_hs(s.employer)))
    {
        let a = debug::coach_ability(c);
        let mut career: Vec<Value> = c
            .history
            .iter()
            .map(|s| json!({"from": s.from, "to": s.to, "role": s.role.name(), "at": employer(w, s.employer), "w": s.wins, "l": s.losses, "titles": s.titles, "ended": s.ended}))
            .collect();
        if let Some(j) = c.job {
            career.push(json!({"from": j.since, "to": null, "role": j.role.name(), "at": employer(w, j.employer), "w": c.wins, "l": c.losses, "titles": c.titles, "ended": "current"}));
        }
        f.put(json!({
            "id": c.id, "name": c.name, "eth": c.ethnicity.name(), "age": c.age, "rep": r2(c.reputation),
            "ambition": r1(c.personality.ambition), "loyalty": r1(c.personality.loyalty), "temper": r1(c.personality.temper),
            "mentor": c.mentor, "ex_pro": c.ex_pro, "retired": c.retired, "career_titles": c.career_titles,
            "came_up": c.came_up.name(), "prove_it": c.prove_it,
            "hidden": {"hc_value": r2(a.head_coach_value()), "teaching": a.teaching.iter().map(|t| r1(*t)).collect::<Vec<_>>(),
                "scheme": r1(a.scheme), "game_mgmt": r1(a.game_management), "motivation": r1(a.motivation),
                "discipline": r1(a.discipline), "recruiting": r1(a.recruiting), "network": r1(a.network), "relationships": r1(a.relationships)},
            "units": Group::ALL.iter().map(|&g| { let u = c.units[g as usize]; json!([g.name(), r2(u.growth_read()), r1(u.seasons), r1(u.moved_up)]) }).collect::<Vec<_>>(),
            "career": career,
        }));
    }
    counts.push(("coaches".into(), f.n));

    let mut f = Lines::new(dir, "alumni.jsonl");
    for a in &w.alumni {
        let mut v = json!({
            "name": a.name, "eth": a.ethnicity.name(), "state": STATE_CENTERS[a.home_state].0, "hs": w.school_name(a.high_school),
            "colleges": a.colleges.iter().map(|&c| w.programs[c].real_name.as_str()).collect::<Vec<_>>(),
            "college_ids": a.colleges, "stars": a.stars, "camp_stars": a.camp_stars, "year": a.year, "pick": a.pick, "pos": a.position.name(),
            "overall": r1(a.overall), "potential": r1(debug::alumnus_potential(a)), "visibility": r2(debug::alumnus_visibility(a)),
            "juco": a.juco, "left": a.left_football, "class_of": a.class_of, "steered": a.steered,
            "background": a.background_summary, "notes": a.notes, "team": a.nfl_team, "gpa": a.gpa_at_signing.map(r2),
            "staff": a.college_staff,
        });
        if let Some(p) = &a.player {
            v["detail"] = player_detail(p);
        }
        f.put(v);
    }
    counts.push(("alumni".into(), f.n));

    let mut f = Lines::new(dir, "people.jsonl");
    for q in &w.people {
        // College players in full; high schoolers only once recruited (3
        // stars and up from the services or the specialist camps).
        let college = matches!(q.stage, Stage::College | Stage::PrepSchool);
        if !college && !(q.stage == Stage::HighSchool && q.stars.max(q.camp_stars) >= 3) {
            continue;
        }
        let mut v = json!({
            "id": q.player.id, "name": q.name, "eth": q.ethnicity.name(), "stage": format!("{:?}", q.stage),
            "pos": q.player.position.name(), "age": q.player.age, "overall": r1(q.player.overall()),
            "potential": r1(debug::person_potential(q)), "stars": q.stars, "camp_stars": q.camp_stars, "class_of": q.class_of,
            "state": STATE_CENTERS[q.home_state].0, "hs": q.hs, "college": q.college, "path": q.path,
            "college_seasons": q.college_seasons, "transfers": q.transfers, "juco": q.juco, "nil": q.nil_income.round(),
            "role": q.last_role.map(|r| r.name()), "perf": q.last_performance.map(r1), "gpa": q.gpa_at_signing.map(r2),
        });
        if college {
            v["detail"] = player_detail(&q.player);
        }
        f.put(v);
    }
    counts.push(("active people".into(), f.n));

    let mut f = Lines::new(dir, "nfl.jsonl");
    for t in &w.nfl {
        f.put(json!({
            "name": t.name, "draft_skill": r1(t.draft_skill), "owner": r1(t.owner), "coaching": r1(t.coaching), "staff_years": t.staff_years,
            "depth": t.depth.iter().enumerate().map(|(i, d)| json!([afm_engine::positions::Position::ALL[i].name(),
                d.iter().map(|m| json!([r1(m.rating), m.age, m.id, m.role.map(|r| r.name()), m.depth])).collect::<Vec<_>>()])).collect::<Vec<_>>(),
        }));
    }
    counts.push(("nfl teams".into(), f.n));

    let mut f = Lines::new(dir, "news.jsonl");
    for n in &w.news {
        f.put(json!({"year": n.year, "kind": format!("{:?}", n.kind), "channel": format!("{:?}", n.channel), "program": n.program, "text": n.text}));
    }
    counts.push(("news items".into(), f.n));

    // Section 4: each football year's calendar counters and its medium and
    // louder events (kept with the game log).
    let mut f = Lines::new(dir, "calendar_years.jsonl");
    for l in &w.log_history {
        f.put(json!({
            "year": l.year, "college_games": l.games, "snow": l.snow_games, "rain": l.rain_games, "freezing": l.freezing_games,
            "lightning": l.lightning_delays, "moved": l.weather_moves, "cancelled": l.weather_cancels, "opt_outs": l.opt_outs,
            "bowl_practice_gain": r1(l.bowl_practice_gain), "four_game_redshirts": l.four_game_redshirts, "redshirt_tricks": l.redshirt_tricks,
            "spring_notices": l.spring_notices, "spring_breakouts": l.spring_breakouts, "spring_fizzles": l.spring_fizzles,
            "hs_games": l.hs_games, "hs_forfeits": l.hs_forfeits, "hs_no_contests": l.hs_no_contests, "hs_grade_outs": l.hs_grade_outs,
            "hs_homecomings": l.hs_homecomings, "hs_senior_nights": l.hs_senior_nights, "hs_all_state": l.hs_all_state,
            "heat_days": l.heat_limited_days, "commits": l.commits, "commits_by_month": l.commits_by_month, "decommits": l.decommits,
            "flips": l.flips, "signed_early": l.signed_early, "signed_late": l.signed_late, "signed_by_division": l.signed_by_division,
            "official_visits": l.official_visits, "unofficial_visits": l.unofficial_visits, "drop_ins": l.drop_ins,
            "early_enrollees": l.early_enrollees, "declared": l.declared, "withdrew": l.withdrew, "portal_entries": l.portal_entries,
            "portal_by_day": l.portal_by_day.iter().map(|(d, n)| json!([d.to_string(), n])).collect::<Vec<_>>(),
            "portal_landed": l.portal_landed, "portal_returned": l.portal_returned, "whispers": l.whispers,
            "sit_out_transfers": l.sit_out_transfers, "spring_ineligible": l.spring_ineligible, "fall_ineligible": l.fall_ineligible,
            "combine_invites": l.combine_invites, "all_star_invites": l.all_star_invites,
            "huge_by_team": l.huge_by_team.iter().map(|&(t, n)| json!([w.programs[t].real_name, n])).collect::<Vec<_>>(),
            "rule_changes": l.rule_changes, "nfl_games": l.nfl_games,
            "nfl_huge_by_team": l.nfl_huge_by_team.iter().enumerate().map(|(t, &n)| json!([w.nfl[t].name, n])).collect::<Vec<_>>(),
        }));
    }
    counts.push(("calendar years".into(), f.n));
    let mut f = Lines::new(dir, "calendar_events.jsonl");
    for (year, events) in &w.event_history {
        for e in events {
            f.put(json!({
                "year": year, "date": e.when.date.to_string(), "min": e.when.minute, "level": format!("{:?}", e.level),
                "kind": format!("{:?}", e.kind), "loud": format!("{:?}", e.loud), "rule": e.rule,
                "nfl": e.nfl_team.map(|t| w.nfl[t].name), "program": e.program.map(|c| w.programs[c].real_name.as_str()),
                "text": e.text, "memorable": e.memorable, "decision": e.needs_decision,
            }));
        }
    }
    counts.push(("calendar events".into(), f.n));
    let mut f = Lines::new(dir, "nfl_champions.jsonl");
    for &(year, t) in &w.nfl_champions {
        f.put(json!({"year": year, "team": w.nfl[t].name}));
    }
    counts.push(("nfl champions".into(), f.n));

    let meta = json!({
        "seed": seed, "years": years, "first": first, "last": w.year - 1,
        "signing_classes": w.signing_classes, "fbs_signees_by_state": (0..51).map(|k| json!([STATE_CENTERS[k].0, w.fbs_signees_by_state[k]])).collect::<Vec<_>>(),
        "overflow": (0..51).map(|k| json!([STATE_CENTERS[k].0, r2(w.overflow[k])])).collect::<Vec<_>>(),
        "situ_values": ["plays", "yards", "epa", "successes", "first_downs", "tds", "turnovers", "extra (completions, sacks, makes, touchbacks, missed tackles on runs)"],
        "files": counts.iter().map(|(k, n)| json!([k, n])).collect::<Vec<_>>(),
    });
    std::fs::write(format!("{dir}/meta.json"), serde_json::to_string_pretty(&meta).unwrap()).unwrap();
    counts.iter().map(|(k, n)| format!("{n} {k}")).collect::<Vec<_>>().join(", ")
}
