//! Names in a talk as links (Cain 2026-11-26: "make sure the live talks have
//! hyperlinks on names so I can easily navigate and don't need to manually go
//! to the squad page").
//!
//! The game's own words already carry link marks (`[[p:12|Name]]`). What a
//! person says in a live talk is free text the voice wrote, and your typed
//! lines have their marks stripped on purpose (`say::typed_lines`), so a name
//! in either reaches the screen plain. This step finds the people the talk can
//! be about, your roster, your staff, the people across from you and the
//! friends of the man across, and sends each line again with their names
//! marked: `rich` beside `text`. `text` is never changed (the page finds
//! your lines by it), and a mark only ever points at a real person the game
//! found, so a typed line still can't draw a link of its own.
//!
//! A full name always links. A last name alone links only when exactly one
//! person in the pool has it, and not at the start of a sentence when it is
//! also an everyday word ("Young players need reps").

use std::collections::HashMap;

use afm_engine::world::World;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::scene::Who;
use afm_engine::world::folk::{self, Team, cast, ties};
use serde_json::Value;

/// Surnames that are also everyday words: never linked on their own at the
/// start of a sentence.
const EVERYDAY: [&str; 40] = [
    "Young", "Brown", "White", "Black", "Green", "Hall", "Bell", "Cook", "King", "Long", "Hill", "Price", "Stone",
    "Wood", "Page", "Rice", "Love", "Best", "Strong", "Short", "Fox", "Hunt", "Ford", "Banks", "Carter", "Cross",
    "Day", "Early", "Fair", "Rose", "Good", "Grant", "Hardy", "Jolly", "Lamb", "Mason", "Miller", "Moody", "Rich",
    "Sharp",
];

/// A name (or a last name) and the mark that links it.
struct Entry {
    key: String,
    mark: String,
    /// A last name used alone.
    alias: bool,
}

/// The people a talk may be about, found by their names.
pub struct Pool {
    by_first: HashMap<char, Vec<Entry>>,
}

impl Pool {
    /// From (full name, link mark) pairs.
    pub fn new(people: Vec<(String, String)>) -> Pool {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut firsts: Vec<(String, String)> = Vec::new();
        for (name, mark) in people {
            if name.trim().is_empty() || seen.contains_key(&name) {
                continue;
            }
            seen.insert(name.clone(), firsts.len());
            firsts.push((name, mark));
        }
        let mut lasts: HashMap<String, Vec<usize>> = HashMap::new();
        for (k, (name, _)) in firsts.iter().enumerate() {
            if let Some(l) = last_name(name) {
                lasts.entry(l.to_string()).or_default().push(k);
            }
        }
        let mut by_first: HashMap<char, Vec<Entry>> = HashMap::new();
        for (name, mark) in &firsts {
            if let Some(c) = name.chars().next() {
                by_first.entry(c).or_default().push(Entry { key: name.clone(), mark: mark.clone(), alias: false });
            }
        }
        for (l, at) in lasts {
            // Two men with one last name: only the full names link.
            if at.len() != 1 || l.chars().count() < 3 || seen.contains_key(&l) {
                continue;
            }
            if let Some(c) = l.chars().next() {
                by_first.entry(c).or_default().push(Entry { key: l, mark: firsts[at[0]].1.clone(), alias: true });
            }
        }
        for v in by_first.values_mut() {
            v.sort_by_key(|e| std::cmp::Reverse(e.key.len()));
        }
        Pool { by_first }
    }

    pub fn is_empty(&self) -> bool {
        self.by_first.is_empty()
    }

