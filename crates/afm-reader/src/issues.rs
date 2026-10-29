//! The known-issues layer (inventory T1): `data/known_issues.json` read
//! against the world's commit, so every number an issue still touches in
//! this world carries its id.

use rusqlite::params;

use crate::book::Book;

pub const ISSUES: &str = include_str!("../data/known_issues.json");

#[derive(Clone, Debug, Default)]
pub struct Issue {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub live: bool,
    pub live_why: String,
    pub title: String,
    pub touches: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Issues(pub Vec<Issue>);

impl Issues {
    /// The live issues touching a check (by its id or one of its areas).
    pub fn touching(&self, check: &str) -> String {
        let areas: Vec<String> =
            crate::defs::checks().find(|c| c.id == check).map(|c| crate::pages::expand(c.areas)).unwrap_or_default();
        self.0
            .iter()
            .filter(|i| i.live && i.touches.iter().any(|t| t == check || areas.contains(t)))
            .map(|i| i.id.clone())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Whether the world was built with a fix: by git when the commits are
/// here, else by the dates.
fn has_fix(world_commit: &str, written: &str, fix_commit: &str, fix_date: &str) -> (bool, String) {
    if !world_commit.is_empty()
        && world_commit != "unknown"
        && let Ok(o) =
            std::process::Command::new("git").args(["merge-base", "--is-ancestor", fix_commit, world_commit]).output()
    {
        match o.status.code() {
            Some(0) => return (true, format!("the world's commit {} has the fix", short(world_commit))),
            Some(1) => {
                return (false, format!("the world's commit {} is from before the fix", short(world_commit)));
            }
            _ => {}
        }
    }
    let day = written.get(..10).unwrap_or("");
    if day.is_empty() {
        return (false, "the world file says neither commit nor date; counted as live".into());
    }
    if day > fix_date {
        (true, format!("the world was written {day}, after the fix ({fix_date})"))
    } else if day < fix_date {
        (false, format!("the world was written {day}, before the fix ({fix_date})"))
    } else {
        (false, format!("the world was written the day of the fix ({fix_date}); counted as live"))
    }
}

fn short(c: &str) -> &str {
    c.get(..7).unwrap_or(c)
}

pub fn write(b: &Book, world_commit: &str, written: &str) -> rusqlite::Result<Issues> {
    let doc: serde_json::Value = serde_json::from_str(ISSUES).unwrap_or_default();
    b.conn.execute_batch(
        "CREATE TABLE known_issues(id TEXT PRIMARY KEY, kind TEXT, status TEXT, live INTEGER, live_why TEXT, title TEXT,
            system TEXT, location TEXT, levels TEXT, years TEXT, touches TEXT, fix_commit TEXT, fix_date TEXT, fix_pr INTEGER);",
    )?;
    let s = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let mut out = Issues::default();
    for i in doc.get("issues").and_then(|x| x.as_array()).into_iter().flatten() {
        let status = s(i, "status");
        let fix = i.get("fix").and_then(|f| f.as_str()).and_then(|f| doc.pointer(&format!("/fixes/{f}")));
        let (fix_commit, fix_date, fix_pr) = fix.map_or((String::new(), String::new(), None), |f| {
            (s(f, "commit"), s(f, "date"), f.get("pr").and_then(|p| p.as_i64()))
        });
        let (live, why) = match status.as_str() {
            "fixed" => {
                let (has, why) = has_fix(world_commit, written, &fix_commit, &fix_date);
                (!has, why)
            }
            "parked" => (true, "parked: measured, not tuned".to_string()),
            "waiting" => (true, "a trial finding waiting for Cain's yes or no".to_string()),
            _ => (true, "open in the code".to_string()),
        };
        let list = |k: &str| -> Vec<String> {
            i.get(k)
                .and_then(|x| x.as_array())
                .into_iter()
                .flatten()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        };
        let touches = list("touches");
        b.conn.execute(
            "INSERT INTO known_issues VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                s(i, "id"),
                s(i, "kind"),
                status,
                live as i64,
                why,
                s(i, "title"),
                s(i, "system"),
                s(i, "where"),
                list("levels").join(", "),
                s(i, "years"),
                touches.join(" "),
                fix_commit,
                fix_date,
                fix_pr
            ],
        )?;
        out.0.push(Issue {
            id: s(i, "id"),
            kind: s(i, "kind"),
            status,
            live,
            live_why: why,
            title: s(i, "title"),
            touches,
        });
    }
    Ok(out)
}
