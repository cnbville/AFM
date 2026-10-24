//! The storyline engine (update 2, lane 1): the catalogue reads whole, every
//! line in the bank belongs to a kind and a beat and names only values the
//! stories fill, every kind can be told from spark to end, and a story you
//! are in plays through your moves to a chapter that pays.

use std::collections::BTreeSet;

use crate::positions::Position;
use crate::world::seat::{Job, You};
use crate::world::stories::TeamKey;
use crate::world::storylines::catalogue::{CATALOGUE, by_key};
use crate::world::storylines::moves::{self, Move};
use crate::world::storylines::{self as sl, Family, K, Part, Phase, Spot, Who, view};
use crate::world::{World, WorldOptions};

const BANK: &str = include_str!("../../../../data/lines/stories/storylines.txt");
const TELL: &str = include_str!("../world/storylines/tell.rs");
const TEAM_FILL: &str = include_str!("../world/stories/mod.rs");

/// The bank's storyline keys, each with its templates.
fn bank() -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for raw in BANK.lines() {
        let l = raw.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some(head) = l.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            let key = head.split_once('@').map_or(head, |x| x.0).trim().to_string();
            out.push((key, Vec::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.push(l.to_string());
        }
    }
    out
}

/// The literal names a source file sets on a fill: `.set("name"`.
fn literal_sets(src: &str) -> BTreeSet<String> {
    src.split(".set(\"").skip(1).filter_map(|s| s.split_once('"').map(|x| x.0.to_string())).collect()
}

#[test]
fn the_catalogue_reads_whole() {
    assert_eq!(CATALOGUE.len(), K::ALL.len());
    assert_eq!(CATALOGUE.len(), 111, "the design names 111 kinds");
    let mut keys = BTreeSet::new();
    for (k, d) in CATALOGUE.iter().enumerate() {
        assert!(keys.insert(d.key), "{} twice", d.key);
        assert_eq!(by_key(d.key), Some(k as u16));
        assert_eq!(K::ALL[k].def().key, d.key);
        assert!(!d.name.is_empty() && !d.question.is_empty(), "{}", d.key);
        assert!(d.levels != 0 && d.quiet > 0, "{}", d.key);
        assert!(!d.endings.is_empty(), "{} has no ending", d.key);
        for (e, v) in d.endings {
            assert!((-1.0..=1.0).contains(v), "{}.{e}", d.key);
        }
        assert!(!d.real.is_empty(), "{} names no real story it is drawn from", d.key);
    }
    for f in Family::ALL {
        let n = CATALOGUE.iter().filter(|d| d.family == f).count();
        assert!(n >= 3, "{} has {n} kinds", f.name());
    }
}

#[test]
fn every_line_belongs_to_a_kind_and_a_beat() {
    let kinds: BTreeSet<&str> = CATALOGUE.iter().map(|d| d.key).collect();
    let fams: BTreeSet<&str> = Family::ALL.iter().map(|f| f.key()).collect();
    let beats = [
        "seed", "spark", "report", "leak", "quote", "post", "radio", "column", "jab", "move", "denial", "fact", "peak",
        "end", "fizzle", "callback", "memo", "fans", "private", "chapter", "legacy", "fixer",
    ];
    for (key, lines) in bank().iter().filter(|(k, _)| k.starts_with("sl.")) {
        let parts: Vec<&str> = key.split('.').collect();
        assert!(parts.len() >= 3, "{key}: too short");
        let head = parts[1];
        assert!(
            head == "any" || head == "name" || kinds.contains(head) || fams.contains(head),
            "{key}: no kind or family {head}"
        );
        assert!(beats.contains(&parts[2]), "{key}: no beat {}", parts[2]);
        assert!(!lines.is_empty(), "{key} has no lines");
        // An ending's key names one of its kind's endings.
        if parts[2] == "end" && parts.len() > 3 && kinds.contains(head) {
            let d = &CATALOGUE[by_key(head).unwrap() as usize];
            assert!(d.endings.iter().any(|(e, _)| *e == parts[3]), "{key}: {head} has no ending {}", parts[3]);
        }
    }
    // Every kind has its own spark, and an end line for each of its endings.
    let have: BTreeSet<String> = bank().into_iter().map(|x| x.0).collect();
    for d in CATALOGUE.iter() {
        assert!(have.contains(&format!("sl.{}.spark", d.key)), "{} has no spark line", d.key);
        for (e, _) in d.endings {
            assert!(
                have.contains(&format!("sl.{}.end.{e}", d.key)) || have.contains(&format!("sl.{}.end", d.key)),
                "{}.{e} has no end line",
                d.key
            );
        }
    }
    for b in ["spark", "report", "leak", "radio", "column", "jab", "fans", "post", "peak", "end", "fizzle"] {
        assert!(have.contains(&format!("sl.any.{b}")), "no sl.any.{b} to fall back on");
    }
}

