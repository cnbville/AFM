//! Section 2 trial: builds a real-size world with 30 years of hidden history
//! and prints the notable outcomes with the facts behind each one, so every
//! result can be given a "why". Uses the debug feature (hidden values).
//! Usage: world_trial [seed] [history_years] [future_years]
//! Future years run the world on past the save's start (2026), the way the
//! game will, and the report covers history and future together.

use std::collections::BTreeMap;
use std::time::Instant;

use afm_engine::debug;
use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::data::{self, Division};
use afm_engine::world::geo::STATE_CENTERS;
use afm_engine::world::news::{Channel, NewsKind};
use afm_engine::world::program::{GradeCat, letter};
use afm_engine::world::{Alumnus, Stage, World, WorldOptions};

fn div_name(w: &World, a: &Alumnus) -> String {
    a.colleges
        .iter()
        .map(|&c| format!("{} ({})", w.programs[c].real_name, w.programs[c].division.name()))
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn show_player(w: &World, a: &Alumnus) {
    let outcome = match a.pick {
        Some(p) => format!("pick {p} in {}", a.year),
        None if a.left_football => format!("left football by {}", a.year),
        None => format!("undrafted in {}", a.year),
    };
    println!(
        "- {} ({}, {}-star, {} {}) | {} | true potential {:.0}, overall at exit {:.0}, senior visibility {:.2}",
        a.name,
        a.position.name(),
        a.stars,
        w.school_name(a.high_school),
        STATE_CENTERS[a.home_state].0,
        outcome,
        debug::alumnus_potential(a),
        a.overall,
        debug::alumnus_visibility(a)
    );
    println!("  path: {}", div_name(w, a));
    println!("  background: {}", a.background_summary);
    for n in &a.notes {
        println!("  * {n}");
    }
}

fn employer_name(w: &World, e: Employer) -> String {
    match e {
        Employer::College(p) => format!("{} ({})", w.programs[p].real_name, w.programs[p].division.name()),
        Employer::HighSchool(h) => {
            format!("{} HS ({})", w.high_schools[h].name, STATE_CENTERS[w.high_schools[h].state].0)
        }
        Employer::Nfl(t) => format!("{} (NFL)", w.nfl[t].name),
    }
}

fn coach_career(w: &World, c: usize) {
    let coach = &w.coaches[c];
    let a = debug::coach_ability(coach);
    let teach: f64 = a.teaching.iter().sum::<f64>() / 8.0;
    println!(
        "- {} (age {}), reputation {:.1}; hidden: head-coach value {:.1}, teaching {:.1}, recruiting {:.1}, network {:.1}, game mgmt {:.1}; ex-pro {}; mentor {}",
        coach.name,
        coach.age,
        coach.reputation,
        a.head_coach_value(),
        teach,
        a.recruiting,
        a.network,
        a.game_management,
        coach.ex_pro,
        coach.mentor.map_or("none".to_string(), |m| w.coaches[m].name.clone())
    );
    for s in &coach.history {
        println!(
            "  {}-{} {} at {}: {}-{}, {} titles, {}",
            s.from,
            s.to,
            s.role.name(),
            employer_name(w, s.employer),
            s.wins,
            s.losses,
            s.titles,
            s.ended
        );
    }
    if let Some(j) = coach.job {
        println!(
            "  {}-now {} at {}: {}-{}, {} titles",
            j.since,
            j.role.name(),
            employer_name(w, j.employer),
            coach.wins,
            coach.losses,
            coach.titles
        );
    }
}

fn program_story(w: &World, i: usize) {
    let p = &w.programs[i];
    println!(
        "### {} ({}, {}): prestige {:.2} -> {:.2}, national titles in history {}, conference titles {}",
        p.real_name,
        p.division.name(),
        p.conference,
        p.prestige_start,
        p.prestige,
        p.seasons.iter().filter(|s| s.national_title).count(),
        p.seasons.iter().filter(|s| s.conference_title).count()
    );
    let mut line = String::new();
    for s in &p.seasons {
        let t = if s.national_title {
            "*N"
        } else if s.conference_title {
            "*C"
        } else {
            ""
        };
        line.push_str(&format!("{} {}-{}{}; ", s.year, s.wins, s.losses, t));
    }
    println!("seasons: {line}");
    let mut coaches: Vec<(i32, String)> = Vec::new();
    for c in &w.coaches {
        for s in &c.history {
            if s.employer == Employer::College(i) && s.role == Role::HeadCoach {
                coaches
                    .push((s.from, format!("{}-{} {} ({}-{}, {})", s.from, s.to, c.name, s.wins, s.losses, s.ended)));
            }
        }
        if let Some(j) = c.job
            && j.employer == Employer::College(i)
            && j.role == Role::HeadCoach
        {
            coaches.push((
                j.since,
                format!("{}-now {} ({}-{}, rep {:.1})", j.since, c.name, c.wins, c.losses, c.reputation),
            ));
        }
    }
    coaches.sort();
    println!("head coaches: {}", coaches.into_iter().map(|c| c.1).collect::<Vec<_>>().join("; "));
    if !p.story.is_empty() {
        println!("why it moved: {}", p.story.iter().map(|(y, t)| format!("{y} {t}")).collect::<Vec<_>>().join(" | "));
    }
    let warnings: Vec<i32> = w
        .news
        .iter()
        .filter(|n| n.program == Some(i) && n.channel == Channel::News && matches!(n.kind, NewsKind::NilWarning { .. }))
        .map(|n| n.year)
        .collect();
    println!("NIL now ${:.1}M; warning years: {:?}", p.collective.budget / 1e6, warnings);
    let mut pipes: Vec<(f64, &str)> =
        (0..51).filter(|&s| p.pipeline[s] > 0.05).map(|s| (p.pipeline[s], STATE_CENTERS[s].0)).collect();
    pipes.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("pipelines: {}", pipes.iter().take(5).map(|(v, s)| format!("{s} {v:.2}")).collect::<Vec<_>>().join(", "));
    let picks: BTreeMap<u32, usize> = p.draft_picks.iter().fold(BTreeMap::new(), |mut m, &(_, r)| {
        *m.entry(r).or_default() += 1;
        m
    });
    println!("draft picks by round: {picks:?}");
    let g = |c: GradeCat| letter(p.grades[c as usize]);
    println!(
        "grades now: title chances {}, coach {}, pipeline {}, NIL {}, tradition {}, facilities {}, playing time {}",
        g(GradeCat::TitleChances),
        g(GradeCat::CoachPrestige),
        g(GradeCat::ProPipeline),
        g(GradeCat::Nil),
        g(GradeCat::Tradition),
        g(GradeCat::Facilities),
        g(GradeCat::PlayingTime)
    );
}

/// A program's division in a season: its real one through history, then
/// its own (trial 7 finding 17: seasons carry no division, so old FCS
/// titles counted as FBS).
fn division_in(p: &afm_engine::world::program::Program, year: i32) -> Division {
    if year <= afm_engine::world::data::HISTORY_LAST {
        p.real_status(year).map_or(p.division, |s| s.0)
    } else {
        p.division
    }
}

fn fbs_in(p: &afm_engine::world::program::Program, year: i32) -> bool {
    division_in(p, year) == Division::FBS
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let t = Instant::now();
    // AFM_ENGINE_FROM=year plays college games on the match engine from that year.
    let engine_from = std::env::var("AFM_ENGINE_FROM").ok().and_then(|v| v.parse().ok());
    // The dashboard and player pages read the game log (box score totals and
    // college stat lines). Keeping it only watches the games.
    let keep_game_log =
        std::env::var("AFM_DASH").is_ok() || std::env::var("AFM_CARDS").is_ok() || std::env::var("AFM_RAW").is_ok();
    // AFM_SCALE=0.05: a small world, for quick checks of the tooling only.
    let scale = std::env::var("AFM_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let mut w = World::new(WorldOptions {
        seed,
        history_years: years,
        engine_from,
        keep_game_log,
        scale,
        ..Default::default()
    });
    let future: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let built = t.elapsed().as_secs_f64();
    // Memory (trial 8 finding 11), with the game log on or off: the log is
    // trial tooling only, so the world's own budget is read without it.
    let log_state = if keep_game_log { "on" } else { "off" };
    eprintln!("memory after the history (game log {log_state}): {}", afm_tools::memory_text());
    // With AFM_RAW, the world's tendencies by level at the save's start and
    // after every future season, written as they come.
    let mut drift = std::env::var("AFM_RAW").ok().map(|dir| {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::File::create(format!("{dir}/tend_by_year.jsonl")).unwrap()
    });
    let mut snap = |w: &World| {
        if let Some(f) = drift.as_mut() {
            use std::io::Write;
            writeln!(f, "{}", afm_tools::raw::tend_summary(w)).unwrap();
        }
    };
    snap(&w);
    for _ in 0..future {
        let s = Instant::now();
        w.advance_year();
        eprintln!(
            "future season to {}: {:.0}s, memory {}",
            w.year,
            s.elapsed().as_secs_f64(),
            afm_tools::memory_text()
        );
        snap(&w);
    }
    println!(
        "# World trial: seed {seed}, {years} years of history and {future} into the future, history built in {built:.0}s, all in {:.0}s, memory {} with the game log {log_state}\n",
        t.elapsed().as_secs_f64(),
        afm_tools::memory_text()
    );
    // Every report below covers the whole recorded span, history and future.
    let years = years + future;

    // AFM_CARDS=path.json: full player pages for featured players.
    let mut card_values: Vec<serde_json::Value> = Vec::new();
    if let Ok(path) = std::env::var("AFM_CARDS") {
        let first = w.year - years as i32 + 5;
        let al = || w.alumni.iter().enumerate().filter(|(_, a)| a.player.is_some() && a.year >= first);
        let mut picked: Vec<(usize, String)> = Vec::new();
        for (k, a) in al() {
            let tag = match a.pick {
                Some(1) => "Number 1 pick".to_string(),
                Some(p) if p <= 32 && a.year + 2 >= w.year => format!("Round 1 of the {} draft", a.year),
                _ => continue,
            };
            picked.push((k, tag));
        }
        let gems = al().filter(|(_, a)| {
            a.stars <= 2
                && a.pick.is_some_and(|p| p <= 64)
                && a.colleges.first().is_some_and(|&c| w.programs[c].division != Division::FBS)
        });
        picked.extend(gems.take(3).map(|(k, _)| (k, "Hidden gem".to_string())));
        let busts = al().filter(|(_, a)| a.stars == 5 && a.left_football);
        picked.extend(busts.take(3).map(|(k, _)| (k, "Five-star who left football".to_string())));
        let late = al().filter(|(_, a)| a.pick.is_some_and(|p| p >= 200));
        picked.extend(late.take(5).map(|(k, _)| (k, "Late-round pick".to_string())));
        let undrafted =
            al().filter(|(_, a)| a.year + 1 >= w.year && a.pick.is_none() && !a.left_football && a.stars >= 4);
        picked.extend(undrafted.take(5).map(|(k, _)| (k, "Undrafted blue-chip".to_string())));
        // Pro careers (section 5): Hall of Famers, MVPs, big free agents,
        // traded players, holdouts and undrafted starters.
        let career = |k: usize| {
            w.people
                .iter()
                .find(|q| q.pro.as_ref().and_then(|c| c.alumnus) == Some(k))
                .and_then(|q| q.pro.as_deref())
                .or(w.alumni[k].pro.as_deref())
        };
        let id_of = |k: usize| w.alumni[k].player.as_ref().map(|p| p.id);
        let hall: Vec<u32> = w.nfl_book.hall.iter().map(|h| h.1).collect();
        let mvp: Vec<u32> = w.nfl_book.awards.iter().filter(|a| a.1 == "MVP").map(|a| a.2).collect();
        let traded_ids: Vec<u32> = w.market.picks.trades.iter().map(|t| t.player).collect();
        let mut extra = |label: &str, n: usize, f: &dyn Fn(usize) -> bool| {
            let mut got = 0;
            for (k, _) in al() {
                if got == n {
                    break;
                }
                if !picked.iter().any(|x| x.0 == k) && f(k) {
                    picked.push((k, label.to_string()));
                    got += 1;
                }
            }
        };
        extra("Hall of Famer", 3, &|k| id_of(k).is_some_and(|id| hall.contains(&id)));
        extra("NFL MVP", 2, &|k| id_of(k).is_some_and(|id| mvp.contains(&id)));
        extra("Traded", 3, &|k| id_of(k).is_some_and(|id| traded_ids.contains(&id)));
        extra("Big free agent deal", 3, &|k| {
            career(k).is_some_and(|c| {
                c.contract.as_ref().is_some_and(|d| {
                    d.kind == afm_engine::world::pro::money::Kind::Veteran
                        && c.draft.is_none_or(|x| x.3 != d.team)
                        && d.apy() > afm_engine::world::pro::money::price_cap(w.year) * 0.08
                })
            })
        });
        extra("Held out", 2, &|k| career(k).is_some_and(|c| c.hold != afm_engine::world::pro::market::Hold::None));
        extra("Undrafted to starter", 3, &|k| {
            w.alumni[k].pick.is_none()
                && career(k).is_some_and(|c| c.seasons.iter().map(|s| s.starts).sum::<u32>() >= 32)
        });
        // Build 9 (sections 6 and 7): draft-day trades, undrafted rush
        // signings, suspensions and captains.
        let noted = |k: usize, text: &str| {
            w.people
                .iter()
                .find(|q| q.pro.as_ref().and_then(|c| c.alumnus) == Some(k))
                .map_or(&w.alumni[k].notes, |q| &q.notes)
                .iter()
                .any(|n| n.contains(text))
        };
        extra("Traded up for on draft day", 3, &|k| noted(k, "traded up to pick"));
        extra("Undrafted rush signing", 3, &|k| noted(k, "as an undrafted free agent"));
        extra("Suspended", 3, &|k| noted(k, "Suspended"));
        extra("Off-field trouble, no suspension", 2, &|k| noted(k, "ended without a suspension"));
        let cards: Vec<serde_json::Value> =
            picked.iter().map(|(k, tag)| afm_tools::dashboard::card(&w, *k, tag)).collect();
        std::fs::write(&path, serde_json::to_string(&cards).unwrap()).unwrap();
        println!("wrote {} cards to {path}\n", cards.len());
        card_values = cards;
    }
    if let Ok(path) = std::env::var("AFM_DASH") {
        write_dashboard(&w, &path, seed, years, &card_values);
        println!("wrote the debug dashboard data to {path}\n");
    }
    // AFM_RAW=dir: everything the world recorded, as JSON lines.
    if let Ok(dir) = std::env::var("AFM_RAW") {
        let summary = afm_tools::raw::write_raw(&w, &dir, seed, years);
        println!("wrote the raw export to {dir}: {summary}\n");
    }
    if keep_game_log {
        afm_tools::dashboard::print_engine(&w);
    }
    // AFM_ROOKIES=1: how good drafted players are on draft day, by round, then stop.
    if std::env::var("AFM_ROOKIES").is_ok() {
        let first = w.year - years as i32 + 5;
        let drafted: Vec<&Alumnus> = w.alumni.iter().filter(|a| a.pick.is_some() && a.year >= first).collect();
        let yrs = (w.year - first + 1).max(1) as f64;
        println!("drafted {} over {:.0} drafts", drafted.len(), yrs);
        for r in 1..=7u32 {
            let mut v: Vec<f64> = drafted
                .iter()
                .filter(|a| afm_engine::career::draft_round(a.pick.unwrap(), 257) == r)
                .map(|a| a.overall)
                .collect();
            v.sort_by(|a, b| a.total_cmp(b));
            if v.is_empty() {
                continue;
            }
            let q = |f: f64| v[((v.len() - 1) as f64 * f) as usize];
            println!(
                "round {r}: n {} | median {:.0} | 10th pct {:.0} | 90th pct {:.0} | max {:.0}",
                v.len(),
                q(0.5),
                q(0.1),
                q(0.9),
                q(1.0)
            );
        }
        for cut in [1u32, 5, 32] {
            let mut by: BTreeMap<&str, usize> = BTreeMap::new();
            for a in drafted.iter().filter(|a| a.pick.unwrap() <= cut) {
                *by.entry(a.position.name()).or_default() += 1;
            }
            println!("picks 1-{cut} by position: {by:?}");
        }
        for t in [80.0, 85.0, 88.0, 92.0] {
            let n = drafted.iter().filter(|a| a.overall >= t).count();
            println!("rookies at {t}+: {n} ({:.1} per draft)", n as f64 / yrs);
        }
        return;
    }
    // AFM_SHOW="Georgia": one program's story and every head coach it had, then stop.
    if let Ok(name) = std::env::var("AFM_SHOW") {
        if let Some(i) = w.programs.iter().position(|p| p.real_name == name) {
            program_story(&w, i);
            println!("\nIts head coaches:");
            for c in 0..w.coaches.len() {
                let here = w.coaches[c]
                    .history
                    .iter()
                    .any(|s| s.role == Role::HeadCoach && s.employer == Employer::College(i))
                    || w.programs[i].head_coach == Some(c);
                if here {
                    coach_career(&w, c);
                }
            }
        }
        return;
    }

    special_teams(&w);

    // --- Calibration ---
    println!("## Calibration");
    let picks: Vec<&Alumnus> = w.alumni.iter().filter(|a| a.pick.is_some()).collect();
    let mut by_div: BTreeMap<&str, usize> = BTreeMap::new();
    for a in &picks {
        *by_div.entry(w.programs[*a.colleges.last().unwrap()].division.name()).or_default() += 1;
    }
    println!("draft picks by final division: {by_div:?} of {}", picks.len());
    let complete: Vec<i32> = w.signing_classes.iter().map(|c| c.0).filter(|&y| y + 6 <= w.year).collect();
    for stars in [5u8, 4, 3, 2] {
        let total: u32 =
            w.signing_classes.iter().filter(|c| complete.contains(&c.0)).map(|c| c.1[stars as usize]).sum();
        let drafted = picks
            .iter()
            .filter(|a| a.stars == stars && !a.position.is_specialist() && complete.contains(&a.class_of))
            .count();
        if total > 0 {
            println!("{stars}-star draft rate: {:.1}% ({drafted} of {total})", 100.0 * drafted as f64 / total as f64);
        }
    }
    let total_sim: u32 = w.fbs_signees_by_state.iter().sum();
    let total_real: f64 = data::STATES.iter().map(|s| s.fbs_signees_per_year).sum();
    println!("FBS high school signees by home state, share sim vs real:");
    let mut rows: Vec<(f64, String)> = data::STATES
        .iter()
        .map(|s| {
            let i = afm_engine::world::geo::state_index(&s.code).unwrap();
            let sim = w.fbs_signees_by_state[i] as f64 / total_sim.max(1) as f64;
            (
                s.fbs_signees_per_year / total_real,
                format!("{} {:.1}% vs {:.1}%", s.code, 100.0 * sim, 100.0 * s.fbs_signees_per_year / total_real),
            )
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("  {}", rows.iter().take(10).map(|r| r.1.clone()).collect::<Vec<_>>().join(", "));
    println!("  {}", rows.iter().rev().take(5).map(|r| r.1.clone()).collect::<Vec<_>>().join(", "));
    // Fix list topic 2: recruited players from the school's own state (real
    // FBS 2022-25: median 37%, p10 15%, p90 65%; Texas Tech 80%, Texas 68%,
    // LSU 55%, Ohio State 45%, Alabama 43%, Michigan 24%, Oregon 18%, Notre
    // Dame 6%).
    let mut shares: Vec<(f64, &str)> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS && p.plays(w.year))
        .filter_map(|p| {
            let recruited: Vec<_> = w
                .people
                .iter()
                .filter(|q| q.stage == Stage::College && q.college == Some(p.id) && !q.walk_on && q.stars >= 2)
                .collect();
            (recruited.len() >= 20).then(|| {
                let home = recruited.iter().filter(|q| q.home_state == p.state).count();
                (home as f64 / recruited.len() as f64, p.real_name.as_str())
            })
        })
        .collect();
    shares.sort_by(|a, b| a.0.total_cmp(&b.0));
    if !shares.is_empty() {
        let q = |f: f64| 100.0 * shares[((shares.len() - 1) as f64 * f).round() as usize].0;
        println!(
            "FBS recruited players from the school's state: median {:.0}%, p10 {:.0}%, p90 {:.0}% (real 37%, 15%, 65%)",
            q(0.5),
            q(0.1),
            q(0.9)
        );
        let named = [
            "Texas Tech",
            "Texas",
            "LSU",
            "Florida",
            "USC",
            "Georgia",
            "Ohio State",
            "Alabama",
            "Penn State",
            "Clemson",
            "Pittsburgh",
            "Michigan",
            "Oregon",
            "Notre Dame",
        ];
        println!(
            "  {}",
            named
                .iter()
                .filter_map(|n| shares.iter().find(|s| s.1 == *n).map(|s| format!("{n} {:.0}%", 100.0 * s.0)))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // --- National champions ---
    println!("\n## FBS national champions");
    let champs: BTreeMap<i32, String> = w
        .programs
        .iter()
        .flat_map(|p| {
            p.seasons
                .iter()
                .filter(|s| s.national_title && fbs_in(p, s.year))
                .map(move |s| (s.year, format!("{} {}-{}", p.real_name, s.wins, s.losses)))
        })
        .collect();
    println!("{}", champs.iter().map(|(y, n)| format!("{y} {n}")).collect::<Vec<_>>().join("; "));
    // Part B 25: a season with no FBS champion is a bug.
    let first = w.options.start_year - w.options.history_years as i32;
    for y in first..w.year {
        if !champs.contains_key(&y) {
            println!("WARNING: no FBS national champion in {y}");
        }
    }
    for d in [Division::FCS, Division::D2, Division::NAIA, Division::D3] {
        let c: BTreeMap<String, usize> = w
            .programs
            .iter()
            .map(|p| {
                let won = p.seasons.iter().filter(|s| s.national_title && division_in(p, s.year) == d).count();
                (p.real_name.clone(), won)
            })
            .filter(|x| x.1 > 0)
            .collect();
        let mut v: Vec<_> = c.into_iter().collect();
        v.sort_by_key(|b| std::cmp::Reverse(b.1));
        println!(
            "{} titles: {}",
            d.name(),
            v.iter().take(8).map(|(n, k)| format!("{n} {k}")).collect::<Vec<_>>().join(", ")
        );
    }

    // --- Programs ---
    println!("\n## Programs");
    let mut fbs: Vec<usize> = (0..w.programs.len()).filter(|&i| w.programs[i].division == Division::FBS).collect();
    fbs.sort_by(|&a, &b| w.programs[b].prestige.total_cmp(&w.programs[a].prestige));
    println!(
        "top 12 prestige now: {}",
        fbs.iter()
            .take(12)
            .map(|&i| format!(
                "{} {:.2} (was {:.2})",
                w.programs[i].real_name, w.programs[i].prestige, w.programs[i].prestige_start
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut change = fbs.clone();
    change.sort_by(|&a, &b| {
        let da = w.programs[a].prestige - w.programs[a].prestige_start;
        let db = w.programs[b].prestige - w.programs[b].prestige_start;
        db.total_cmp(&da)
    });
    println!("\nBiggest riser and biggest faller:");
    program_story(&w, change[0]);
    program_story(&w, *change.last().unwrap());
    for name in ["Alabama", "Nebraska", "Indiana", "James Madison"] {
        if let Some(i) = w.programs.iter().position(|p| p.real_name == name) {
            program_story(&w, i);
        }
    }
    // Doormats (trial 3 item 11) and dynasties (item 4).
    let mut droughts: Vec<(usize, &str)> = w
        .programs
        .iter()
        .filter(|p| p.power_four())
        .map(|p| {
            let (mut run, mut best) = (0usize, 0usize);
            for s in &p.seasons {
                run = if s.wins > s.losses { 0 } else { run + 1 };
                best = best.max(run);
            }
            (best, p.real_name.as_str())
        })
        .collect();
    droughts.sort_by_key(|d| std::cmp::Reverse(d.0));
    // FBS programs with no winning season in the last 15 recorded years (trial 4 topic 4).
    let dry: Vec<&str> = fbs
        .iter()
        .map(|&i| &w.programs[i])
        .filter(|p| {
            let last: Vec<_> = p.seasons.iter().rev().take(15).collect();
            last.len() == 15 && last.iter().all(|s| s.wins <= s.losses)
        })
        .map(|p| p.real_name.as_str())
        .collect();
    println!("\nFBS programs with no winning season in the last 15 years: {} ({})", dry.len(), dry.join(", "));
    println!(
        "\nLongest Power 4 runs without a winning season: {}",
        droughts.iter().take(6).map(|(n, name)| format!("{name} {n}")).collect::<Vec<_>>().join(", ")
    );
    let mut titles: Vec<(usize, &str)> = w
        .programs
        .iter()
        .map(|p| (p.seasons.iter().filter(|s| s.national_title && fbs_in(p, s.year)).count(), p.real_name.as_str()))
        .filter(|t| t.0 > 0)
        .collect();
    titles.sort_by_key(|t| std::cmp::Reverse(t.0));
    println!(
        "Most FBS titles: {}",
        titles.iter().take(6).map(|(n, name)| format!("{name} {n}")).collect::<Vec<_>>().join(", ")
    );
    // Why the top program kept winning: its roster strength against the rest of FBS.
    if let Some(&(_, top)) = titles.first()
        && let Some(i) = w.programs.iter().position(|p| p.real_name == top)
    {
        program_story(&w, i);
        let mut line = String::new();
        for s in &w.programs[i].seasons {
            let mut others: Vec<f64> = w
                .programs
                .iter()
                .enumerate()
                .filter(|(j, p)| *j != i && p.division == Division::FBS)
                .filter_map(|(_, p)| p.seasons.iter().find(|t| t.year == s.year).map(|t| t.strength))
                .collect();
            others.sort_by(|a, b| b.total_cmp(a));
            let rank = others.iter().filter(|&&o| o > s.strength).count() + 1;
            let coach = s.coach.map_or(String::new(), |c| w.coaches[c].name.clone());
            line.push_str(&format!(
                "{} {:.1} (#{rank}, next best {:.1}, HC {coach}){}; ",
                s.year,
                s.strength,
                others.first().copied().unwrap_or(0.0),
                if s.national_title { " *N" } else { "" }
            ));
        }
        println!("starter strength by year: {line}");
    }
    if let Some(i) = w.programs.iter().position(|p| p.real_name == "Mississippi State") {
        program_story(&w, i);
    }
    // Back-to-back conference titles and the title chances grade.
    println!("\nTitle chances grade after back-to-back conference titles (first five cases):");
    let mut shown = 0;
    'outer: for p in w.programs.iter().filter(|p| p.division == Division::FBS) {
        for k in 2..p.seasons.len() {
            let (a, b, before) = (&p.seasons[k - 1], &p.seasons[k], &p.seasons[k - 2]);
            if a.conference_title && b.conference_title && !before.conference_title {
                let g = |y: i32| {
                    p.grade_history.iter().find(|h| h.0 == y).map(|h| letter(h.1[GradeCat::TitleChances as usize]))
                };
                println!(
                    "- {}: {} before, {} after one title, {} after two",
                    p.real_name,
                    g(before.year).unwrap_or("?"),
                    g(a.year).unwrap_or("?"),
                    g(b.year).unwrap_or("?")
                );
                shown += 1;
                if shown >= 5 {
                    break 'outer;
                }
                continue 'outer;
            }
        }
    }
    // The NIL collective with the most warnings.
    let mut warn: BTreeMap<usize, usize> = BTreeMap::new();
    for n in w.news.iter().filter(|n| n.channel == Channel::News && matches!(n.kind, NewsKind::NilWarning { .. })) {
        *warn.entry(n.program.unwrap()).or_default() += 1;
    }
    let total_warned = warn.len();
    if let Some((&i, &k)) = warn.iter().filter(|x| w.programs[*x.0].division == Division::FBS).max_by_key(|x| x.1) {
        println!("\nNIL: {total_warned} programs got warnings in history. Most warned FBS program ({k} years):");
        program_story(&w, i);
        let examples: Vec<&str> = w
            .news
            .iter()
            .filter(|n| n.program == Some(i) && matches!(n.kind, NewsKind::NilWarning { .. }))
            .take(3)
            .map(|n| n.text.as_str())
            .collect();
        println!("sample warnings: {examples:?}");
    }

    // --- Coaches ---
    println!("\n## Coaches");
    let mut hcs: Vec<usize> =
        w.programs.iter().filter(|p| p.division == Division::FBS).filter_map(|p| p.head_coach).collect();
    hcs.sort_by(|&a, &b| w.coaches[b].reputation.total_cmp(&w.coaches[a].reputation));
    println!("Top 3 FBS head coaches by reputation:");
    for &c in hcs.iter().take(3) {
        coach_career(&w, c);
    }
    // Over- and underrated: biggest gaps between reputation and hidden value.
    let gap = |c: usize| w.coaches[c].reputation - debug::coach_ability(&w.coaches[c]).head_coach_value();
    let mut by_gap = hcs.clone();
    by_gap.sort_by(|&a, &b| gap(b).total_cmp(&gap(a)));
    println!("Most overrated FBS head coach:");
    coach_career(&w, by_gap[0]);
    println!("Most underrated FBS head coach:");
    coach_career(&w, *by_gap.last().unwrap());
    // An FCS head coach who won titles and moved up (the Cignetti path).
    println!("FCS head coaches with titles who moved up to FBS head jobs:");
    let mut k = 0;
    for c in &w.coaches {
        let fcs_titles = c.history.iter().any(|s| {
            s.role == Role::HeadCoach
                && s.titles > 0
                && matches!(s.employer, Employer::College(p) if w.programs[p].division == Division::FCS)
        });
        let now_fbs = c.job.is_some_and(|j| {
            j.role == Role::HeadCoach
                && matches!(j.employer, Employer::College(p) if w.programs[p].division == Division::FBS)
        });
        if fcs_titles && now_fbs {
            k += 1;
            if k <= 2 {
                coach_career(&w, c.id);
            }
        }
    }
    println!("({k} in total)");
    // Recency bias: fired after a short tenure.
    let quick: Vec<usize> = w
        .coaches
        .iter()
        .filter(|c| {
            c.history.iter().any(|s| {
                s.ended.starts_with("fired")
                    && s.role == Role::HeadCoach
                    && s.to - s.from <= 2
                    && matches!(s.employer, Employer::College(p) if w.programs[p].division == Division::FBS)
            })
        })
        .map(|c| c.id)
        .collect();
    println!("FBS head coaches fired within two seasons: {}. Example:", quick.len());
    if let Some(&c) = quick.last() {
        coach_career(&w, c);
    }
    // Coaching trees.
    let mut tree: BTreeMap<usize, usize> = BTreeMap::new();
    for c in &w.coaches {
        if let (Some(m), Some(j)) = (c.mentor, c.job)
            && j.role == Role::HeadCoach
            && matches!(j.employer, Employer::College(_))
        {
            *tree.entry(m).or_default() += 1;
        }
    }
    if let Some((&m, &n)) = tree.iter().max_by_key(|x| x.1) {
        println!("Biggest coaching tree: {n} current college head coaches came up under {}", w.coaches[m].name);
        coach_career(&w, m);
    }
    // High school hiring: network vs teaching by school prestige.
    let (mut small, mut big) = ((0.0, 0.0, 0), (0.0, 0.0, 0));
    let mut ex_pros_small = 0;
    for s in &w.high_schools {
        if let Some(c) = s.coach {
            let a = debug::coach_ability(&w.coaches[c]);
            let teach = a.teaching.iter().sum::<f64>() / 8.0;
            let bucket = if s.prestige < 0.3 {
                &mut small
            } else if s.prestige >= 0.75 {
                &mut big
            } else {
                continue;
            };
            bucket.0 += a.network;
            bucket.1 += teach;
            bucket.2 += 1;
            if s.prestige < 0.3 && w.coaches[c].ex_pro {
                ex_pros_small += 1;
            }
        }
    }
    println!(
        "High school head coaches: small programs network {:.1} / teaching {:.1} (n {}, {} ex-pros); powerhouses network {:.1} / teaching {:.1} (n {})",
        small.0 / small.2.max(1) as f64,
        small.1 / small.2.max(1) as f64,
        small.2,
        ex_pros_small,
        big.0 / big.2.max(1) as f64,
        big.1 / big.2.max(1) as f64,
        big.2
    );

    coaching_market(&w, years);
    recruiting_spread(&w);

    // --- Players ---
    println!("\n## Players");
    let recorded: Vec<&Alumnus> = w.alumni.iter().filter(|a| a.class_of > w.year - years as i32 + 4).collect();
    let pick_round = |a: &Alumnus| a.pick.map(|p| afm_engine::career::draft_round(p, 257));
    let hidden_gem = recorded.iter().find(|a| {
        a.stars <= 2
            && pick_round(a).is_some_and(|r| r <= 2)
            && a.colleges.first().is_some_and(|&c| w.programs[c].division != Division::FBS)
    });
    println!("Hidden gem (two stars or fewer, started below FBS, drafted in rounds 1-2):");
    if let Some(a) = hidden_gem {
        show_player(&w, a);
    }
    println!(
        "  (all such gems: {})",
        recorded
            .iter()
            .filter(|a| a.stars <= 2
                && pick_round(a).is_some_and(|r| r <= 2)
                && a.colleges.first().is_some_and(|&c| w.programs[c].division != Division::FBS))
            .count()
    );
    println!("JUCO to the draft:");
    if let Some(a) = recorded.iter().filter(|a| a.juco && a.pick.is_some()).min_by_key(|a| a.pick) {
        show_player(&w, a);
    }
    println!("Steered to a school by family, then transferred out:");
    if let Some(a) = recorded.iter().find(|a| a.steered && a.notes.iter().any(|n| n.contains("free at last"))) {
        show_player(&w, a);
    }
    println!(
        "  (steered recruits on record: {}, of whom transferred out once free: {})",
        recorded.iter().filter(|a| a.steered).count(),
        recorded.iter().filter(|a| a.steered && a.notes.iter().any(|n| n.contains("free at last"))).count()
    );
    println!("Five-star busts (left football):");
    for a in recorded.iter().filter(|a| a.stars == 5 && !a.position.is_specialist() && a.left_football).take(2) {
        show_player(&w, a);
    }
    println!("Powerhouse transfer who was drafted:");
    if let Some(a) = recorded.iter().find(|a| a.pick.is_some() && a.notes.iter().any(|n| n.contains("to powerhouse"))) {
        show_player(&w, a);
    }
    println!("Viral clip:");
    if let Some(a) = recorded.iter().find(|a| a.notes.iter().any(|n| n.contains("went viral"))) {
        show_player(&w, a);
    }
    println!("Top pick of the last draft:");
    if let Some(a) = w.alumni.iter().filter(|a| a.pick == Some(1)).max_by_key(|a| a.year) {
        show_player(&w, a);
    }

    trial1_notes(&w, years);
    stat_book(&w);
    walk_on_picks(&w);
    nfl_league(&w, years);

    // --- Character report: what the game shows instead ---
    println!("\n## What a scout sees (character report on a current five-star)");
    if let Some(i) = w.people.iter().position(|p| p.stars == 5 && !p.player.position.is_specialist()) {
        let name = w.people[i].name.clone();
        for acc in [6.0, 16.0] {
            let r = w.people[i].character_report(acc);
            println!("- {name}, scout judging {acc}: {}", r.join(" "));
        }
        let (bg, fam) = debug::person_background(&w.people[i]);
        println!(
            "  (truth: money {:.2}, safety {:.2}, stability {:.0}, decider {:?}, parent {:?})",
            bg.money, bg.safety, bg.stability, fam.decider, fam.parent_style
        );
    }
}

/// Fix list 5.7: walk-ons develop under the same model, so some are drafted.
/// Real: about 25% of picks were unrated or unmatched to a recruiting
/// record (walk-ons, JUCO, prep schools, name mismatches; trial 3 baselines).
fn walk_on_picks(w: &World) {
    let picks: Vec<&Alumnus> = w.alumni.iter().filter(|a| a.pick.is_some()).collect();
    let n = picks.len().max(1) as f64;
    let unrated = picks.iter().filter(|a| a.stars == 0).count();
    let walked = picks.iter().filter(|a| a.notes.iter().any(|x| x.starts_with("Walked on at"))).count();
    let juco = picks.iter().filter(|a| a.juco).count();
    println!("\n## Walk-ons and unrated players in the draft (fix list 5.7)");
    println!(
        "{} picks: unrated {:.1}% (real about 25% with JUCO, prep and mismatches), walked on {:.1}% ({walked}), JUCO {:.1}%",
        picks.len(),
        100.0 * unrated as f64 / n,
        100.0 * walked as f64 / n,
        100.0 * juco as f64 / n
    );
}

/// The career stats book (section 3, topic 8): the top of each FBS record
/// book with the era view, footprints by division, the Hall of Fame and
/// the coaches with the most buzz from their units.
fn stat_book(w: &World) {
    use afm_engine::world::records::{Book, RECORD_STATS, STREAKS, Scope, TESTING};
    let b = &w.stats;
    println!("\n## The career stats book (section 3, topic 8)");
    println!(
        "{} player seasons kept over {} seasons; {} records broken; {} Hall of Fame inductees",
        b.seasons.len(),
        b.year_index.len(),
        b.broken.len(),
        b.hall.len()
    );
    for d in [Division::FBS, Division::FCS, Division::D2, Division::D3] {
        println!("### {} record books (best three)", d.name());
        let mut books: Vec<Book> = (0..RECORD_STATS.len()).flat_map(|k| [Book::Season(k), Book::Career(k)]).collect();
        books.extend((0..STREAKS.len()).map(Book::Streak));
        books.push(Book::WinStreak);
        for bk in books {
            let Some(list) = b.books.get(&(Scope::Division(d), bk)) else { continue };
            let row: Vec<String> = list
                .iter()
                .take(3)
                .map(|e| {
                    let era = if let Book::Season(k) = bk {
                        b.career(e.id)
                            .into_iter()
                            .find(|r| r.year as i32 == e.year && r.division == d)
                            .map(|r| format!(", era-adjusted {:.0}", b.era_adjusted(r, RECORD_STATS[k])))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    let years = if e.from != e.year { format!("{}-{}", e.from, e.year) } else { e.year.to_string() };
                    format!(
                        "{:.0} {} ({}, {years}{}{era})",
                        e.value,
                        e.name,
                        w.programs[e.program].real_name,
                        if e.live { ", running" } else { "" }
                    )
                })
                .collect();
            println!("- {}: {}", bk.name(), row.join("; "));
        }
    }
    // High school winning streaks across the states (real: De La Salle's 151, 1992-2004).
    let mut hs: Vec<_> = b
        .books
        .iter()
        .filter(|(k, _)| matches!(k.0, Scope::State(_)))
        .flat_map(|(k, v)| v.iter().map(move |e| (k.0.clone(), e)))
        .collect();
    // Ties by name: the books are a hash map, read in no set order.
    hs.sort_by(|a, b| b.1.value.total_cmp(&a.1.value).then_with(|| a.1.name.cmp(&b.1.name)));
    let row: Vec<String> = hs
        .iter()
        .take(5)
        .map(|(sc, e)| {
            let st = if let Scope::State(s) = sc { afm_engine::world::geo::STATE_CENTERS[*s].0 } else { "" };
            format!("{:.0} {} {st} {}-{}{}", e.value, e.name, e.from, e.year, if e.live { " running" } else { "" })
        })
        .collect();
    println!("### Longest high school winning streaks: {}", row.join("; "));
    println!("### Hidden-gem footprints by season (flagged seasons: stats / testing)");
    for &(year, _) in b.year_index.iter().rev().take(5) {
        let recs = b.year(year);
        let line: Vec<String> = Division::ALL
            .iter()
            .map(|&d| {
                let stats = recs.iter().filter(|r| r.division == d && r.footprint & !TESTING != 0).count();
                let test = recs.iter().filter(|r| r.division == d && r.footprint & TESTING != 0).count();
                format!("{} {stats}/{test}", d.name())
            })
            .collect();
        println!("- {year}: {}", line.join(", "));
    }
    // Footprinted small-school players and the draft.
    let flagged: Vec<u32> = {
        let mut v: Vec<u32> = b
            .seasons
            .iter()
            .filter(|r| r.division != Division::FBS && r.footprint & !TESTING != 0)
            .map(|r| r.id)
            .collect();
        v.sort();
        v.dedup();
        v
    };
    println!("{} small-school players with a stats footprint", flagged.len());
    println!("### College Football Hall of Fame (last three classes)");
    let last: Vec<i32> = {
        let mut y: Vec<i32> = b.hall.iter().map(|h| h.year).collect();
        y.sort();
        y.dedup();
        y.into_iter().rev().take(3).collect()
    };
    for y in last {
        for h in b.hall.iter().filter(|h| h.year == y) {
            let progs: Vec<&str> = h.programs.iter().map(|&p| w.programs[p].real_name.as_str()).collect();
            println!(
                "- {y}: {}{} ({}, {}): {}",
                h.name,
                if h.coach { ", coach" } else { "" },
                h.division.name(),
                progs.join(", "),
                h.why
            );
        }
    }
    println!("### Coach buzz from unit stats (last season, biggest gains)");
    let last_year = b.buzz.iter().map(|x| x.year).max().unwrap_or(0);
    let mut top: Vec<_> = b.buzz.iter().filter(|x| x.year == last_year).collect();
    top.sort_by(|a, b| b.gain.total_cmp(&a.gain));
    for x in top.iter().take(8) {
        println!(
            "- {} ({}): +{:.2}, {}; buzz now {:.2}",
            w.coaches[x.coach].name, w.programs[x.program].real_name, x.gain, x.why, w.coaches[x.coach].stat_buzz
        );
    }
}

/// Fix list topic 1 checks: kickers, punters and long snappers on every
/// roster, scholarships against walk-ons, position counts against the real
/// FBS table, and (with the game log) kicking in the last engine season.
fn special_teams(w: &World) {
    use afm_engine::positions::{POSITION_COUNT, Position};
    println!("## Kickers, punters and roster needs (fix list topic 1)");
    println!("Rosters going into {} (after signing day):", w.year);
    // Real FBS 2022-24 per roster, from the fix list table.
    let real = [5.8, 8.5, 16.2, 7.4, 9.36, 10.14, 9.1, 8.4, 14.6, 10.2, 12.0, 3.5, 2.0, 2.5];
    for d in Division::ALL {
        let teams: Vec<usize> =
            w.programs.iter().filter(|p| p.division == d && !p.roster.is_empty()).map(|p| p.id).collect();
        if teams.is_empty() {
            continue;
        }
        // Counted from the people themselves: the program roster lists hold
        // stale indices between the year-end clean-up and the next rebuild.
        let mut counts = vec![[0u32; POSITION_COUNT]; w.programs.len()];
        let mut sch = [0u32; POSITION_COUNT];
        for p in &w.people {
            let Some(t) = p.college else { continue };
            if p.stage != Stage::College || w.programs[t].division != d {
                continue;
            }
            let k = p.player.position.index();
            counts[t][k] += 1;
            if !p.walk_on {
                sch[k] += 1;
            }
        }
        let mut sizes: Vec<u32> = teams.iter().map(|&t| counts[t].iter().sum()).collect();
        sizes.sort();
        let n = teams.len() as f64;
        let none = |pos: Position| teams.iter().filter(|&&t| counts[t][pos.index()] == 0).count();
        let per = |pos: Position| teams.iter().map(|&t| counts[t][pos.index()]).sum::<u32>() as f64 / n;
        let range = |pos: Position| {
            let v: Vec<u32> = teams.iter().map(|&t| counts[t][pos.index()]).collect();
            (v.iter().copied().min().unwrap_or(0), v.iter().copied().max().unwrap_or(0))
        };
        println!(
            "- {}: {} teams, roster median {} ({}-{}); no kicker {}, no punter {}, no long snapper {}, neither K nor P {}",
            d.name(),
            teams.len(),
            sizes[sizes.len() / 2],
            sizes[sizes.len() / 10],
            sizes[sizes.len() * 9 / 10],
            none(Position::K),
            none(Position::P),
            none(Position::LS),
            teams
                .iter()
                .filter(|&&t| counts[t][Position::K.index()] == 0 && counts[t][Position::P.index()] == 0)
                .count()
        );
        for pos in [Position::K, Position::P, Position::LS] {
            let (lo, hi) = range(pos);
            let k = pos.index();
            let total = per(pos) * n;
            println!(
                "  {}: {:.2} a roster ({lo}-{hi}), on scholarship {:.2} a team, walk-ons {:.0}%",
                pos.name(),
                per(pos),
                sch[k] as f64 / n,
                100.0 * (1.0 - sch[k] as f64 / total.max(1.0))
            );
        }
        if d == Division::FBS {
            let scale = 105.0 / 119.7;
            let line: Vec<String> = Position::ALL
                .iter()
                .map(|&pos| format!("{} {:.1} ({:.1})", pos.name(), per(pos), real[pos.index()] * scale))
                .collect();
            println!("  per roster against the real table scaled to 105: {}", line.join(", "));
        }
    }
    println!(
        "Real FBS 2022-24: K 3.5 (1-7, never 0), P 2.0 (5% none), LS 2.5; no recruiting record K 69%, P 65%, LS 82%."
    );
    // Kicking on the engine, last season with a game log.
    let Some(last) = w.game_log.games.iter().map(|g| g.year).max() else {
        println!("(no game log kept: run with AFM_DASH=path for the engine checks)\n");
        return;
    };
    for d in [Division::FBS, Division::FCS] {
        let gs: Vec<_> = w.game_log.games.iter().filter(|g| g.year == last && g.division == d).collect();
        let tg = 2.0 * gs.len() as f64;
        if gs.is_empty() {
            continue;
        }
        let sum = |f: &dyn Fn(&afm_engine::world::game_log::GameSummary, usize) -> f64| -> f64 {
            gs.iter().map(|g| f(g, 0) + f(g, 1)).sum::<f64>()
        };
        let fga = sum(&|g, s| g.stats[s].fg_att as f64);
        let fgm = sum(&|g, s| g.stats[s].fg_made as f64);
        let xp = sum(&|g, s| g.xp[s] as f64);
        let two = sum(&|g, s| g.two_point[s] as f64);
        let punts = sum(&|g, s| g.punt_yds[s].0 as f64);
        let punt_yds = sum(&|g, s| g.punt_yds[s].1);
        println!(
            "{} {last} on the engine, per team game: field goal tries {:.2} (made {:.0}%), extra point kicks {:.2}, two-point tries {:.2}, punts {:.2} at {:.1} gross",
            d.name(),
            fga / tg,
            100.0 * fgm / fga.max(1.0),
            xp / tg,
            two / tg,
            punts / tg,
            punt_yds / punts.max(1.0)
        );
        // Who kicked: field goal tries by listed position.
        let mut by_pos: BTreeMap<&str, u32> = BTreeMap::new();
        for (&(y, _), s) in &w.game_log.players {
            if y == last && s.fg_att > 0 && w.programs[s.program].division == d {
                *by_pos.entry(s.pos.map_or("?", |p| p.name())).or_default() += s.fg_att;
            }
        }
        println!("  field goal tries by the kicker's listed position: {by_pos:?}");
    }
    println!("Real FBS 2022-24 per team game: field goal tries 1.56, punt gross 42.3 (FCS 40.0).\n");
}

/// Checks for the eight trial 1 notes, with the facts behind each.
fn trial1_notes(w: &World, years: u32) {
    use afm_engine::world::coach::Group;
    use afm_engine::world::ethnicity::Ethnicity;
    let first = w.year - years as i32;
    println!("\n## Trial 1 notes");

    // 1. Prove-it extensions.
    println!("\n### 1. Prove-it extensions");
    let ext: Vec<_> = w.news.iter().filter(|n| n.kind == NewsKind::CoachExtended).collect();
    let failed =
        w.coaches.iter().flat_map(|c| c.history.iter()).filter(|s| s.ended == "fired after his prove-it year").count();
    println!("extensions in history: {} (of those, fired after the prove-it year: {failed})", ext.len());
    for n in ext.iter().take(3) {
        println!("- {} {}", n.year, n.text);
    }
    if let Some(n) = ext.first() {
        let p = n.program.unwrap();
        let name = n.text.split("head coach ").nth(1).and_then(|t| t.split(" a contract").next()).unwrap_or("");
        if let Some(c) = w.coaches.iter().position(|c| c.name == name) {
            coach_career(w, c);
        }
        let s: Vec<String> = w.programs[p]
            .seasons
            .iter()
            .filter(|s| (n.year - 3..=n.year + 2).contains(&s.year))
            .map(|s| format!("{} {}-{}", s.year, s.wins, s.losses))
            .collect();
        println!("  {} around it: {}", w.programs[p].real_name, s.join(", "));
    }

    // 2. Ethnicity and the coaching gap.
    println!("\n### 2. Ethnicity and the coaching gap");
    for y in [first, 2009, 2016, w.year - 1] {
        if let Some(n) = w.news.iter().find(|n| n.kind == NewsKind::DiversityReport && n.year == y) {
            println!("- {y}: {}", n.text);
        }
    }
    let mut roles: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for p in w.programs.iter().filter(|p| p.division == Division::FBS) {
        let mut add = |k: &'static str, c: Option<usize>| {
            if let Some(c) = c {
                let e = roles.entry(k).or_default();
                e.1 += 1;
                if w.coaches[c].ethnicity != Ethnicity::White {
                    e.0 += 1;
                }
            }
        };
        add("head coaches", p.head_coach);
        add("coordinators", p.oc);
        add("coordinators", p.dc);
        for &pc in &p.position_coaches {
            add("position coaches", pc);
        }
    }
    for (k, (c, n)) in &roles {
        println!("FBS {k} of color now: {c} of {n} ({:.0}%)", 100.0 * *c as f64 / (*n).max(1) as f64);
    }
    let mut fbs = [0usize; 5];
    for p in w.people.iter() {
        if p.stage == afm_engine::world::Stage::College
            && p.college.is_some_and(|c| w.programs[c].division == Division::FBS)
        {
            fbs[p.ethnicity as usize] += 1;
        }
    }
    let tot: usize = fbs.iter().sum();
    println!(
        "FBS players now: {}",
        Ethnicity::ALL
            .iter()
            .map(|&e| format!("{} {:.1}%", e.name(), 100.0 * fbs[e as usize] as f64 / tot.max(1) as f64))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut hs = [0usize; 5];
    for p in w.people.iter().filter(|p| p.stage == afm_engine::world::Stage::HighSchool) {
        hs[p.ethnicity as usize] += 1;
    }
    let th: usize = hs.iter().sum();
    println!(
        "High school players now: {}",
        Ethnicity::ALL
            .iter()
            .map(|&e| format!("{} {:.1}%", e.name(), 100.0 * hs[e as usize] as f64 / th.max(1) as f64))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // 3. Names.
    println!("\n### 3. Names");
    for e in Ethnicity::ALL {
        let names: Vec<&str> = w
            .people
            .iter()
            .filter(|p| p.ethnicity == e && p.stage == afm_engine::world::Stage::College)
            .take(8)
            .map(|p| p.name.as_str())
            .collect();
        println!("- {}: {}", e.name(), names.join(", "));
    }
    let white: Vec<&str> =
        w.people.iter().filter(|p| p.ethnicity == Ethnicity::White).map(|p| p.name.as_str()).collect();
    for first_name in ["Jalen", "Jaylen", "Kobe", "Trey", "DeShawn", "Wyatt"] {
        let n = white.iter().filter(|n| n.split(' ').next() == Some(first_name)).count();
        let all = w.people.iter().filter(|p| p.name.split(' ').next() == Some(first_name)).count();
        println!("  {first_name}: {all} players, {n} of them white");
    }
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for p in w.people.iter() {
        *seen.entry(p.name.as_str()).or_default() += 1;
    }
    let shared = w.people.iter().filter(|p| seen[p.name.as_str()] > 1).count();
    println!(
        "players sharing a full name with another active player: {:.1}% of {}",
        100.0 * shared as f64 / w.people.len() as f64,
        w.people.len()
    );

    // 4. Pipelines.
    println!("\n### 4. Pipelines");
    let with_hs = w.programs.iter().filter(|p| p.hs_pipeline.values().any(|v| v.0 >= 0.1)).count();
    println!("programs with a high school pipeline of 0.10+: {with_hs} of {}", w.programs.len());
    for name in ["Bowling Green", "Toledo", "Georgia", "North Dakota State"] {
        let Some(i) = w.programs.iter().position(|p| p.real_name == name) else { continue };
        let p = &w.programs[i];
        let mut st: Vec<(f64, &str)> =
            (0..51).filter(|&s| p.pipeline[s] > 0.05).map(|s| (p.pipeline[s], STATE_CENTERS[s].0)).collect();
        st.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut hsl: Vec<(f64, String)> = p
            .hs_pipeline
            .iter()
            .map(|(&h, v)| (v.0, format!("{} ({})", w.high_schools[h].name, STATE_CENTERS[w.high_schools[h].state].0)))
            .collect();
        hsl.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut ar: Vec<(f64, String)> = p
            .area_pipeline
            .iter()
            .map(|(&a, v)| (v.0, format!("{} area ({})", w.areas[a].1, STATE_CENTERS[w.areas[a].0].0)))
            .collect();
        ar.sort_by(|a, b| b.0.total_cmp(&a.0));
        println!(
            "- {name} (prestige {:.2} -> {:.2}): states {} | schools {} | areas {}",
            p.prestige_start,
            p.prestige,
            st.iter().take(4).map(|(v, s)| format!("{s} {v:.2}")).collect::<Vec<_>>().join(", "),
            hsl.iter().take(3).map(|(v, s)| format!("{s} {v:.2}")).collect::<Vec<_>>().join(", "),
            ar.iter().take(3).map(|(v, s)| format!("{s} {v:.2}")).collect::<Vec<_>>().join(", ")
        );
    }
    // Where the MAC's impact players came from.
    let mut mac: BTreeMap<&str, usize> = BTreeMap::new();
    for a in w.alumni.iter().filter(|a| a.pick.is_some()) {
        if a.colleges.first().is_some_and(|&c| w.programs[c].conference == "MAC") {
            *mac.entry(STATE_CENTERS[a.home_state].0).or_default() += 1;
        }
    }
    let mut mv: Vec<_> = mac.into_iter().collect();
    mv.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!("MAC draft picks by home state: {mv:?}");

    // 5. The coaching ladder.
    println!("\n### 5. The coaching ladder");
    let mut moves: BTreeMap<String, usize> = BTreeMap::new();
    let rung = |e: Employer, r: Role| -> String {
        let lvl = match e {
            Employer::HighSchool(_) => "HS".to_string(),
            Employer::College(p) => w.programs[p].division.name().to_string(),
            Employer::Nfl(_) => "NFL".to_string(),
        };
        let role = match r {
            Role::HeadCoach => "head coach",
            Role::OffensiveCoordinator | Role::DefensiveCoordinator => "coordinator",
            Role::PositionCoach(_) => "position coach",
        };
        format!("{lvl} {role}")
    };
    let mut example: Option<usize> = None;
    for c in &w.coaches {
        let mut jobs: Vec<(Employer, Role)> = c.history.iter().map(|s| (s.employer, s.role)).collect();
        if let Some(j) = c.job {
            jobs.push((j.employer, j.role));
        }
        for pair in jobs.windows(2) {
            let (a, b) = (rung(pair[0].0, pair[0].1), rung(pair[1].0, pair[1].1));
            if a != b {
                *moves.entry(format!("{a} -> {b}")).or_default() += 1;
            }
        }
        let from_hs = jobs.first().is_some_and(|j| matches!(j.0, Employer::HighSchool(_)));
        let to_fbs_hc = c.job.is_some_and(|j| {
            j.role == Role::HeadCoach
                && matches!(j.employer, Employer::College(p) if w.programs[p].division == Division::FBS)
        });
        if from_hs && to_fbs_hc && example.is_none() {
            example = Some(c.id);
        }
    }
    let mut mv: Vec<_> = moves.into_iter().filter(|m| !m.0.starts_with("HS head coach -> HS")).collect();
    mv.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (m, n) in mv.iter().take(14) {
        println!("- {m}: {n}");
    }
    if let Some(c) = example {
        println!("From a high school job to an FBS head job:");
        coach_career(w, c);
    }
    let hs_hires: Vec<_> =
        w.news.iter().filter(|n| n.kind == NewsKind::CoachHired && n.text.contains("High School football")).collect();
    println!("college hires straight from high school: {}", hs_hires.len());
    for n in hs_hires.iter().rev().take(3) {
        println!("- {} {}", n.year, n.text);
    }

    // 6. Academics.
    println!("\n### 6. Academics");
    let recorded: Vec<&Alumnus> = w.alumni.iter().filter(|a| a.class_of > first + 4).collect();
    let count = |pat: &str| recorded.iter().filter(|a| a.notes.iter().any(|n| n.contains(pat))).count();
    println!(
        "on record ({} drafted or blue-chip players): academic redshirts {}, special admissions {}, JUCO for grades {}, lifted by summer school {}, ineligible seasons {}",
        recorded.len(),
        count("academic redshirt"),
        count("special admissions"),
        count("under the NCAA's 2.0 floor, so four-year"),
        count("summer school"),
        count("Academically ineligible")
    );
    let five_juco = recorded.iter().filter(|a| a.stars == 5 && !a.position.is_specialist() && a.juco).count();
    let five = recorded.iter().filter(|a| a.stars == 5 && !a.position.is_specialist()).count();
    println!("five-stars who went through JUCO: {five_juco} of {five}");
    if let Some(a) = recorded.iter().find(|a| a.notes.iter().any(|n| n.contains("special admissions"))) {
        show_player(w, a);
    }
    if let Some(a) = recorded.iter().find(|a| a.stars >= 4 && a.notes.iter().any(|n| n.contains("academic redshirt"))) {
        show_player(w, a);
    }

    // 7. The draft by position.
    println!("\n### 7. The draft by position (sim vs real 2000-2025)");
    let drafted: Vec<&Alumnus> = w.alumni.iter().filter(|a| a.pick.is_some() && a.year > first + 4).collect();
    // Built in, so the report finds it from any folder (trial 7 finding 17).
    let real = include_str!("../../../../data/real-world/draft_positions_2000_2025.csv");
    for (band, cut) in [("pick_1", 1u32), ("picks_1_5", 5), ("round_1", 32)] {
        let sel: Vec<&&Alumnus> = drafted.iter().filter(|a| a.pick.unwrap() <= cut).collect();
        let mut by: BTreeMap<&str, usize> = BTreeMap::new();
        for a in &sel {
            *by.entry(a.position.name()).or_default() += 1;
        }
        let mut v: Vec<_> = by.into_iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        println!(
            "- {band}: sim {}",
            v.iter()
                .map(|(p, n)| format!("{p} {:.0}%", 100.0 * *n as f64 / sel.len().max(1) as f64))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if let Some(line) = real.lines().find(|l| l.starts_with(band)) {
            let head: Vec<&str> = real.lines().next().unwrap().split(',').collect();
            let vals: Vec<&str> = line.split(',').collect();
            let mut r: Vec<(f64, &str)> = (3..head.len())
                .filter_map(|k| vals[k].parse::<f64>().ok().map(|x| (x, head[k].trim_start_matches("pct_"))))
                .filter(|x| x.0 > 0.0)
                .collect();
            r.sort_by(|a, b| b.0.total_cmp(&a.0));
            println!("  real {}", r.iter().map(|(x, p)| format!("{p} {x:.0}%")).collect::<Vec<_>>().join(", "));
        }
    }
    if let Some(a) = w.alumni.iter().filter(|a| a.pick == Some(1)).max_by_key(|a| a.year) {
        println!("last number 1 pick: {} ({}), {}", a.name, a.position.name(), a.notes.last().unwrap());
    }

    // 8. Measurables and combine.
    println!("\n### 8. Measurables and combine (drafted players, sim vs real combine averages)");
    let combine = include_str!("../../../../data/real-world/combine_by_position.csv");
    for pos in afm_engine::positions::Position::ALL {
        let sel: Vec<&&Alumnus> = drafted.iter().filter(|a| a.position == pos).collect();
        if sel.is_empty() {
            continue;
        }
        let (mut h, mut wt, mut f, mut v) = (0.0, 0.0, 0.0, 0.0);
        let (mut be, mut br, mut co, mut sh) = (0.0, 0.0, 0.0, 0.0);
        for a in &sel {
            let p = a.player.as_ref().unwrap();
            let b = afm_engine::body::body_now(p);
            let mut rng = afm_engine::rng::Rng::new(p.id ^ 0xC0B1);
            let c = afm_engine::body::combine(p, &mut rng);
            h += b.height;
            wt += b.weight;
            f += c.forty;
            v += c.vertical;
            be += c.bench;
            br += c.broad;
            co += c.cone;
            sh += c.shuttle;
        }
        let n = sel.len() as f64;
        let real = combine.lines().find(|l| l.starts_with(&format!("{},", pos.name()))).map(|l| {
            let x: Vec<&str> = l.split(',').collect();
            format!(
                "{}\" {} lb {}s {}\" bench {} broad {} cone {} shuttle {}",
                x[2], x[4], x[6], x[10], x[8], x[12], x[14], x[16]
            )
        });
        println!(
            "- {}: sim {:.1}\" {:.0} lb {:.2}s {:.1}\" bench {:.1} broad {:.0} cone {:.2} shuttle {:.2} | real {}",
            pos.name(),
            h / n,
            wt / n,
            f / n,
            v / n,
            be / n,
            br / n,
            co / n,
            sh / n,
            real.unwrap_or_default()
        );
    }
    let _ = Group::QB;
}

/// AFM_DASH=path.json: everything the trial 3 debug dashboard shows. Hidden
/// values are included; this is debug tooling, never the game.
fn write_dashboard(w: &World, path: &str, seed: u32, years: u32, cards: &[serde_json::Value]) {
    use afm_engine::world::coach::Group;
    use serde_json::json;
    let first = w.year - years as i32;
    let programs: Vec<serde_json::Value> = w
        .programs
        .iter()
        .map(|p| {
            let seasons: Vec<serde_json::Value> = p
                .seasons
                .iter()
                .filter(|s| s.year >= first)
                .map(|s| {
                    let t = &s.totals;
                    json!([s.year, s.wins, s.losses, s.conference_title, s.national_title,
                        (s.strength * 10.0).round() / 10.0,
                        s.coach.map(|c| w.coaches[c].name.clone()),
                        t.games, t.points_for, t.points_against, t.yards.round(), t.pass_yards.round(),
                        t.rush_yards.round(), t.yards_allowed.round(), t.turnovers, t.takeaways, t.sacks, t.sacks_allowed,
                        t.att, t.cmp, t.carries, t.plays])
                })
                .collect();
            let mut pipes: Vec<(f64, &str)> =
                (0..51).filter(|&k| p.pipeline[k] > 0.05).map(|k| (p.pipeline[k], STATE_CENTERS[k].0)).collect();
            pipes.sort_by(|a, b| b.0.total_cmp(&a.0));
            let picks: Vec<u32> = (1..=7).map(|r| p.draft_picks.iter().filter(|d| d.1 == r && d.0 >= first).count() as u32).collect();
            json!({
                "name": p.real_name, "div": p.division.name(), "conf": p.conference, "state": STATE_CENTERS[p.state].0,
                "prestige_start": (p.prestige_start * 100.0).round() / 100.0, "prestige": (p.prestige * 100.0).round() / 100.0,
                "prestige_history": p.prestige_history.iter().filter(|h| h.0 >= first).map(|h| json!([h.0, (h.1 * 100.0).round() / 100.0])).collect::<Vec<_>>(),
                "grades": GradeCat::ALL.iter().map(|&g| json!([g.name(), letter(p.grades[g as usize])])).collect::<Vec<_>>(),
                "grade_history": p.grade_history.iter().filter(|h| h.0 >= first).map(|h| json!([h.0, GradeCat::ALL.iter().map(|&g| letter(h.1[g as usize])).collect::<Vec<_>>()])).collect::<Vec<_>>(),
                "tradition": (p.tradition * 100.0).round() / 100.0, "facilities": (p.facilities * 100.0).round() / 100.0,
                "fans": (p.fans * 100.0).round() / 100.0, "nil": (p.collective.budget / 1e5).round() / 10.0,
                "pipelines": pipes.iter().take(5).map(|(v, k)| json!([k, (v * 100.0).round() / 100.0])).collect::<Vec<_>>(),
                "picks_by_round": picks, "story": p.story.iter().map(|(y, t)| json!([y, t])).collect::<Vec<_>>(),
                "seasons": seasons,
            })
        })
        .collect();
    let stint = |e: Employer, r: Role, from: i32, to: Option<i32>, wins: u32, losses: u32, ended: &str| {
        json!([from, to, r.name(), employer_name(w, e), wins, losses, ended])
    };
    let coaches: Vec<serde_json::Value> = w
        .coaches
        .iter()
        .filter(|c| {
            c.job.is_some_and(|j| match j.employer {
                Employer::College(p) => {
                    matches!(w.programs[p].division, Division::FBS | Division::FCS)
                        && (j.role == Role::HeadCoach || w.programs[p].division == Division::FBS)
                }
                _ => false,
            })
        })
        .map(|c| {
            let a = debug::coach_ability(c);
            let mut career: Vec<serde_json::Value> =
                c.history.iter().map(|s| stint(s.employer, s.role, s.from, Some(s.to), s.wins, s.losses, s.ended)).collect();
            if let Some(j) = c.job {
                career.push(stint(j.employer, j.role, j.since, None, c.wins, c.losses, "current"));
            }
            json!({
                "name": c.name, "age": c.age, "eth": c.ethnicity.name(), "rep": (c.reputation * 10.0).round() / 10.0,
                "role": c.job.map(|j| j.role.name()), "at": c.job.map(|j| employer_name(w, j.employer)),
                "came_up": c.came_up.name(), "ex_pro": c.ex_pro, "mentor": c.mentor.map(|m| w.coaches[m].name.clone()),
                "hidden": {"hc_value": (a.head_coach_value() * 10.0).round() / 10.0, "teaching": a.teaching.iter().map(|t| (t * 10.0).round() / 10.0).collect::<Vec<_>>(),
                    "scheme": a.scheme.round(), "game_mgmt": a.game_management.round(), "motivation": a.motivation.round(),
                    "discipline": a.discipline.round(), "recruiting": a.recruiting.round(), "network": a.network.round(), "relationships": a.relationships.round(),
                    "sportsmanship": debug::coach_sportsmanship(c).round()},
                "units": Group::ALL.iter().map(|&g| { let u = c.units[g as usize]; json!([g.name(), (u.growth_read() * 100.0).round() / 100.0, (u.moved_up * 10.0).round() / 10.0]) }).collect::<Vec<_>>(),
                "career": career,
            })
        })
        .collect();
    let draft: Vec<serde_json::Value> = w
        .alumni
        .iter()
        .filter(|a| a.pick.is_some() && a.year > first)
        .map(|a| {
            json!([
                a.year,
                a.pick,
                a.name,
                a.position.name(),
                a.stars,
                div_name(w, a),
                a.nfl_team,
                a.overall.round(),
                debug::alumnus_potential(a).round(),
                a.ethnicity.name(),
                STATE_CENTERS[a.home_state].0
            ])
        })
        .collect();
    // FBS head coaches by ethnicity, per year, from every coach's stints.
    let mut diversity: BTreeMap<i32, BTreeMap<&str, u32>> = BTreeMap::new();
    for c in &w.coaches {
        // A finished stint covers its seasons up to the year it ended; the
        // current job runs through the last season played.
        let mut spans: Vec<(Employer, Role, i32, i32)> =
            c.history.iter().map(|s| (s.employer, s.role, s.from, s.to - 1)).collect();
        if let Some(j) = c.job {
            spans.push((j.employer, j.role, j.since, w.year - 1));
        }
        for (e, r, from, to) in spans {
            if r == Role::HeadCoach && matches!(e, Employer::College(p) if w.programs[p].division == Division::FBS) {
                for y in from.max(first)..=to.min(w.year - 1) {
                    *diversity.entry(y).or_default().entry(c.ethnicity.name()).or_default() += 1;
                }
            }
        }
    }
    let out = json!({
        "seed": seed, "years": years, "first": first, "last": w.year - 1,
        "programs": programs, "coaches": coaches, "draft": draft, "diversity": diversity, "cards": cards,
        "log": afm_tools::dashboard::extras(w, years),
        "season_cols": ["year","w","l","conf","nat","strength","hc","g","pf","pa","yds","pass","rush","yds_allowed","to","takeaways","sacks","sacks_allowed","att","cmp","carries","plays"],
        "draft_cols": ["year","pick","name","pos","stars","path","team","overall","potential","eth","state"],
    });
    std::fs::write(path, serde_json::to_string(&out).unwrap()).unwrap();
}

/// The coaching market (dynasty item 11.11-11.15): head coach moves per year,
/// NFL hires, early exits, and how long title-winning coaches stay.
/// Dynasty item 11 checks: how 5-stars spread over FBS programs, who enters
/// the portal, and whether transfers start where they land.
fn recruiting_spread(w: &World) {
    use afm_engine::player::PlayingTime;
    println!("\n## Recruiting spread and the portal (dynasty items 11.1-11.3, 11.16-11.18)");
    let log = &w.recruiting_log;
    let mut by_year: BTreeMap<i32, BTreeMap<usize, u32>> = BTreeMap::new();
    for &(y, j) in &log.five_stars {
        *by_year.entry(y).or_default().entry(j).or_default() += 1;
    }
    let (mut share_sum, mut programs_sum, mut n) = (0.0, 0.0, 0.0);
    for (y, m) in &by_year {
        let total: u32 = m.values().sum();
        let Some((&top_j, &top)) = m.iter().max_by_key(|e| *e.1) else { continue };
        let share = top as f64 / total as f64;
        println!(
            "  class {y}: {total} FBS 5-stars, {} programs signed one, top {} with {top} ({:.0}%)",
            m.len(),
            w.program_name(top_j),
            100.0 * share
        );
        share_sum += share;
        programs_sum += m.len() as f64;
        n += 1.0;
    }
    if n > 0.0 {
        println!(
            "  average: top program {:.1}% of FBS 5-stars, {:.1} programs sign one (real 2024-25, 247Sports: 14% and 15-17)",
            100.0 * share_sum / n,
            programs_sum / n
        );
    }
    // Portal entrants by their role the season before.
    let mut roles: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
    for r in &log.portal {
        let name = r.role.map_or("none", |x| x.name());
        let e = roles.entry(name).or_default();
        e.0 += 1;
        e.1 += r.to.is_some() as u32;
    }
    let total: u32 = roles.values().map(|e| e.0).sum();
    let years = log.portal.iter().map(|r| r.year).collect::<std::collections::BTreeSet<_>>().len().max(1);
    println!(
        "  portal entrants: {total} ({:.0} a year), by role last season (share, landed):",
        total as f64 / years as f64
    );
    for (name, (k, landed)) in &roles {
        println!(
            "    {name}: {k} ({:.0}%), {:.0}% found a school",
            100.0 * *k as f64 / total.max(1) as f64,
            100.0 * *landed as f64 / (*k).max(1) as f64
        );
    }
    let mut rs_why: BTreeMap<&str, u32> = BTreeMap::new();
    for r in log.portal.iter().filter(|r| r.role == Some(PlayingTime::Redshirt)) {
        *rs_why.entry(r.why).or_default() += 1;
    }
    println!("    redshirts by reason: {rs_why:?}");
    // Part B 10: FBS entrants by cycle (real NCAA: 2,531 in 2021, 2,918 in
    // 2022; Hudl 2025: about 3,700 from FBS conferences).
    let mut by_year: BTreeMap<i32, u32> = BTreeMap::new();
    for r in log.portal.iter().filter(|r| r.from == Division::FBS) {
        *by_year.entry(r.year).or_default() += 1;
    }
    println!("    FBS entrants by cycle: {by_year:?}");
    let fbs_entrants = log.portal.iter().filter(|r| r.from == Division::FBS).count();
    let fbs_landed_fbs = log.portal.iter().filter(|r| r.from == Division::FBS && r.to == Some(Division::FBS)).count();
    println!(
        "  FBS entrants who landed at FBS: {:.0}% of {fbs_entrants}",
        100.0 * fbs_landed_fbs as f64 / fbs_entrants.max(1) as f64
    );
    // Transfers' first season at the new school.
    for (label, div) in [("all", None), ("FBS", Some(Division::FBS)), ("FCS", Some(Division::FCS))] {
        let v: Vec<_> = log.transfer_seasons.iter().filter(|t| div.is_none_or(|d| t.3 == d)).collect();
        let starts = v.iter().filter(|t| t.2 == PlayingTime::Starter).count();
        let rotation = v.iter().filter(|t| t.2 == PlayingTime::Rotation).count();
        println!(
            "  transfers' first season ({label}, {}): {:.0}% start, {:.0}% rotation",
            v.len(),
            100.0 * starts as f64 / v.len().max(1) as f64,
            100.0 * rotation as f64 / v.len().max(1) as f64
        );
    }
    let rs: Vec<_> = log.transfer_seasons.iter().filter(|t| t.1 == PlayingTime::Redshirt).collect();
    let rs_start = rs.iter().filter(|t| t.2 == PlayingTime::Starter).count();
    println!(
        "  former redshirts among them: {}, {:.0}% start",
        rs.len(),
        100.0 * rs_start as f64 / rs.len().max(1) as f64
    );
}

fn coaching_market(w: &World, years: u32) {
    println!("\n## Coaching market");
    let first = w.year - years as i32;
    let n = years as usize;
    let fbs = |e: Employer| matches!(e, Employer::College(p) if w.programs[p].division == Division::FBS);
    let early = |why: &str| why == "retired for his health" || why == "stepped away, burned out";
    // Every stint, the current job last: (employer, role, from, to, why it ended).
    type StintRow = (Employer, Role, i32, Option<i32>, &'static str);
    let stints = |c: &afm_engine::world::coach::Coach| {
        let mut v: Vec<StintRow> =
            c.history.iter().map(|s| (s.employer, s.role, s.from, Some(s.to), s.ended)).collect();
        if let Some(j) = c.job {
            v.push((j.employer, j.role, j.since, None, ""));
        }
        v
    };
    let mut fbs_changes = vec![0u32; n];
    let mut away_college = vec![0u32; n];
    let mut away_nfl = vec![0u32; n];
    let mut fbs_fired = vec![0u32; n];
    let mut fbs_in_season = vec![0u32; n];
    let mut fired_tenure: Vec<i32> = Vec::new();
    let mut nfl_hires = vec![0u32; n];
    let mut nfl_from_college = vec![0u32; n];
    let mut nfl_from_coord = vec![0u32; n];
    let mut nfl_fired = vec![0u32; n];
    let mut early_all = vec![0u32; n];
    let mut early_fbs = vec![0u32; n];
    let mut early_burnout = 0u32;
    let mut college_to_nfl: Vec<usize> = Vec::new();
    let slot = |y: i32| if y >= first && y < w.year { Some((y - first) as usize) } else { None };
    for c in &w.coaches {
        let v = stints(c);
        for (k, &(e, r, from, to, why)) in v.iter().enumerate() {
            if r == Role::HeadCoach && fbs(e) {
                if let Some(i) = slot(from) {
                    fbs_changes[i] += 1;
                }
                if let Some(y) = to
                    && let Some(i) = slot(y)
                {
                    if why == "left for another job" {
                        match v.get(k + 1).map(|x| x.0) {
                            Some(Employer::Nfl(_)) => away_nfl[i] += 1,
                            _ => away_college[i] += 1,
                        }
                    }
                    if why.starts_with("fired") || why.starts_with("not retained") {
                        fbs_fired[i] += 1;
                        fired_tenure.push(y - from);
                    }
                    if why == "fired during the season" {
                        fbs_in_season[i] += 1;
                    }
                    if early(why) {
                        early_fbs[i] += 1;
                    }
                }
            }
            if let Some(y) = to
                && let Some(i) = slot(y)
                && early(why)
            {
                early_all[i] += 1;
                if why == "stepped away, burned out" {
                    early_burnout += 1;
                }
            }
            if r == Role::HeadCoach && matches!(e, Employer::Nfl(_)) {
                if let Some(i) = slot(from) {
                    nfl_hires[i] += 1;
                    match k.checked_sub(1).map(|j| (v[j].0, v[j].1, v[j].3)) {
                        Some((Employer::College(_), _, Some(t))) if t == from => {
                            nfl_from_college[i] += 1;
                            college_to_nfl.push(c.id);
                        }
                        Some((Employer::Nfl(_), r, Some(t))) if t == from && r != Role::HeadCoach => {
                            nfl_from_coord[i] += 1
                        }
                        _ => {}
                    }
                }
                if let Some(y) = to
                    && let Some(i) = slot(y)
                    && why == "fired"
                {
                    nfl_fired[i] += 1;
                }
            }
        }
    }
    let avg = |v: &[u32]| v.iter().sum::<u32>() as f64 / n.max(1) as f64;
    let row = |name: &str, v: &[u32]| {
        println!("{name}: {:.1} a year ({})", avg(v), v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "))
    };
    row("FBS head coach changes (new head coaches)", &fbs_changes);
    row("FBS head coaches fired", &fbs_fired);
    row("  of them during the season", &fbs_in_season);
    if !fired_tenure.is_empty() {
        let short = fired_tenure.iter().filter(|&&t| t <= 2).count();
        let mean = fired_tenure.iter().sum::<i32>() as f64 / fired_tenure.len() as f64;
        println!(
            "  seasons in the job when fired: mean {mean:.1}; within two seasons {short} of {} ({:.1} a year)",
            fired_tenure.len(),
            short as f64 / n.max(1) as f64
        );
    }
    row("FBS head coaches hired away by another college", &away_college);
    row("FBS head coaches hired away by the NFL", &away_nfl);
    row("NFL head coach hires", &nfl_hires);
    row("  of them from an NFL coordinator job", &nfl_from_coord);
    row("  of them straight from a college job", &nfl_from_college);
    row("NFL head coaches fired", &nfl_fired);
    row("Coaches retiring before 60 (health or burnout), all levels", &early_all);
    row("  of them FBS head coaches", &early_fbs);
    println!("  (burnout {early_burnout}, health {})", early_all.iter().sum::<u32>() - early_burnout);
    // How long national-title coaches stay where they won.
    let mut tenures: Vec<(String, String, i32)> = Vec::new();
    for (p, prog) in w.programs.iter().enumerate() {
        if prog.division != Division::FBS {
            continue;
        }
        for s in prog.seasons.iter().filter(|s| s.national_title) {
            let Some(c) = s.coach else { continue };
            let coach = &w.coaches[c];
            let len = stints(coach)
                .iter()
                .find(|x| {
                    x.0 == Employer::College(p)
                        && x.1 == Role::HeadCoach
                        && x.2 < s.year
                        && x.3.is_none_or(|t| t >= s.year)
                })
                .map(|x| x.3.unwrap_or(w.year) - x.2);
            if let Some(len) = len {
                tenures.push((prog.real_name.clone(), coach.name.clone(), len));
            }
        }
    }
    if !tenures.is_empty() {
        let mut lens: Vec<i32> = tenures.iter().map(|t| t.2).collect();
        lens.sort();
        let mean = lens.iter().sum::<i32>() as f64 / lens.len() as f64;
        println!(
            "FBS national-title coaches' tenure at that school (to now if still there): mean {mean:.1}, median {}, max {}, 10+ seasons {} of {}",
            lens[lens.len() / 2],
            lens[lens.len() - 1],
            lens.iter().filter(|&&l| l >= 10).count(),
            lens.len()
        );
        let mut by_coach: BTreeMap<(String, String), (u32, i32)> = BTreeMap::new();
        for (school, coach, len) in &tenures {
            let e = by_coach.entry((coach.clone(), school.clone())).or_default();
            e.0 += 1;
            e.1 = *len;
        }
        let mut list: Vec<_> = by_coach.into_iter().collect();
        list.sort_by_key(|x| std::cmp::Reverse(x.1.0));
        let shown: Vec<String> =
            list.iter().take(6).map(|((c, s), (t, l))| format!("{c} at {s}: {t} titles, {l} seasons")).collect();
        println!("  most titles: {}", shown.join("; "));
    }
    println!("Example stories:");
    // FBS and NFL stories first, then any level.
    let big =
        |n: &&afm_engine::world::news::NewsItem| n.program.is_none_or(|p| w.programs[p].division == Division::FBS);
    let mut seen: Vec<&str> = Vec::new();
    for pat in ["for the ", "worn out", "for health", "a chance to go home", "a big raise", "a bigger program"] {
        let fits = |n: &&afm_engine::world::news::NewsItem| {
            n.kind == NewsKind::CoachLeft && n.text.contains(pat) && !seen.contains(&n.text.as_str())
        };
        if let Some(n) = w.news.iter().rev().filter(fits).find(big).or_else(|| w.news.iter().rev().find(fits)) {
            println!("- {}: {}", n.year, n.text);
            seen.push(&n.text);
        }
    }
    for n in w.news.iter().rev().filter(|n| n.kind == NewsKind::CoachHired && n.program.is_none()).take(3) {
        println!("- {}: {}", n.year, n.text);
    }
    if let Some(&c) = college_to_nfl.last() {
        println!("A college head coach who went to the NFL:");
        coach_career(w, c);
    }
}

/// Section 5 in the world: the living NFL (rosters of people, money, the
/// market, trades, honors), with rough real ranges to read it against
/// (from memory, to verify: about 550 unrestricted free agents a year, 6-9
/// franchise tags, at most one or two offer sheets, about half of fifth-year
/// options taken, 25-40 cutdown claims, 40-60 player trades, 32-39 comp
/// picks, 10-15 futures deals a team).
fn nfl_league(w: &World, years: u32) {
    use afm_engine::world::pro::{Status, money, payroll};
    println!("\n## The NFL as a living league (section 5)");
    let first = w.year - years.min(30) as i32;
    println!(
        "| year | UFA | re-signed | new team | perfect | swayed | tags | trans | tenders | sheets (matched) | 5th opt taken/declined | ext | rookie ext | restruct | cap cuts | June 1 | futures | claims | holdouts/ins | bluffs (called) | agents fired | promises broken | trades (deadline) | comp (formula/supp; CFAs noted/valued) | other lg | IPP | cap use (low/median/high) | dead money | carryover |"
    );
    println!(
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for m in w.market.years.iter().filter(|m| m.year >= first) {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} ({}) | {}/{} | {} | {} | {} | {} | {} | {} | {} | {}/{} | {} ({}) | {} | {} | {} ({}) | {} ({}/{}; {}/{}) | {} | {} | {:.2}/{:.2}/{:.2} | {:.2}/{:.2}/{:.2} | {:.2}/{:.2}/{:.2} |",
            m.year,
            m.ufa,
            m.resigned,
            m.signed_new,
            m.perfect,
            m.swayed,
            m.tags,
            m.transition,
            m.tenders,
            m.sheets,
            m.sheets_matched,
            m.options_taken,
            m.options_declined,
            m.extensions,
            m.rookie_extensions,
            m.restructures,
            m.cap_cuts,
            m.june1,
            m.futures,
            m.waiver_claims,
            m.holdouts,
            m.holdins,
            m.bluffs,
            m.bluffs_called,
            m.agents_fired,
            m.promises_broken,
            m.trades,
            m.deadline_trades,
            m.comp_picks,
            m.comp_formula,
            m.comp_supplemental,
            m.cfa_noted,
            m.cfa_valued,
            m.other_league,
            m.ipp,
            m.cap_use[0],
            m.cap_use[1],
            m.cap_use[2],
            m.dead[0],
            m.dead[1],
            m.dead[2],
            m.carryover[0],
            m.carryover[1],
            m.carryover[2]
        );
    }
    sections_6_and_7(w, first);
    // Who holds the rosters now.
    let (mut drafted, mut undrafted, mut generated, mut ipp, mut held) = (0, 0, 0, 0, 0);
    let mut ages = Vec::new();
    for p in w.people.iter().filter(|p| p.stage == Stage::Pro) {
        let c = p.pro.as_ref().unwrap();
        if !c.held() {
            continue;
        }
        held += 1;
        ages.push(p.player.age as f64);
        if c.ipp {
            ipp += 1;
        } else if p.notes.iter().any(|n| n.contains("generated")) {
            generated += 1;
        } else if c.draft.is_some() {
            drafted += 1;
        } else {
            undrafted += 1;
        }
    }
    let age = ages.iter().sum::<f64>() / ages.len().max(1) as f64;
    println!(
        "\nOn NFL books now: {held} (drafted {drafted}, undrafted {undrafted}, generated {generated}, pathway {ipp}); average age {age:.1} (real about 26)."
    );
    let year = w.year;
    let cap = money::price_cap(year);
    let mut use_: Vec<(f64, &str)> = (0..w.clubs.len())
        .filter(|&t| afm_engine::world::franchise::active(t, year))
        .map(|t| (payroll(w, t, year) / cap, w.nfl[t].name))
        .collect();
    use_.sort_by(|a, b| a.0.total_cmp(&b.0));
    if let (Some(lo), Some(hi)) = (use_.first(), use_.last()) {
        println!(
            "Cap use in {year} (cap ${:.1}M): lowest {:.0}% ({}), median {:.0}%, highest {:.0}% ({}).",
            cap / 1e6,
            lo.0 * 100.0,
            lo.1,
            use_[use_.len() / 2].0 * 100.0,
            hi.0 * 100.0,
            hi.1
        );
    }
    // The biggest deals.
    let mut deals: Vec<(f64, String)> = w
        .people
        .iter()
        .filter_map(|p| {
            let k = p.pro.as_ref()?.contract.as_ref()?;
            Some((
                k.apy(),
                format!(
                    "{} ({}, {}): {} years, ${:.1}M a year, ${:.1}M guaranteed, {:?}",
                    p.name,
                    p.player.position.name(),
                    w.nfl[k.team].name,
                    k.real_years(),
                    k.apy() / 1e6,
                    k.guaranteed() / 1e6,
                    k.kind
                ),
            ))
        })
        .collect();
    deals.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("\n### Biggest deals now (share of the cap)");
    for (apy, d) in deals.iter().take(10) {
        println!("- {d} ({:.1}% of the cap)", apy / cap * 100.0);
    }
    println!("\n### Holdouts and hold-ins (last five years)");
    for p in w.people.iter().filter(|p| p.stage == Stage::Pro) {
        for n in p.notes.iter().filter(|n| n.starts_with("Held out") || n.starts_with("Held in")) {
            println!("- {}: {n}", p.name);
        }
    }
    println!("\n### Trades (last 15)");
    for t in w.market.picks.trades.iter().rev().take(15) {
        println!("- {}: {}", t.date.pretty(), t.text);
    }
    println!("\n### NFL awards (last ten seasons)");
    for a in w
        .nfl_book
        .awards
        .iter()
        .filter(|a| a.0 >= w.year - 10 && matches!(a.1, "MVP" | "OPOY" | "DPOY" | "OROY" | "DROY"))
    {
        println!("- {} {}: {} ({})", a.0, a.1, a.3, w.nfl[a.4].name);
    }
    println!("\n### Pro Football Hall of Fame (last five classes)");
    for h in w.nfl_book.hall.iter().filter(|h| h.0 >= w.year - 5) {
        println!("- {}: {} ({}) {}", h.0, h.2, h.3.name(), h.4);
    }
    println!("\n### NFL records set in the world");
    for r in w.nfl_book.records.iter().filter(|r| r.world) {
        println!(
            "- {} {}: {:.0} by {} ({})",
            if r.career { "career" } else { "season" },
            r.stat,
            r.value,
            r.holder,
            r.year
        );
    }
    let unsigned = w
        .people
        .iter()
        .filter(|p| p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent))
        .count();
    let other = w
        .people
        .iter()
        .filter(|p| p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| matches!(c.status, Status::Other(_))))
        .count();
    println!("\nUnsigned pros now: {unsigned}; in other leagues: {other}.");
}

/// Build 9's world side, measured (sections 6 and 7): each year's draft-day
/// trades, undrafted signings, scouting looks, poaching, injury
/// settlements, elevations and off-field cases; then moods, captains,
/// support staffs and scouting departments now.
fn sections_6_and_7(w: &World, first: i32) {
    use afm_engine::player::Trait;
    use afm_engine::world::pro::scouts::Role;
    println!("\n## Sections 6 and 7, the world side (build 9)");
    println!(
        "| year | draft trades | undrafted signed | scout looks | poached | injury settlements | elevations | trouble cases | suspensions |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for m in w.market.years.iter().filter(|m| m.year >= first) {
        let elev: u32 = w
            .people
            .iter()
            .filter_map(|p| p.pro.as_ref())
            .flat_map(|c| c.seasons.iter().filter(|s| s.year == m.year))
            .map(|s| s.elevations)
            .sum::<u32>()
            + w.alumni
                .iter()
                .filter_map(|a| a.pro.as_ref())
                .flat_map(|c| c.seasons.iter().filter(|s| s.year == m.year))
                .map(|s| s.elevations)
                .sum::<u32>();
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            m.year,
            m.draft_trades,
            m.udfa_signed,
            m.scout_looks,
            m.poached,
            m.settlements,
            elev,
            m.trouble,
            m.suspensions
        );
    }
    // Moods now, by level.
    let band = |v: &[f32]| {
        let n = v.len().max(1) as f64;
        let low = v.iter().filter(|&&m| m < -0.2).count() as f64 / n;
        let high = v.iter().filter(|&&m| m > 0.2).count() as f64 / n;
        let mean = v.iter().map(|&m| m as f64).sum::<f64>() / n;
        format!(
            "{} players, mean {mean:+.2}, {:.1}% low (under -0.2), {:.1}% high (over 0.2)",
            v.len(),
            100.0 * low,
            100.0 * high
        )
    };
    let college: Vec<f32> = w.people.iter().filter(|p| p.stage == Stage::College).map(|p| p.mood).collect();
    let nfl: Vec<f32> = w
        .people
        .iter()
        .filter(|p| p.pro.as_ref().is_some_and(|c| c.team.is_some() && c.held()))
        .map(|p| p.mood)
        .collect();
    println!("\nMoods now: college {}; NFL {}.", band(&college), band(&nfl));
    let mut why = [0usize; 9];
    for p in w.people.iter().filter(|p| p.stage == Stage::College || p.pro.as_ref().is_some_and(|c| c.held())) {
        why[(p.mood_why as usize).min(8)] += 1;
    }
    let names = ["settled", "role", "happy with role", "losing", "winning", "coach", "life", "room", "money"];
    println!(
        "Main reasons: {}.",
        names.iter().zip(why.iter()).map(|(n, c)| format!("{n} {c}")).collect::<Vec<_>>().join(", ")
    );
    let trust: Vec<f64> = w.people.iter().filter(|p| p.stage == Stage::College).map(|p| p.trust as f64).collect();
    println!("College trust in the head coach: mean {:.2}.", trust.iter().sum::<f64>() / trust.len().max(1) as f64);
    // Rooms.
    let year = w.year;
    let fbs: Vec<&afm_engine::world::program::Program> =
        w.programs.iter().filter(|p| p.division == Division::FBS && p.plays(year)).collect();
    let mut rooms: Vec<(f32, &str)> = fbs.iter().map(|p| (p.team_mood, p.real_name.as_str())).collect();
    rooms.sort_by(|a, b| a.0.total_cmp(&b.0));
    if let (Some(lo), Some(hi)) = (rooms.first(), rooms.last()) {
        println!(
            "FBS rooms: team mood {:+.2} ({}) to {:+.2} ({}); captains per program {:.1}.",
            lo.0,
            lo.1,
            hi.0,
            hi.1,
            fbs.iter().map(|p| p.captains.len()).sum::<usize>() as f64 / fbs.len().max(1) as f64
        );
    }
    let mut cap_lines = Vec::new();
    for t in afm_engine::world::franchise::active_teams(year).into_iter().take(4) {
        let c = &w.clubs[t];
        let names: Vec<String> = c
            .captains
            .iter()
            .map(|&i| {
                let p = &w.people[i];
                format!(
                    "{} ({}, leadership {:.0})",
                    p.name,
                    p.player.position.name(),
                    p.player.personality.traits.get(Trait::Leadership)
                )
            })
            .collect();
        cap_lines.push(format!("- {} (team mood {:+.2}): {}", w.nfl[t].name, c.team_mood, names.join(", ")));
    }
    println!("NFL captains (first four clubs):\n{}", cap_lines.join("\n"));
    // Support staffs and scouting departments.
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let support: Vec<f64> = w
        .clubs
        .iter()
        .flat_map(|c| [c.support.strength.rating, c.support.medical.rating, c.support.analytics.rating])
        .filter(|&r| r > 0)
        .map(|r| r as f64)
        .collect();
    let college_support: Vec<f64> = fbs
        .iter()
        .flat_map(|p| {
            [p.support.strength.rating, p.support.medical.rating, p.support.analytics.rating, p.support.academic.rating]
        })
        .map(|r| r as f64)
        .collect();
    println!(
        "Support heads: NFL mean {:.1} of 20 ({} posts), FBS mean {:.1} ({} posts).",
        mean(&support),
        support.len(),
        mean(&college_support),
        college_support.len()
    );
    let mut sizes = Vec::new();
    let mut judge = Vec::new();
    for t in afm_engine::world::franchise::active_teams(year) {
        let s = &w.clubs[t].scouts;
        let area = s.iter().filter(|x| x.role == Role::Area).count();
        let nat = s.iter().filter(|x| x.role == Role::National).count();
        sizes.push((area, nat));
        judge.extend(s.iter().map(|x| x.judge as f64));
    }
    let areas: Vec<f64> = sizes.iter().map(|x| x.0 as f64).collect();
    println!(
        "Scouting departments: area scouts {:.1} a club ({}-{}), national scouts {:.1}, judging mean {:.1}.",
        mean(&areas),
        sizes.iter().map(|x| x.0).min().unwrap_or(0),
        sizes.iter().map(|x| x.0).max().unwrap_or(0),
        mean(&sizes.iter().map(|x| x.1 as f64).collect::<Vec<_>>()),
        mean(&judge)
    );
}
