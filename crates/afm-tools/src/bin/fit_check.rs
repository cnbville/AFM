//! The scheme-fit check (lane SF0, `docs/design/scheme-fit.md`): the audit's
//! measuring tool, so every scheme-fit lane measures its items the same way.
//! Team 0 of a 32-team league is set to the level's typical team; each arm
//! changes one thing on it and replays the same games as its context's
//! typical arm (`afm_tools::fit`).
//!
//! Usage:
//!   fit_check list [fbs|nfl|hs|fcs] [arms]
//!   fit_check run <fbs|nfl|hs|fcs> [games] [seed] [out] [arms]
//!   fit_check read <run folder>
//!   fit_check repeat <label=folder> <label=folder> ...
//!
//! `run` plays the arms into a folder (default `fit-out/<level>-<seed>`; an
//! arm already there is kept, so a stopped run picks up where it was).
//! Defaults are the audit's: FBS 3,000 games an arm on seed 151, the NFL
//! 2,400 on seed 161, high school and FCS 3,000 on seed 151; games must be a
//! multiple of 40, so every block holds as many. `arms` is a
//! comma-separated list of words: an arm is picked when its id holds any of
//! them, `bases` picks the typical arms, `draws` the as-recruited ones and
//! `repeat` the audit's repeat set (the arms it played again on FBS seed 171
//! and the world's NFL rosters); each picked arm's typical arm comes too.
//! With none, every arm plays.
//!
//! `read` writes the report's tables into `<run folder>/report/`. `repeat`
//! prints the repeat table (the headline claims in every run) and each later
//! run against the first on points, arm by arm.
//!
//! AFM_AB_WORLD: a world file whose 32 NFL clubs replace the synthetic league
//! (NFL only, like `ab_check`); team 0 is still set to the typical team.

extern crate afm_tools;
use std::path::{Path, PathBuf};

use afm_engine::player::Level;
use afm_tools::fit::{self, Setup, read};

fn level_of(s: &str) -> Option<Level> {
    match s {
        "fbs" => Some(Level::FBS),
        "fcs" => Some(Level::FCS),
        "nfl" => Some(Level::NFL),
        "hs" => Some(Level::HS),
        _ => None,
    }
}

fn words(s: Option<&String>) -> Vec<String> {
    s.map(|s| s.split(',').map(str::trim).filter(|x| !x.is_empty()).map(str::to_string).collect()).unwrap_or_default()
}

fn usage() -> ! {
    eprintln!(
        "usage:\n  fit_check list [fbs|nfl|hs|fcs] [arms]\n  fit_check run <fbs|nfl|hs|fcs> [games] [seed] [out] [arms]\n  \
         fit_check read <run folder>\n  fit_check repeat <label=folder> <label=folder> ..."
    );
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("list") => {
            let level = args.get(2).and_then(|s| level_of(s)).unwrap_or(Level::FBS);
            let list = fit::pick(&fit::arms(level), &words(args.get(3)));
            for a in &list {
                println!("{}\t{}\t{}", a.id, a.ctx.name(), a.label);
            }
            eprintln!("{} arms", list.len());
        }
        Some("run") => {
            let Some(level) = args.get(2).and_then(|s| level_of(s)) else { usage() };
            let nfl = level == Level::NFL;
            let games: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(if nfl { 2400 } else { 3000 });
            let seed: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(if nfl { 161 } else { 151 });
            let out: PathBuf =
                args.get(5).map(PathBuf::from).unwrap_or_else(|| format!("fit-out/{}-{seed}", args[2]).into());
            let list = fit::pick(&fit::arms(level), &words(args.get(6)));
            if list.is_empty() {
                eprintln!("no arm matches {:?}", args.get(6));
                std::process::exit(1);
            }
            if let Err(e) = fit::run(&Setup { level, games, seed }, &out, &list) {
                eprintln!("{e}");
                std::process::exit(1);
            }
            eprintln!("next: fit_check read {}", out.display());
        }
        Some("read") => {
            let Some(dir) = args.get(2) else { usage() };
            let run = read::Run::load(Path::new(dir)).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1)
            });
            match read::report(&run) {
                Ok(files) => {
                    for f in files {
                        println!("{f}");
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some("repeat") => {
            let mut runs = Vec::new();
            for a in &args[2..] {
                let (label, dir) = a.split_once('=').unwrap_or((a.as_str(), a.as_str()));
                match read::Run::load(Path::new(dir)) {
                    Ok(r) => runs.push((label.to_string(), r)),
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                }
            }
            if runs.is_empty() {
                usage();
            }
            print!("{}", read::repeat(&runs));
        }
        _ => usage(),
    }
}