    /// The text with every known name marked, the marks it already carries
    /// left alone. `None` when nothing in it names anyone.
    pub fn link(&self, text: &str) -> Option<String> {
        if self.is_empty() || !text.chars().any(char::is_uppercase) {
            return None;
        }
        let mut out = String::with_capacity(text.len() + 32);
        let mut changed = false;
        let mut rest = text;
        while !rest.is_empty() {
            // A mark the game already put in: copy it whole.
            if rest.starts_with("[[")
                && let Some(end) = rest.find("]]")
            {
                out.push_str(&rest[..end + 2]);
                rest = &rest[end + 2..];
                continue;
            }
            let c = rest.chars().next().unwrap_or(' ');
            let before = out.chars().rev().find(|x| !x.is_whitespace());
            let boundary = out.chars().next_back().is_none_or(|p| !p.is_alphanumeric() && p != '\'' && p != '’');
            if boundary
                && c.is_uppercase()
                && let Some(list) = self.by_first.get(&c)
                && let Some(e) = list.iter().find(|e| {
                    fits(&e.key, rest) && !(e.alias && starts_sentence(before) && EVERYDAY.contains(&e.key.as_str()))
                })
            {
                out.push_str(&mark_as(&e.mark, &e.key));
                rest = &rest[e.key.len()..];
                changed = true;
                continue;
            }
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
        changed.then_some(out)
    }
}

/// Whether `rest` starts with `key` as a whole word.
fn fits(key: &str, rest: &str) -> bool {
    rest.starts_with(key) && rest[key.len()..].chars().next().is_none_or(|n| !n.is_alphanumeric())
}

fn starts_sentence(before: Option<char>) -> bool {
    before.is_none_or(|p| matches!(p, '.' | '!' | '?' | '"' | '“' | ':'))
}

/// The mark with the words as written (a last name alone keeps its own
/// label, not the full name).
fn mark_as(mark: &str, shown: &str) -> String {
    match mark.rsplit_once('|') {
        Some((head, _)) => format!("{head}|{shown}]]"),
        None => mark.to_string(),
    }
}

fn last_name(name: &str) -> Option<&str> {
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.as_slice() {
        [.., l, s] if matches!(*s, "Jr." | "Sr." | "II" | "III" | "IV" | "V") => Some(l),
        [_, .., l] => Some(l),
        _ => None,
    }
}

/// The mark for a person in a talk: his page where he has one.
pub fn them(w: &World, who: Them) -> Option<String> {
    match who {
        Them::Player(id) => folk::person(w, id).map(|i| folk::player_link(w, i)),
        Them::Coach(c) if c < w.coaches.len() => Some(folk::coach_link(w, c)),
        Them::Cast(r, _) => Some(cast::get(w, r).link()),
        _ => None,
    }
}

/// Everyone a talk can be about.
pub fn pool(w: &World, scene: Option<u32>) -> Pool {
    let mut people: Vec<(String, String)> = Vec::new();
    let you = w.career.as_ref().map(|c| c.coach);
    let player =
        |people: &mut Vec<(String, String)>, i: usize| people.push((w.people[i].name.clone(), folk::player_link(w, i)));
    let seat = Team::seat(w);
    // The people across, and the friends of the man across.
    if let Some(s) = scene.and_then(|id| w.folk.scenes.iter().find(|s| s.id == id)) {
        for x in &s.who {
            match x {
                Who::Player(id) => {
                    if let Some(i) = folk::person(w, *id) {
                        player(&mut people, i);
                    }
                    for f in ties::friends(w, *id) {
                        if let Some(j) = folk::person(w, f) {
                            player(&mut people, j);
                        }
                    }
                }
                Who::Coach(c) if c < &w.coaches.len() && Some(*c) != you => {
                    people.push((w.coaches[*c].name.clone(), folk::coach_link(w, *c)))
                }
                Who::Cast(tok) => {
                    if let Some(c) = cast::from_token(w, tok) {
                        people.push((c.name.clone(), c.link()));
                    }
                }
                _ => {}
            }
        }
    }
    // Your roster and your staff.
    if let Some(team) = seat {
        for i in team.roster(w) {
            player(&mut people, i);
        }
        for (c, coach) in w.coaches.iter().enumerate() {
            if Some(c) != you && coach.job.is_some_and(|j| Team::from_employer(j.employer) == team) {
                people.push((coach.name.clone(), folk::coach_link(w, c)));
            }
        }
    }
    Pool::new(people)
}

/// A scene's reply gets `rich` on each line, `name_rich` on a speaker, and a
/// `title_rich` and `outcome_rich` where those name someone.
/// Nothing else in the reply changes.
pub fn scene_names(w: &World, v: &mut Value) {
    let is_scene = v
        .get("lines")
        .and_then(Value::as_array)
        .is_some_and(|l| l.iter().all(|x| x.is_object() && x.get("by").is_some()));
    if !is_scene {
        return;
    }
    let id = v.get("id").and_then(Value::as_u64).map(|x| x as u32);
    let pool = pool(w, id);
    if pool.is_empty() {
        return;
    }
    if let Some(lines) = v.get_mut("lines").and_then(Value::as_array_mut) {
        for l in lines {
            if let Some(r) = l.get("text").and_then(Value::as_str).and_then(|t| pool.link(t)) {
                l["rich"] = Value::String(r);
            }
            if l["by"] != 1
                && let Some(r) = l.get("name").and_then(Value::as_str).and_then(|t| pool.link(t))
            {
                l["name_rich"] = Value::String(r);
            }
        }
    }
    if let Some(r) = v.get("title").and_then(Value::as_str).and_then(|t| pool.link(t)) {
        v["title_rich"] = Value::String(r);
    }
    let outcome: Option<Vec<Value>> = v.get("outcome").and_then(Value::as_array).map(|o| {
        o.iter()
            .map(|x| x.as_str().map_or(x.clone(), |t| Value::String(pool.link(t).unwrap_or_else(|| t.to_string()))))
            .collect()
    });
    if let (Some(o), Some(plain)) = (outcome, v.get("outcome").and_then(Value::as_array))
        && o.iter().zip(plain).any(|(a, b)| a != b)
    {
        v["outcome_rich"] = Value::Array(o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Pool {
        Pool::new(vec![
            ("Marcus Hill".into(), "[[p:1|Marcus Hill]]".into()),
            ("Jaylen Ortiz".into(), "[[p:2|Jaylen Ortiz]]".into()),
            ("Dee Young".into(), "[[p:3|Dee Young]]".into()),
            ("Sam Jones".into(), "[[p:4|Sam Jones]]".into()),
            ("Tre Jones".into(), "[[p:5|Tre Jones]]".into()),
            ("Coy Wells Jr.".into(), "[[c:9|Coy Wells Jr.]]".into()),
        ])
    }

    #[test]
    fn a_full_name_links() {
        let r = sample().link("I like Marcus Hill's feet. Jaylen Ortiz reads it faster.").unwrap();
        assert_eq!(r, "I like [[p:1|Marcus Hill]]'s feet. [[p:2|Jaylen Ortiz]] reads it faster.");
    }

    #[test]
    fn a_last_name_alone_links_when_it_is_one_man() {
        let p = sample();
        assert_eq!(p.link("Ortiz is quick.").unwrap(), "[[p:2|Ortiz]] is quick.");
        assert_eq!(p.link("Wells came by.").unwrap(), "[[c:9|Wells]] came by.");
        assert_eq!(p.link("Ask Hill about it").unwrap(), "Ask [[p:1|Hill]] about it");
    }

    #[test]
    fn two_men_with_one_last_name_link_only_by_full_name() {
        let p = sample();
        assert_eq!(p.link("Jones is hurt."), None);
        assert_eq!(p.link("Sam Jones is hurt.").unwrap(), "[[p:4|Sam Jones]] is hurt.");
    }

    #[test]
    fn an_everyday_word_is_not_a_name_at_the_start_of_a_sentence() {
        let p = sample();
        assert_eq!(p.link("Young players need reps."), None);
        assert_eq!(p.link("I'd start Young at nickel.").unwrap(), "I'd start [[p:3|Young]] at nickel.");
        assert_eq!(p.link("Dee Young is ready.").unwrap(), "[[p:3|Dee Young]] is ready.");
    }

    #[test]
    fn a_mark_the_game_already_made_is_left_alone() {
        let p = sample();
        assert_eq!(p.link("[[p:1|Marcus Hill]] and Ortiz."), Some("[[p:1|Marcus Hill]] and [[p:2|Ortiz]].".into()));
        assert_eq!(p.link("[[p:1|Marcus Hill]] only."), None);
    }

    #[test]
    fn a_name_inside_a_longer_word_is_not_a_name() {
        let p = sample();
        assert_eq!(p.link("Hillary and Ortizes"), None);
    }

    #[test]
    fn nothing_to_link_changes_nothing() {
        assert_eq!(sample().link("Good morning, coach."), None);
        assert_eq!(Pool::new(vec![]).link("Marcus Hill"), None);
    }

    #[test]
    fn a_reply_that_is_not_a_scene_is_left_alone() {
        let w = World::empty(afm_engine::world::WorldOptions {
            scale: 0.001,
            history_years: 0,
            start_year: 2037,
            ..Default::default()
        });
        let mut v = serde_json::json!({ "lines": ["a string", "another"], "id": 3 });
        let before = v.clone();
        scene_names(&w, &mut v);
        assert_eq!(v, before);
    }

    #[test]
    fn your_staff_and_roster_names_link_in_a_talk_on_a_small_world() {
        use afm_engine::world::WorldOptions;
        use afm_engine::world::data::Division;
        use afm_engine::world::seat::Job;
        let mut w =
            World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
        let p = w.programs.iter().position(|p| p.division == Division::FBS).unwrap();
        let team = Team::College(p);
        let you = serde_json::from_value(serde_json::json!({
            "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
            "teaching": vec![12.0; afm_engine::world::coach::GROUP_COUNT], "scheme": 12.0, "game_management": 12.0,
            "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
            "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
            "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
        }))
        .unwrap();
        w.start_career(&you, Job::College(p), 7);
        let me = w.career.as_ref().map(|c| c.coach);
        let staff: Vec<usize> = (0..w.coaches.len())
            .filter(|&c| Some(c) != me && w.coaches[c].job.is_some_and(|j| Team::from_employer(j.employer) == team))
            .collect();
        assert!(!staff.is_empty(), "the program has a staff");
        let c = staff[0];
        let said = format!("Ask {} what he thinks.", w.coaches[c].name);
        let r = pool(&w, None).link(&said).expect("a coach on your staff is linked");
        assert!(r.contains(&format!("[[c:{c}|{}]]", w.coaches[c].name)), "{r}");
        let mine = w.coaches[me.unwrap()].name.clone();
        assert!(pool(&w, None).link(&format!("{mine} is the coach.")).is_none(), "you are not a link to yourself");
    }
}
