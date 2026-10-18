//! The full raw export of a world trial (`AFM_RAW=dir`): everything the world
//! recorded, as JSON lines, so dashboards can be rebuilt without re-running.
//! Hidden values included: debug only. It only reads the world.

use std::io::Write;

use afm_engine::debug;
use afm_engine::world::coach::{Employer, Group};
use afm_engine::world::geo::STATE_CENTERS;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

/// A franchise's name in a season (trial 7 finding 17: the export used
/// today's names, the Commanders in 1996).
fn club(t: usize, year: i32) -> &'static str {
    afm_engine::world::franchise::name(t, year)
}

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
            "dip": h.adjustment, "spring": h.spring_gain.map(r1), "spring_open": h.spring_open.map(r1)
        })).collect::<Vec<_>>(),
        "positions": p.position_history.iter().map(|(age, pos)| json!([age, pos.name()])).collect::<Vec<_>>(),
        "personality": {"work_ethic": r1(p.personality.work_ethic), "coachability": r1(p.personality.coachability),
            "competitiveness": r1(p.personality.competitiveness), "love_of_game": r1(p.personality.love_of_game)},
        "bloom": r1(p.bloom), "injury": r1(p.injury_proneness),
    })
}

/// One pro career for the export: who he is, his deal and every season.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pro_row(
    w: &World,
    id: u32,
    name: &str,
    pos: &str,
    age: u32,
    overall: f64,
    potential: f64,
    c: &afm_engine::world::pro::ProCareer,
    notes: &[String],
    active: bool,
) -> Value {
    let deal = c.contract.as_ref().map(|k| json!({
        "team": w.nfl[k.team].name, "kind": format!("{:?}", k.kind), "signed": k.signed, "years": k.real_years(), "last": k.last_year(),
        "apy": k.apy().round(), "guaranteed": k.guaranteed().round(), "bonus": k.signing_bonus.round(), "option": k.option, "no_trade": k.no_trade,
        "voids": k.years.iter().filter(|y| y.void).count(),
        "by_year": k.years.iter().map(|y| json!([y.year, y.cap_hit().round(), y.cash().round(), y.void])).collect::<Vec<_>>(),
    }));
    let seasons: Vec<Value> = c.seasons.iter().map(|s| {
        let l = &s.stats;
        json!({"year": s.year, "team": s.team.map(|t| club(t, s.year)), "g": s.games, "gs": s.starts, "snaps": s.snaps, "opp": r2(if s.snaps[0] + s.snaps[1] > 0 { s.opp / (s.snaps[0] + s.snaps[1]) as f64 } else { 0.0 }),
            "ps_weeks": s.ps_weeks, "ir_weeks": s.ir_weeks, "pay": s.pay.round(), "pro_bowl": s.pro_bowl, "all_pro": s.all_pro, "awards": s.awards,
            "grade": if l.graded > 0 { Some(r1(l.grade())) } else { None },
            "pass": [l.cmp, l.att, l.pass_yds.round(), l.pass_td, l.int], "rush": [l.rush, l.rush_yds.round(), l.rush_td],
            "rec": [l.rec, l.rec_yds.round(), l.rec_td], "def": [l.tackles, r1(l.sacks), l.ints, l.pbus]})
    }).collect();
    json!({
        "id": id, "name": name, "pos": pos, "age": age, "overall": r1(overall), "potential": r1(potential), "active": active,
        "status": format!("{:?}", c.status), "team": c.team.map(|t| w.nfl[t].name), "draft": c.draft.map(|d| json!([d.0, d.1, d.2, club(d.3, d.0)])),
        "first_year": c.first_year, "accrued": c.accrued, "earnings": c.earnings.round(), "teams": c.teams.iter().map(|&t| w.nfl[t].name).collect::<Vec<_>>(),
        "agent": c.agent.map(|a| w.market.agents[a].name.clone()), "trust": r2(c.trust), "tags": c.tags, "hold": format!("{:?}", c.hold),
        "ipp": c.ipp, "deal": deal, "seasons": seasons, "notes": notes,
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
            "stats": [team_stats(&g.stats[0]), team_stats(&g.stats[1])],
            "tags": g.tags, "loud": g.loud, "emergency_snaps": g.emergency_snaps
        }));
    }
    counts.push(("games".into(), f.n));

    // NFL games on the engine: the same box score, teams by name.
    let mut f = Lines::new(dir, "nfl_games.jsonl");
    for g in &log.nfl_games {
        f.put(json!({
            "year": g.year, "div": "NFL", "home": club(g.teams[0], g.year), "away": club(g.teams[1], g.year),
            "score": g.score, "ot": g.overtime, "kr": [[g.kr[0].0, r1(g.kr[0].1)], [g.kr[1].0, r1(g.kr[1].1)]],
            "pr": [[g.pr[0].0, r1(g.pr[0].1)], [g.pr[1].0, r1(g.pr[1].1)]],
            "stats": [team_stats(&g.stats[0]), team_stats(&g.stats[1])],
        }));
    }
    counts.push(("nfl games".into(), f.n));

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

    // Section 5 (part B 21): every pro career, season by season, with his
    // deal, his agent and his honors; the market's books; trades, comp
    // picks, front offices, NFL honors and the Pro Football Hall of Fame.
    let mut f = Lines::new(dir, "pros.jsonl");
    // A pro who retired at the end of the last season is archived already
    // and still among the people until the year's clean-up: his archived
    // row stands (trial 7 finding 17: 86 pros were written twice).
    let archived: std::collections::HashSet<u32> =
        w.alumni.iter().filter(|a| a.pro.is_some()).filter_map(|a| a.player.as_ref().map(|p| p.id)).collect();
    for q in w.people.iter().filter(|q| q.pro.is_some() && !archived.contains(&q.player.id)) {
        f.put(pro_row(
            w,
            q.player.id,
            &q.name,
            q.player.position.name(),
            q.player.age,
            q.player.overall(),
            debug::person_potential(q),
            q.pro.as_ref().unwrap(),
            &q.notes,
            true,
        ));
    }
    for a in w.alumni.iter().filter(|a| a.pro.is_some()) {
        let (id, age) = a.player.as_ref().map_or((0, 0), |p| (p.id, p.age));
        f.put(pro_row(
            w,
            id,
            &a.name,
            a.position.name(),
            age,
            a.overall,
            debug::alumnus_potential(a),
            a.pro.as_ref().unwrap(),
            &a.notes,
            false,
        ));
    }
    counts.push(("pros".into(), f.n));

    let mut f = Lines::new(dir, "nfl_market.jsonl");
    for m in &w.market.years {
        f.put(json!({
            "year": m.year, "ufa": m.ufa, "resigned": m.resigned, "signed_new": m.signed_new, "perfect": m.perfect, "swayed": m.swayed,
            "tags": m.tags, "transition": m.transition, "tenders": m.tenders, "sheets": m.sheets, "sheets_matched": m.sheets_matched,
            "options_taken": m.options_taken, "options_declined": m.options_declined, "extensions": m.extensions,
            "rookie_extensions": m.rookie_extensions, "restructures": m.restructures, "cap_cuts": m.cap_cuts, "june1": m.june1,
            "futures": m.futures, "waiver_claims": m.waiver_claims, "holdouts": m.holdouts, "holdins": m.holdins, "bluffs": m.bluffs,
            "bluffs_called": m.bluffs_called, "agents_fired": m.agents_fired, "promises_broken": m.promises_broken, "trades": m.trades,
            "deadline_trades": m.deadline_trades, "picks_traded": m.picks_traded, "comp_picks": m.comp_picks,
            "other_league": m.other_league, "ipp": m.ipp, "floor_paid": m.floor_paid.round(),
            "cap_use": m.cap_use.map(|x| (x * 1000.0).round() / 1000.0),
            "dead": m.dead.map(|x| (x * 1000.0).round() / 1000.0),
            "carryover": m.carryover.map(|x| (x * 1000.0).round() / 1000.0),
        }));
    }
    counts.push(("market years".into(), f.n));

    let mut f = Lines::new(dir, "nfl_trades.jsonl");
    for t in &w.market.picks.trades {
        let y = t.date.year();
        f.put(json!({"date": t.date.pretty(), "buyer": club(t.buyer, y), "seller": club(t.seller, y), "picks": t.picks, "text": t.text}));
    }
    for c in &w.market.picks.comp {
        f.put(json!({"comp": true, "year": c.year, "team": club(c.team, c.year), "owner": club(c.owner, c.year), "round": c.round, "kind": format!("{:?}", c.kind), "value": r1(c.value)}));
    }
    counts.push(("trades and comp picks".into(), f.n));

    let mut f = Lines::new(dir, "nfl_clubs.jsonl");
    for (t, c) in w.clubs.iter().enumerate() {
        let o = &c.office;
        let g = &o.gm;
        let yr = w.year;
        f.put(json!({
            "team": w.nfl[t].name, "roster": c.roster.len(), "payroll": afm_engine::world::pro::payroll(w, t, yr).round(),
            "space": afm_engine::world::pro::space(w, t, yr).round(), "dead": c.dead_in(yr).round(), "carryover": c.carryover.round(),
            "cash": c.cash.iter().map(|x| json!([x.0, x.1.round()])).collect::<Vec<_>>(),
            "mode": format!("{:?}", o.mode), "why": o.why, "fans": r2(o.fans), "gm_changes": o.gm_changes,
            "gm": {"name": g.name, "since": g.since, "skill": r1(g.skill), "film_lean": r2(g.film_lean), "sunk_cost": r2(g.sunk_cost),
                "impatience": r2(g.impatience), "not_my_guy": r2(g.not_my_guy), "loyalty": r2(g.loyalty), "recency": r2(g.recency),
                "copycat": r2(g.copycat), "overconfidence": r2(g.overconfidence), "risk": r2(g.risk), "name_value": r2(g.name_value),
                "fixation": g.fixation.map(|p| p.name()), "classic_chart": g.classic_chart, "pays_backs": g.pays_backs,
                "restructures": r2(g.restructures), "early_extensions": r2(g.early_extensions)},
            "owner": {"meddling": r2(o.owner.meddling), "pet": o.owner.pet.map(|i| w.people.get(i).map(|p| p.name.clone())),
                "cash_will": r2(o.owner.cash_will), "patience": r2(o.owner.patience), "pr": r2(o.owner.pr),
                "draft_override": r2(o.owner.draft_override), "trust": r2(o.owner.trust)},
        }));
    }
    counts.push(("nfl clubs".into(), f.n));

    let mut f = Lines::new(dir, "nfl_honors.jsonl");
    let b = &w.nfl_book;
    for a in &b.awards {
        f.put(json!({"kind": "award", "year": a.0, "award": a.1, "id": a.2, "name": a.3, "team": club(a.4, a.0)}));
    }
    for a in &b.all_pro {
        f.put(json!({"kind": "all_pro", "year": a.0, "team_no": a.1, "id": a.2, "pos": a.3.name(), "team": club(a.4, a.0)}));
    }
    for a in &b.pro_bowls {
        f.put(json!({"kind": "pro_bowl", "year": a.0, "id": a.1, "team": club(a.2, a.0)}));
    }
    for r in &b.records {
        f.put(json!({"kind": "record", "stat": r.stat, "career": r.career, "value": r1(r.value), "holder": r.holder, "year": r.year, "world": r.world}));
    }
    for r in &b.broken {
        f.put(json!({"kind": "broken", "year": r.0, "stat": r.1, "holder": r.2, "value": r1(r.3)}));
    }
    for h in &b.hall {
        f.put(json!({"kind": "hall", "year": h.0, "id": h.1, "name": h.2, "pos": h.3.name(), "why": h.4}));
    }
    counts.push(("nfl honors".into(), f.n));

    let mut f = Lines::new(dir, "agents.jsonl");
    for a in &w.market.agents {
        f.put(json!({"name": a.name, "agency": a.agency, "size": a.size, "skill": r1(a.skill), "honesty": r2(a.honesty), "clients": a.clients, "fired": a.fired,
            "rel": a.rel.iter().map(|x| r2(*x)).collect::<Vec<_>>()}));
    }
    counts.push(("agents".into(), f.n));

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
            // The warm-up years before the history (part B 23): checks read only the history.
            "year": l.year, "spin_up": l.year < first, "college_games": l.games, "snow": l.snow_games, "rain": l.rain_games, "freezing": l.freezing_games,
            "lightning": l.lightning_delays, "moved": l.weather_moves, "cancelled": l.weather_cancels, "opt_outs": l.opt_outs,
            "bowl_practice_gain": r1(l.bowl_practice_gain), "four_game_redshirts": l.four_game_redshirts, "redshirt_tricks": l.redshirt_tricks,
            "system_switches": l.system_switches, "fbs_systems": l.fbs_systems,
            "spring_notices": l.spring_notices, "spring_breakouts": l.spring_breakouts, "spring_fizzles": l.spring_fizzles,
            "hs_games": l.hs_games, "hs_forfeits": l.hs_forfeits, "hs_no_contests": l.hs_no_contests, "hs_grade_outs": l.hs_grade_outs,
            "hs_homecomings": l.hs_homecomings, "hs_senior_nights": l.hs_senior_nights, "hs_all_state": l.hs_all_state,
            "heat_days": l.heat_limited_days, "commits": l.commits, "commits_by_month": l.commits_by_month, "decommits": l.decommits,
            "flips": l.flips, "signed_early": l.signed_early, "signed_late": l.signed_late, "signed_by_division": l.signed_by_division,
            "official_visits": l.official_visits, "unofficial_visits": l.unofficial_visits, "drop_ins": l.drop_ins,
            "early_enrollees": l.early_enrollees, "declared": l.declared, "withdrew": l.withdrew, "portal_entries": l.portal_entries,
            "declared_by_day": l.declared_by_day.iter().map(|(d, n)| json!([d.to_string(), n])).collect::<Vec<_>>(),
            "withdrew_by_day": l.withdrew_by_day.iter().map(|(d, n)| json!([d.to_string(), n])).collect::<Vec<_>>(),
            "portal_by_day": l.portal_by_day.iter().map(|(d, n)| json!([d.to_string(), n])).collect::<Vec<_>>(),
            "portal_landed": l.portal_landed, "portal_returned": l.portal_returned, "whispers": l.whispers,
            "sit_out_transfers": l.sit_out_transfers, "spring_ineligible": l.spring_ineligible, "fall_ineligible": l.fall_ineligible,
            "combine_invites": l.combine_invites, "all_star_invites": l.all_star_invites, "all_star_declined": l.all_star_declined, "walk_on_scholarships": l.walk_on_scholarships, "tryouts": l.tryouts,
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
                "year": year, "spin_up": *year < first, "date": e.when.date.to_string(), "min": e.when.minute, "level": format!("{:?}", e.level),
                "kind": format!("{:?}", e.kind), "loud": format!("{:?}", e.loud), "rule": e.rule,
                // The franchise's name on the day (trial 7 finding 17: today's name on 1996 events).
                "nfl": e.nfl_team.map(|t| afm_engine::world::franchise::name(t, e.when.date.year())),
                "program": e.program.map(|c| w.programs[c].real_name.as_str()),
                "text": e.text, "memorable": e.memorable, "decision": e.needs_decision,
            }));
        }
    }
    counts.push(("calendar events".into(), f.n));
    // Every portal entrant (part B 19): days, programs, hold, first choice, return.
    let mut f = Lines::new(dir, "portal.jsonl");
    for r in &w.recruiting_log.portal {
        f.put(json!({
            "year": r.year, "id": r.id, "entered": r.entered.to_string(), "decided": r.decided.to_string(),
            "held": r.held, "from": w.programs[r.from_program].real_name, "from_div": r.from.name(),
            "to": r.to_program.map(|j| w.programs[j].real_name.as_str()), "to_div": r.to.map(|d| d.name()),
            "role": r.role.map(|x| x.name()), "why": r.why, "fell_through": r.fell_through, "returned": r.returned,
        }));
    }
    counts.push(("portal entrants".into(), f.n));
    // High school seasons and state champions (part D 41).
    let mut f = Lines::new(dir, "high_schools.jsonl");
    for h in &w.high_schools {
        if h.seasons.iter().all(|s| s.year < first) {
            continue;
        }
        f.put(json!({
            "id": h.id, "name": h.name, "state": STATE_CENTERS[h.state].0, "town": h.town, "class": h.classification,
            "enrollment": h.enrollment, "private": h.private, "prestige": r2(h.prestige), "titles": h.state_titles,
            "seasons": h.seasons.iter().filter(|s| s.year >= first).map(|s| json!([s.year, s.wins, s.losses, s.state_title])).collect::<Vec<_>>(),
        }));
    }
    counts.push(("high schools".into(), f.n));
    let mut f = Lines::new(dir, "nfl_champions.jsonl");
    for &(year, t) in &w.nfl_champions {
        f.put(json!({"year": year, "team": club(t, year)}));
    }
    counts.push(("nfl champions".into(), f.n));

    // The career stats book (section 3, topic 8): every college season with
    // its nonzero stats, footprint flags and headline percentiles.
    use afm_engine::world::records::{self as rec, Stat};
    let book = &w.stats;
    // Each season's peers (same year, division and position), sorted once
    // per headline metric: the same percentile as `StatsBook::percentile`
    // without scanning the year for every season.
    let mut peers: std::collections::HashMap<(i16, afm_engine::world::data::Division, usize, rec::Metric), Vec<f64>> =
        std::collections::HashMap::new();
    for r in &book.seasons {
        for &m in rec::headline_metrics(r.pos) {
            if let Some(v) = m.value(r) {
                peers.entry((r.year, r.division, r.pos.index(), m)).or_default().push(v);
            }
        }
    }
    for v in peers.values_mut() {
        v.sort_by(|a, b| a.total_cmp(b));
    }
    let percentile = |r: &rec::SeasonRecord, m: rec::Metric| -> Option<f64> {
        let v = m.value(r)?;
        let list = peers.get(&(r.year, r.division, r.pos.index(), m))?;
        if list.len() <= 1 {
            return Some(1.0);
        }
        let below = if m.higher_better() {
            list.partition_point(|&x| x < v)
        } else {
            list.len() - list.partition_point(|&x| x <= v)
        };
        Some(below as f64 / (list.len() - 1) as f64)
    };
    let mut f = Lines::new(dir, "book_seasons.jsonl");
    for r in &book.seasons {
        let mut stats = serde_json::Map::new();
        for &k in Stat::ALL {
            let v = r.get(k);
            if v != 0.0 {
                stats.insert(k.key().to_string(), json!(r2(v)));
            }
        }
        let pct: serde_json::Map<String, Value> = rec::headline_metrics(r.pos)
            .iter()
            .filter_map(|&m| Some((m.name().to_string(), json!(r2(percentile(r, m)?)))))
            .collect();
        f.put(json!({
            "id": r.id, "year": r.year, "div": r.division.name(), "pos": r.pos.to_string(),
            "program": w.programs[r.program as usize].real_name, "g": r.games, "sos": r1(r.sos as f64),
            "footprint": rec::footprint_text(r), "pct": pct, "s": stats,
        }));
    }
    counts.push(("book seasons".into(), f.n));
    let mut f = Lines::new(dir, "record_books.jsonl");
    let mut keys: Vec<_> = book.books.keys().collect();
    keys.sort();
    for key in keys {
        let (scope, b) = key;
        let (div, conf, state) = match scope {
            rec::Scope::Division(d) => (d.name(), None, None),
            rec::Scope::Conference(d, c) => (d.name(), Some(c.as_str()), None),
            rec::Scope::State(s) => ("HS", None, Some(STATE_CENTERS[*s].0)),
        };
        let list = &book.books[key];
        f.put(json!({
            "div": div, "conference": conf, "state": state, "book": b.name(),
            "entries": list.iter().map(|e| json!({
                "value": r1(e.value), "id": e.id, "name": e.name, "program": w.programs.get(e.program).map(|p| p.real_name.as_str()),
                "year": e.year, "from": e.from, "live": e.live,
            })).collect::<Vec<_>>(),
        }));
    }
    counts.push(("record books".into(), f.n));
    let mut f = Lines::new(dir, "hall_of_fame.jsonl");
    for h in &book.hall {
        f.put(json!({
            "year": h.year, "name": h.name, "coach": h.coach, "div": h.division.name(),
            "programs": h.programs.iter().map(|&p| w.programs[p].real_name.as_str()).collect::<Vec<_>>(), "why": h.why,
        }));
    }
    counts.push(("hall of fame".into(), f.n));
    let mut f = Lines::new(dir, "coach_buzz.jsonl");
    for b in &book.buzz {
        f.put(json!({
            "year": b.year, "coach": w.coaches[b.coach].name, "coach_id": b.coach, "program": w.programs[b.program].real_name,
            "offense": b.offense, "pct": r2(b.pct), "gain": r2(b.gain), "why": b.why,
        }));
    }
    counts.push(("coach buzz".into(), f.n));
    let mut f = Lines::new(dir, "records_broken.jsonl");
    for (year, d, b, name, v) in &book.broken {
        f.put(json!({"year": year, "div": d.name(), "book": b.name(), "name": name, "value": r1(*v)}));
    }
    counts.push(("records broken".into(), f.n));

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