#[test]
fn every_line_names_values_the_stories_fill() {
    // What `tell::fill` sets by name, what the team fill sets, and the
    // names it sets from variables (the cast, the counts, the kind's word).
    let mut known = literal_sets(TELL);
    known.extend(literal_sets(TEAM_FILL));
    for k in [
        "lead",
        "lead_last",
        "lead_first",
        "lead_pos",
        "rival",
        "rival_last",
        "rival_first",
        "rival_pos",
        "n",
        "n2",
        "n3",
        "n4",
        "trophy",
        "conf",
        "holder",
        "pos",
        "award",
        "allit_first",
        "allit_last",
    ] {
        known.insert(k.to_string());
    }
    let mut unknown = BTreeSet::new();
    for (key, lines) in bank().iter().filter(|(k, _)| k.starts_with("sl.")) {
        for l in lines {
            assert_eq!(l.matches('{').count(), l.matches('}').count(), "{key}: unbalanced braces in {l}");
            for name in l.split('{').skip(1).filter_map(|s| s.split_once('}').map(|x| x.0)) {
                if !known.contains(name) {
                    unknown.insert(format!("{key}: {{{name}}}"));
                }
            }
        }
    }
    assert!(unknown.is_empty(), "lines name values no story fills, so they are never used: {unknown:?}");
}

/// A tiny world in 2037 with you as an NFL head coach.
fn nfl_world() -> (World, usize) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let t = (0..w.clubs.len())
        .filter(|&t| crate::world::franchise::active(t, w.year))
        .max_by_key(|&t| w.clubs[t].roster.len())
        .unwrap();
    let teaching = vec![12.0; crate::world::coach::GROUP_COUNT];
    let you: You = serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap();
    w.start_career(&you, Job::Nfl(t), 7);
    (w, t)
}

/// A public quarterback battle on your team: the starter against the next
/// man up.
fn qb_battle(w: &mut World, t: usize) -> u32 {
    let mut qbs: Vec<usize> =
        w.clubs[t].roster.iter().copied().filter(|&i| w.people[i].player.position == Position::QB).collect();
    let rest: Vec<usize> = w.clubs[t].roster.iter().copied().filter(|i| !qbs.contains(i)).collect();
    qbs.extend(rest);
    let (a, b) = (w.people[qbs[0]].player.id, w.people[qbs[1]].player.id);
    let team = TeamKey::Nfl(t as u16);
    let spot = Spot::new(K::QbBattle, sl::subj(sl::tcode(team), w.year as u64))
        .team(team)
        .lead(Who::Player(a))
        .rival(Who::Player(b))
        .public();
    sl::notice(w, spot).expect("the battle opens")
}

#[test]
fn a_story_on_your_team_is_yours_and_reads_whole() {
    let (mut w, t) = nfl_world();
    let id = qb_battle(&mut w, t);
    let s = w.storylines.get(id).unwrap().clone();
    assert_eq!(s.you, Some(Part::Decider), "your team's battle is yours to decide");
    assert!(s.public && s.near && s.phase == Phase::Spark);
    assert!(view::mine(&w).iter().any(|x| x.id == id));
    let q = view::question(&w, &s);
    assert!(q.contains("quarterback") && !q.contains('{'), "{q}");
    let spark = s.beats.first().expect("a spark beat");
    assert!(!spark.text.is_empty() && !spark.text.contains('{'), "{}", spark.text);
    // Each move the page offers has its effects read off it: heat first,
    // three at most.
    let opts = view::choices(&mut w, id);
    assert!(opts.len() >= 3, "{} moves", opts.len());
    for o in &opts {
        assert!(!o.chips.is_empty() && o.chips.len() <= 3, "{:?}", o.chips);
        let first = &o.chips[0].0;
        assert!(first.starts_with("heat") || first == "cools it a little" || first == "stays private", "{first}");
    }
    let back = moves::chips(&w, &s, Move::Back);
    assert!(back.iter().any(|c| c.0.contains("starts, said in public")), "{back:?}");
}

