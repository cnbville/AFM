//! Section 2 trial: builds a real-size world with 30 years of hidden history
//! and prints the notable outcomes with the facts behind each one, so every
//! result can be given a "why". Uses the debug feature (hidden values).
//! Usage: world_trial [seed] [history_years]

use std::collections::BTreeMap;
use std::time::Instant;

use afm_engine::debug;
use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::data::{self, Division};
use afm_engine::world::geo::STATE_CENTERS;
use afm_engine::world::news::{Channel, NewsKind};
use afm_engine::world::program::{GradeCat, letter};
use afm_engine::world::{Alumnus, World, WorldOptions};

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
        Employer::Nfl(_) => "NFL".into(),
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let t = Instant::now();
    // AFM_ENGINE_FROM=year plays college games on the match engine from that year.
    let engine_from = std::env::var("AFM_ENGINE_FROM").ok().and_then(|v| v.parse().ok());
    // The dashboard and player pages read the game log (box score totals and
    // college stat lines). Keeping it only watches the games.
    let keep_game_log = std::env::var("AFM_DASH").is_ok() || std::env::var("AFM_CARDS").is_ok();
    // AFM_SCALE=0.05: a small world, for quick checks of the tooling only.
    let scale = std::env::var("AFM_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let w = World::new(WorldOptions {
        seed,
        history_years: years,
        engine_from,
        keep_game_log,
        scale,
        ..Default::default()
    });
    println!("# World trial: seed {seed}, {years} years of history, built in {:.0}s\n", t.elapsed().as_secs_f64());

    // AFM_CARDS=path.json: full player pages for featured players.
    let mut card_values: Vec<serde_json::Value> = Vec::new();
    if let Ok(path) = std::env::var("AFM_CARDS") {
        let first = w.year - years as i32 + 5;
        let mut picked: Vec<(&Alumnus, String)> = Vec::new();
        for a in w.alumni.iter().filter(|a| a.player.is_some() && a.year >= first) {
            let tag = match a.pick {
                Some(1) => "Number 1 pick".to_string(),
                Some(p) if p <= 32 && a.year + 2 >= w.year => format!("Round 1 of the {} draft", a.year),
                _ => continue,
            };
            picked.push((a, tag));
        }
        let gems = w.alumni.iter().filter(|a| {
            a.player.is_some()
                && a.year >= first
                && a.stars <= 2
                && a.pick.is_some_and(|p| p <= 64)
                && a.colleges.first().is_some_and(|&c| w.programs[c].division != Division::FBS)
        });
        picked.extend(gems.take(3).map(|a| (a, "Hidden gem".to_string())));
        let busts =
            w.alumni.iter().filter(|a| a.player.is_some() && a.year >= first && a.stars == 5 && a.left_football);
        picked.extend(busts.take(3).map(|a| (a, "Five-star who left football".to_string())));
        let late =
            w.alumni.iter().filter(|a| a.player.is_some() && a.year >= first && a.pick.is_some_and(|p| p >= 200));
        picked.extend(late.take(5).map(|a| (a, "Late-round pick".to_string())));
        let undrafted = w.alumni.iter().filter(|a| {
            a.player.is_some() && a.year + 1 >= w.year && a.pick.is_none() && !a.left_football && a.stars >= 4
        });
        picked.extend(undrafted.take(5).map(|a| (a, "Undrafted blue-chip".to_string())));
        let cards: Vec<serde_json::Value> =
            picked.iter().map(|(a, tag)| afm_tools::dashboard::card(&w, a, tag)).collect();
        std::fs::write(&path, serde_json::to_string(&cards).unwrap()).unwrap();
        println!("wrote {} cards to {path}\n", cards.len());
        card_values = cards;
    }
    if let Ok(path) = std::env::var("AFM_DASH") {
        write_dashboard(&w, &path, seed, years, &card_values);
        println!("wrote the debug dashboard data to {path}\n");
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
        let drafted = picks.iter().filter(|a| a.stars == stars && complete.contains(&a.class_of)).count();
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

    // --- National champions ---
    println!("\n## FBS national champions");
    let champs: BTreeMap<i32, String> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS)
        .flat_map(|p| {
            p.seasons
                .iter()
                .filter(|s| s.national_title)
                .map(move |s| (s.year, format!("{} {}-{}", p.real_name, s.wins, s.losses)))
        })
        .collect();
    println!("{}", champs.iter().map(|(y, n)| format!("{y} {n}")).collect::<Vec<_>>().join("; "));
    for d in [Division::FCS, Division::D2] {
        let c: BTreeMap<String, usize> = w
            .programs
            .iter()
            .filter(|p| p.division == d)
            .map(|p| (p.real_name.clone(), p.seasons.iter().filter(|s| s.national_title).count()))
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
    println!(
        "\nLongest Power 4 runs without a winning season: {}",
        droughts.iter().take(6).map(|(n, name)| format!("{name} {n}")).collect::<Vec<_>>().join(", ")
    );
    let mut titles: Vec<(usize, &str)> = w
        .programs
        .iter()
        .map(|p| (p.seasons.iter().filter(|s| s.national_title).count(), p.real_name.as_str()))
        .filter(|t| t.0 > 0 && w.programs.iter().any(|p| p.real_name == t.1 && p.division == Division::FBS))
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
                s.ended == "fired"
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
    for a in recorded.iter().filter(|a| a.stars == 5 && a.left_football).take(2) {
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

    // --- Character report: what the game shows instead ---
    println!("\n## What a scout sees (character report on a current five-star)");
    let mut w = w;
    if let Some(i) = w.people.iter().position(|p| p.stars == 5) {
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
    let five_juco = recorded.iter().filter(|a| a.stars == 5 && a.juco).count();
    let five = recorded.iter().filter(|a| a.stars == 5).count();
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
    let real = std::fs::read_to_string("data/real-world/draft_positions_2000_2025.csv").unwrap_or_default();
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
    let combine = std::fs::read_to_string("data/real-world/combine_by_position.csv").unwrap_or_default();
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
                    "discipline": a.discipline.round(), "recruiting": a.recruiting.round(), "network": a.network.round(), "relationships": a.relationships.round()},
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
