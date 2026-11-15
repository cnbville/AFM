//! The analyst's weekly leads (live talks lane 4, LT21). Every Monday the
//! game computes the outliers on your seat's numbers (`outliers`): in each
//! position group's table (`analytics`, the Data section's percentiles among
//! that season's qualifiers at the level), the players whose best numbers sit
//! at the top of the level, adjusted for age (a younger player gets a small
//! capped credit), minus players already on your board, your list and your
//! own squad. `lane_core/mail.rs` writes them as a mail with the game's own
//! top few in a table of linked text: that is the whole mail with no key.
//!
//! With a key, opening the mail asks once for the writer's part (`ask`): he
//! picks the best few of the game's outliers and says why each stands out.
//! The answer is stored on the mail under the lock (`Mail.leads` goes from 1
//! to 2), and the letter shows it from then on. No call is made while the
//! clock runs: the mail is filed with no call, and the one call waits for you
//! to open it. All sizes here are proposed, not confirmed.

use std::collections::HashSet;

use afm_ai::Request;
use afm_engine::world::analytics as an;
use afm_engine::world::desk::Mail;
use afm_engine::world::marks::Tier;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::analyst::{READER_TOKENS, start, text_of};
use crate::lane_core::links;
use crate::live_talk::filter;
use crate::live_talk::tickets::{self, Next, Step};
use crate::views::{self, TeamRef};
use crate::{Game, Reply};

pub const WRITER: &str = "leads_writer";

/// The most leads in a mail, and the most from one position group.
pub const MAX_LEADS: usize = 8;
const PER_GROUP: usize = 2;
/// The top of a player's best numbers must sit at or above this percentile
/// among his season's qualifiers to be an outlier.
const TOP_PCT: f64 = 90.0;
/// The most the writer may pick.
const MAX_PICKS: usize = 5;
/// The most percentile points an age credit may add or take away, and the
/// age (college, then the NFL) a player is level with.
const AGE_CAP: f64 = 4.0;
const AGE_PER_YEAR: f64 = 1.5;

/// One outlier: who, where, and his numbers in words.
pub struct Lead {
    pub id: u32,
    pub name: String,
    pub pos: &'static str,
    pub team: Option<TeamRef>,
    /// "Junior, 20" or "3rd year, 24".
    pub class: String,
    pub score: f64,
    /// His two best numbers: "Yards per route run 2.9, 97th percentile".
    pub best: Vec<String>,
}

