//! The squad's quick filter (Cain, 2026-11-04: "which players have sub 60
//! speed on my WRs or CBs"): what each squad row carries for it and the
//! fields the bar offers. Every attribute is your staff's read as a range,
//! never the true number, and the screen matches the range's middle, as
//! the tables sort. Potential is not a field. Production is this season's
//! numbers in your own games, the season the squad boards read (the
//! finder's stat list, `lane_find::fields::STATS`). How it works:
//! `docs/reference/how-it-works/squad-filter.md`.

use std::collections::HashMap;

use afm_engine::attributes::{Attr, Category};
use afm_engine::world::World;
use afm_engine::world::records::{self, STAT_COUNT, Stat};
use serde_json::{Value, json};

use crate::lane_find::fields::STATS;
use crate::reads::Read;

/// Each attribute's read in `Attr::ALL` order: [lo, hi], or null while
/// nobody on your staff has seen it.
pub(crate) fn attrs_json(reads: &[(Attr, Read)]) -> Value {
    json!(reads.iter().map(|x| x.1.json()).collect::<Vec<_>>())
}

/// The player page's eight groups, in its order.
const GROUPS: [(Category, &str); 8] = [
    (Category::Physical, "Physical"),
    (Category::Passing, "Passing"),
    (Category::Running, "Running"),
    (Category::Receiving, "Receiving"),
    (Category::LinePlay, "Line play"),
    (Category::Defense, "Defense"),
    (Category::KickingAndSnapping, "Kicking and snapping"),
    (Category::Mental, "Mental"),
];

/// Rates from the season's totals, each only with the volume it needs
/// (the finder's floors): key, label, the top and the bottom of the rate,
/// the least of the bottom, the scale.
const RATES: [(&str, &str, Stat, Stat, f64, f64); 4] = [
    ("cmp", "Completion %", Stat::Cmp, Stat::Att, 20.0, 100.0),
    ("ypc", "Yards a carry", Stat::RushYds, Stat::Rush, 20.0, 1.0),
    ("ypr", "Yards a catch", Stat::RecYds, Stat::Rec, 5.0, 1.0),
    ("fg", "Field goal %", Stat::FgMade, Stat::FgAtt, 3.0, 100.0),
];

/// The fields the bar offers, by group: your staff's reads (the rating and
/// the 67 attributes), the player's age and body, and, when your games are
/// kept, the season's production (`season`: its words, "this season").
pub(crate) fn catalog(season: Option<&str>) -> Value {
    let mut groups = vec![json!({
        "name": "Your staff's read",
        "fields": [{ "k": "ovr", "label": "Rating", "kind": "read", "key": "ovr" }],
    })];
    for (cat, name) in GROUPS {
        let fields: Vec<Value> = Attr::ALL
            .iter()
            .enumerate()
            .filter(|(_, a)| a.category() == cat)
            .map(|(k, a)| json!({ "k": format!("a{k}"), "label": a.label(), "kind": "read", "key": a.key() }))
            .collect();
        groups.push(json!({ "name": name, "fields": fields }));
    }
    groups.push(json!({
        "name": "Player",
        "fields": [
            { "k": "age", "label": "Age", "kind": "num" },
            { "k": "ht", "label": "Height", "kind": "height" },
            { "k": "wt", "label": "Weight", "kind": "num" },
            { "k": "stars", "label": "Recruiting stars", "kind": "num" },
        ],
    }));
    if let Some(words) = season {
        let mut fields = vec![json!({ "k": "p:g", "label": "Games played", "kind": "num" })];
        fields.extend(STATS.iter().map(|(k, l, _, _)| json!({ "k": format!("p:{k}"), "label": l, "kind": "num" })));
        fields.extend(
            RATES.iter().map(|(k, l, ..)| json!({ "k": format!("p:{k}"), "label": l, "kind": "num", "dp": 1 })),
        );
        groups.push(json!({ "name": format!("Production, {words} in your games"), "fields": fields }));
    }
    json!(groups)
}

/// One player's season in your games: games played and his summed lines.
#[derive(Clone, Copy)]
struct Line {
    games: u32,
    stats: [f64; STAT_COUNT],
}

impl Line {
    fn json(&self) -> Value {
        let mut v = json!({ "g": self.games });
        for (k, _, _, s) in STATS {
            v[k] = json!(self.stats[s as usize]);
        }
        for (k, _, top, bottom, least, scale) in RATES {
            let (t, b) = (self.stats[top as usize], self.stats[bottom as usize]);
            v[k] = json!((b >= least).then(|| (t / b * scale * 10.0).round() / 10.0));
        }
        v
    }
}

/// This season's production for every player in your games, as each
/// squad row carries it, and the season's words; None when no game of
/// yours is kept yet (a career's first spring). The season is this one
/// once it has a game, else the last (as the squad boards read).
pub(crate) struct Production {
    pub words: String,
    by_id: HashMap<u32, Line>,
}

impl Production {
    pub(crate) fn of(w: &World) -> Option<Production> {
        use crate::lane_squadboards::roster::season_of;
        let games = crate::lane_squadboards::my_games(w);
        if games.is_empty() {
            return None;
        }
        let now = season_of(w.today);
        let season = if games.iter().any(|(g, _)| season_of(g.date) == now) { now } else { now - 1 };
        let mut by_id: HashMap<u32, Line> = HashMap::new();
        for (g, side) in games.iter().filter(|(g, _)| season_of(g.date) == season) {
            for (&id, l) in &g.stats.players[*side] {
                let e = by_id.entry(id).or_insert(Line { games: 0, stats: [0.0; STAT_COUNT] });
                if l.snaps_off + l.snaps_def + l.snaps_st > 0 {
                    e.games += 1;
                }
                for (k, x) in records::from_line(l).iter().enumerate() {
                    e.stats[k] += *x as f64;
                }
            }
        }
        let words = if season == now { "this season".to_string() } else { format!("the {season} season") };
        Some(Production { words, by_id })
    }

    /// His numbers: zeros for a player who hasn't played in your games.
    pub(crate) fn of_player(&self, id: u32) -> Value {
        self.by_id.get(&id).copied().unwrap_or(Line { games: 0, stats: [0.0; STAT_COUNT] }).json()
    }
}
