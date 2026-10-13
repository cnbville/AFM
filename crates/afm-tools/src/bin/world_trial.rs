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
    let w = World::new(WorldOptions { seed, history_years: years, ..Default::default() });
    println!("# World trial: seed {seed}, {years} years of history, built in {:.0}s\n", t.elapsed().as_secs_f64());

    // AFM_CARDS=path.json: full player pages for featured players, then stop.
    if let Ok(path) = std::env::var("AFM_CARDS") {
        let first = w.year - years as i32 + 5;
        let mut picked: Vec<(&Alumnus, String)> = Vec::new();
        for a in w.alumni.iter().filter(|a| a.player.is_some() && a.year >= first) {
            let tag = match a.pick {
                Some(1) => "Number 1 pick".to_string(),
                Some(p) if p <= 5 && a.year == w.year => format!("Top 5 of the {} draft", a.year),
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
        let cards: Vec<serde_json::Value> = picked
            .iter()
            .map(|(a, tag)| {
                let p = a.player.as_ref().unwrap();
                let ceil = debug::ceilings(p);
                let attrs: Vec<serde_json::Value> = afm_engine::attributes::Attr::ALL
                    .iter()
                    .map(|&t| serde_json::json!([format!("{t:?}"), p.attributes[t].round(), ceil[t].round()]))
                    .collect();
                let seasons: Vec<serde_json::Value> = p
                    .history
                    .iter()
                    .map(|h| {
                        serde_json::json!({
                            "age": h.age, "level": h.level.name(), "role": h.playing_time.name(),
                            "start": h.overall_start.round(), "end": h.overall.round(),
                            "perf": h.performance.map(|v| v.round()), "lr": h.level_rating,
                            "pot": debug::season_potential(h).round()
                        })
                    })
                    .collect();
                serde_json::json!({
                    "tag": tag, "name": a.name, "pos": a.position.name(), "stars": a.stars,
                    "hs": w.school_name(a.high_school), "state": STATE_CENTERS[a.home_state].0,
                    "path": div_name(&w, a), "pick": a.pick, "year": a.year,
                    "round": a.pick.map(|pk| afm_engine::career::draft_round(pk, 257)),
                    "overall": a.overall.round(), "potential": debug::alumnus_potential(a).round(),
                    "visibility": (debug::alumnus_visibility(a) * 100.0).round() / 100.0,
                    "background": a.background_summary, "notes": a.notes,
                    "personality": {"work_ethic": p.personality.work_ethic.round(), "coachability": p.personality.coachability.round(),
                        "competitiveness": p.personality.competitiveness.round(), "love_of_game": p.personality.love_of_game.round()},
                    "bloom": (p.bloom * 10.0).round() / 10.0, "injury": p.injury_proneness.round(),
                    "attributes": attrs, "seasons": seasons
                })
            })
            .collect();
        std::fs::write(&path, serde_json::to_string(&cards).unwrap()).unwrap();
        println!("wrote {} cards to {path}", cards.len());
        return;
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