fn ordinal(n: i64) -> String {
    let s = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

/// Everyone your staff already has in view: your squad, your list, your
/// recruiting board or draft board.
fn known(w: &World, me: TeamRef) -> HashSet<u32> {
    let mut out: HashSet<u32> = views::roster_people(w, me).into_iter().map(|k| w.people[k].player.id).collect();
    if let Some(c) = w.career.as_ref() {
        out.extend(c.desk.listed.iter().map(|l| l.id));
    }
    out.extend(w.front_office.college.board.iter().map(|t| t.id));
    out.extend(w.scouting.board.order.iter().copied());
    out
}

fn team_of(w: &World, k: usize) -> Option<TeamRef> {
    let p = &w.people[k];
    match p.stage {
        Stage::College => p.college.map(TeamRef::College),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map(TeamRef::Nfl),
        Stage::HighSchool => Some(TeamRef::HighSchool(p.hs)),
        _ => None,
    }
}

/// The game's outliers on your seat's numbers, best first (at most
/// `MAX_LEADS`, two to a position group). Reads only the Data section's own
/// tables: the numbers any page of it shows, never a rating.
pub fn outliers(w: &World, me: TeamRef) -> Vec<Lead> {
    let tier = match me {
        TeamRef::College(_) => Tier::College,
        TeamRef::Nfl(_) => Tier::Nfl,
        TeamRef::HighSchool(_) => Tier::Hs,
        TeamRef::Other(_) => return Vec::new(),
    };
    let norm = if tier == Tier::Nfl {
        25.0
    } else if tier == Tier::College {
        21.0
    } else {
        16.0
    };
    let skip = known(w, me);
    let mut all: Vec<(f64, usize, Lead)> = Vec::new();
    for (gi, group) in an::GROUPS.iter().enumerate() {
        if matches!(*group, "K" | "P" | "RET") {
            continue;
        }
        let t = crate::lane_data::analytics::table(w, &json!({ "group": gi, "limit": 500 }));
        let cols = t.get("columns").and_then(Value::as_array).cloned().unwrap_or_default();
        let rows = t.get("rows").and_then(Value::as_array).cloned().unwrap_or_default();
        let mut found: Vec<(f64, Lead)> = Vec::new();
        for r in &rows {
            let Some(id) = r.get("id").and_then(Value::as_u64).map(|x| x as u32) else { continue };
            let Some(k) = views::person_index(w, id) else { continue };
            if skip.contains(&id) {
                continue;
            }
            let pct = r.get("pct").and_then(Value::as_array).cloned().unwrap_or_default();
            let mut ranked: Vec<(usize, i64)> = pct
                .iter()
                .enumerate()
                .filter(|(c, _)| cols.get(*c).and_then(|x| x.get("better")).and_then(Value::as_str) != Some("none"))
                .filter_map(|(c, x)| x.as_i64().map(|x| (c, x)))
                .collect();
            ranked.sort_by_key(|x| std::cmp::Reverse(x.1));
            let Some(&(_, top)) = ranked.first() else { continue };
            if (top as f64) < TOP_PCT {
                continue;
            }
            let mean = ranked.iter().take(2).map(|x| x.1 as f64).sum::<f64>() / ranked.len().min(2) as f64;
            let p = &w.people[k];
            let age = p.age_on(w.today) as f64;
            let score = mean + ((norm - age) * AGE_PER_YEAR).clamp(-AGE_CAP, AGE_CAP);
            let cells = r.get("cells").and_then(Value::as_array);
            let best: Vec<String> = ranked
                .iter()
                .take(2)
                .map(|&(c, pc)| {
                    let label = cols.get(c).and_then(|x| x.get("label")).and_then(Value::as_str).unwrap_or("");
                    let cell = cells.and_then(|a| a.get(c)).and_then(Value::as_str).unwrap_or("-");
                    format!("{label} {cell}, {} percentile", ordinal(pc))
                })
                .collect();
            let class = match tier {
                Tier::Nfl => format!("Age {age:.0}"),
                _ => format!("{}, {age:.0}", crate::views::class_text(w, p)),
            };
            found.push((
                score,
                Lead {
                    id,
                    name: p.name.clone(),
                    pos: p.player.position.name(),
                    team: team_of(w, k),
                    class,
                    score,
                    best,
                },
            ));
        }
        found.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
        for (s, l) in found.into_iter().take(PER_GROUP) {
            all.push((s, gi, l));
        }
    }
    all.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.2.id.cmp(&b.2.id)));
    all.into_iter().take(MAX_LEADS).map(|x| x.2).collect()
}

const RULES: &str = "You are a football analyst's voice, picking leads for a head coach from a list the game has made. The game has already found the outliers: players whose best numbers sit at the top of the level. The card lists them, each with an id (l1, l2 and so on) and his best numbers. Pick the few that deserve the coach's time first (at most five) and say, in one short sentence each, why that player stands out, in plain football words, using only the numbers and facts on the card, copied as printed. Never add a number, a name or a fact that is not in the card. Never mention ratings, potential or anything about the game itself. Order the picks best first. Answer only with the JSON the schema allows.";

fn schema() -> Value {
    let ids: Vec<String> = (1..=MAX_LEADS).map(|k| format!("l{k}")).collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["picks"],
        "properties": {
            "picks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["id", "why"],
                    "properties": { "id": { "type": "string", "enum": ids }, "why": { "type": "string" } }
                }
            }
        }
    })
}

/// A mail's leads, from its own table: the id, his words without the link
/// markup, and the numbers text. The table's rows are the mail's own, so the
/// writer sees exactly what the letter shows.
fn rows_of(m: &Mail) -> Vec<(u32, String, String)> {
    m.table
        .iter()
        .skip(1)
        .zip(&m.ids)
        .map(|(row, id)| {
            let cell = |k: usize| row.get(k).map(|c| links::strip(c)).unwrap_or_default();
            (*id, format!("{}, {}, {}, {}", cell(0), cell(1), cell(2), cell(3)), cell(4))
        })
        .collect()
}

fn mail_id(args: &Value) -> Result<u64, String> {
    match args.get("mail") {
        Some(Value::String(s)) => s.parse().map_err(|_| "No such mail.".to_string()),
        Some(Value::Number(n)) => n.as_u64().ok_or_else(|| "No such mail.".to_string()),
        _ => Err("Missing mail.".into()),
    }
}

fn find(g: &Game, id: u64) -> Option<&Mail> {
    g.world.as_ref()?.career.as_ref()?.desk.mails.iter().find(|m| m.id == id)
}