#[test]
fn your_move_becomes_a_beat_and_a_good_ending_pays() {
    let (mut w, t) = nfl_world();
    let id = qb_battle(&mut w, t);
    let beats = w.storylines.get(id).unwrap().beats.len();
    let said = view::choose(&mut w, id, Move::Back.code()).expect("the move is open");
    assert!(!said.is_empty());
    let s = w.storylines.get(id).unwrap();
    assert!(s.beats.len() > beats, "your words are a beat");
    assert_eq!(s.moves, 1);
    // Hot, handled well, won by the man you backed.
    {
        let k = w.storylines.idx(id).unwrap();
        let s = &mut w.storylines.open[k];
        s.heat = 80.0;
        s.peak_heat = 80.0;
        s.handled = 0.6;
    }
    let bank0: f32 = w.storylines.credit.bank.iter().sum();
    sl::close(&mut w, id, "won_job", None);
    // The answer itself is a loud beat: the lesson reads the story's peak
    // with it.
    let peak = w.storylines.get(id).unwrap().peak_heat;
    assert!(peak >= 80.0);
    let c = &w.storylines.credit;
    let lesson: f32 = c.bank.iter().sum::<f32>() - bank0;
    let want = (0.25 + 0.75 * peak / 100.0).clamp(0.25, 1.0);
    assert!((lesson - want).abs() < 1e-3, "0.25 + 0.75 x the peak heat in lesson points: {want}, got {lesson}");
    assert_eq!(c.chapters.len(), 1, "a hot story you saw through writes a chapter");
    let ch = &c.chapters[0];
    assert!(!ch.title.is_empty() && !ch.text.is_empty() && !ch.question.is_empty());
    assert!(ch.paid.iter().any(|p| p.0 == "Reputation, of 20" && p.1.starts_with('+')), "{:?}", ch.paid);
    assert!(ch.paid.iter().any(|p| p.0.starts_with("Lesson points into")), "{:?}", ch.paid);
    assert!(ch.left.iter().any(|l| l.0 == "memento"), "a story this hot leaves a keepsake: {:?}", ch.left);
    assert!(ch.left.iter().any(|l| l.0 == "tie"), "the man you saw through it is tied to you: {:?}", ch.left);
    assert!(!ch.beats.is_empty() && ch.beats.windows(2).all(|x| x[0].0 <= x[1].0), "beats oldest first");
    let ended = w.storylines.get(id).unwrap();
    assert_eq!(ended.phase, Phase::Resolved);
    assert!(view::ended_text(ended).is_some_and(|t| !t.is_empty()));
}

#[test]
fn a_bad_ending_costs_and_a_quiet_story_pays_nothing() {
    let (mut w, t) = nfl_world();
    let id = qb_battle(&mut w, t);
    {
        let k = w.storylines.idx(id).unwrap();
        let s = &mut w.storylines.open[k];
        s.heat = 50.0;
        s.peak_heat = 50.0;
        s.handled = -0.6;
    }
    let bank0: f32 = w.storylines.credit.bank.iter().sum();
    sl::close(&mut w, id, "split", None);
    let c = &w.storylines.credit;
    assert_eq!(c.bank.iter().sum::<f32>(), bank0, "no lesson from a story that went badly");
    let ch = c.chapters.last().expect("heat 50 still writes a chapter");
    assert!(ch.paid.iter().any(|p| p.0 == "Reputation, of 20" && p.1.starts_with('\u{2212}')), "{:?}", ch.paid);
    // A second battle the next season that stays cool writes nothing.
    let (mut w, t) = nfl_world();
    let id = qb_battle(&mut w, t);
    {
        let k = w.storylines.idx(id).unwrap();
        let s = &mut w.storylines.open[k];
        s.heat = 12.0;
        s.peak_heat = 12.0;
    }
    sl::close(&mut w, id, "kept_job", None);
    assert!(w.storylines.credit.chapters.is_empty(), "a cool story leaves no chapter");
}

#[test]
fn the_job_read_names_what_a_job_hands_you() {
    let (mut w, t) = nfl_world();
    let id = qb_battle(&mut w, t);
    {
        let k = w.storylines.idx(id).unwrap();
        w.storylines.open[k].heat = 75.0;
    }
    let read = sl::credit::job_read(&mut w, TeamKey::Nfl(t as u16));
    assert!(read.volatility > 0.0, "a hot public story unsettles the job");
    assert!(read.stories.iter().any(|s| s.id == id), "the battle comes with the job");
    assert!(!read.word.is_empty());
}