/// Writes the writer's picks into the mail, under the lock: the picks first,
/// each with its why, the rest after with the game's numbers; `leads` 2.
fn store(g: &mut Game, id: u64, picks: &[(usize, String)]) {
    let Some(c) = g.world.as_mut().and_then(|w| w.career.as_mut()) else { return };
    let Some(m) = c.desk.mails.iter_mut().find(|m| m.id == id) else { return };
    if m.leads != 1 || m.table.len() < 2 {
        return;
    }
    let rows: Vec<Vec<String>> = m.table[1..].to_vec();
    let ids = m.ids.clone();
    let mut head = m.table[0].clone();
    if head.len() >= 5 {
        head[4] = "Why he stands out".into();
    }
    let mut table = vec![head];
    let mut out_ids = Vec::new();
    let mut used = HashSet::new();
    for (k, why) in picks {
        let Some(row) = rows.get(*k) else { continue };
        let mut row = row.clone();
        if row.len() >= 5 {
            row[4] = format!("{why} ({})", row[4]);
        }
        table.push(row);
        out_ids.push(ids.get(*k).copied().unwrap_or(0));
        used.insert(*k);
    }
    let first = out_ids.len();
    for (k, row) in rows.iter().enumerate().filter(|(k, _)| !used.contains(k)) {
        table.push(row.clone());
        out_ids.push(ids.get(k).copied().unwrap_or(0));
    }
    m.table = table;
    m.ids = out_ids;
    m.body.push(format!(
        "I put the {first} I would look at first at the top, each with why. The rest are the other outliers this week."
    ));
    m.leads = 2;
}

/// `ai_ask {kind: "leads", mail}`: once, as the mail opens.
pub fn ask(g: &mut Game, args: &Value) -> Reply {
    let id = mail_id(args)?;
    let m = find(g, id).ok_or("That mail has cleared.")?;
    if m.leads != 1 {
        // Already written, or not a leads mail: the page just reads it again.
        let t = tickets::ready(g, json!({ "state": "done", "reload": true }));
        return Ok(json!({ "ticket": t }));
    }
    let rows = rows_of(m);
    if rows.is_empty() {
        return Err("The mail holds no leads.".into());
    }
    let mut card = String::from("The outliers this week:\n");
    for (k, (_, who, nums)) in rows.iter().enumerate() {
        card.push_str(&format!("l{}: {who}. His best numbers: {nums}\n", k + 1));
    }
    let req = Request::new(WRITER, RULES, &card).schema(schema()).max_tokens(READER_TOKENS);
    let next: Next = Box::new(move |g, r| {
        // The mail is filled whatever comes back: a failed writer leaves the
        // game's own table and the mail reads again as it was.
        let reply = match r {
            Ok(x) => x,
            Err(f) => return Step::off(&f),
        };
        let v: Value = serde_json::from_str(&reply.text).unwrap_or(Value::Null);
        let mut picks: Vec<(usize, String)> = Vec::new();
        for p in v.get("picks").and_then(Value::as_array).into_iter().flatten().take(MAX_PICKS) {
            let Some(k) = text_of(p, "id").and_then(|s| s.strip_prefix('l')).and_then(|n| n.parse::<usize>().ok())
            else {
                continue;
            };
            if k == 0 || k > rows.len() || picks.iter().any(|x| x.0 == k - 1) {
                continue;
            }
            // Words that fail the filter give way to the game's own numbers.
            let why =
                text_of(p, "why").and_then(|w| filter::check(w, &card).ok()).unwrap_or_else(|| rows[k - 1].2.clone());
            picks.push((k - 1, why));
        }
        if picks.is_empty() {
            picks = (0..rows.len().min(3)).map(|k| (k, rows[k].2.clone())).collect();
        }
        store(g, id, &picks);
        Step::Done(json!({ "state": "done", "reload": true }))
    });
    start(g, req, next)
}

/// Adds this ask's entry to a reply's `ask` list (`live_talk::after`, only
/// with a usable key; `super::push` adds one): on the letter of a leads mail
/// whose writer's part is still wanted.
pub fn flag(g: &Game, cmd: &str, v: &mut Value) {
    if cmd != "letter" {
        return;
    }
    let Some(id) = v.get("id").and_then(Value::as_str).and_then(|s| s.parse::<u64>().ok()) else { return };
    if find(g, id).is_some_and(|m| m.leads == 1) {
        super::push(v, json!({ "cmd": "ai_ask", "args": { "kind": "leads", "mail": id.to_string() }, "auto": true }));
    }
}

#[cfg(test)]
mod tests {
    use super::ordinal;

    #[test]
    fn ordinals_read_right() {
        let s: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 92, 97, 100].iter().map(|&n| ordinal(n)).collect();
        assert_eq!(s, ["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "92nd", "97th", "100th"]);
    }
}
