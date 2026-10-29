//! The reader's topic pages and the inventory areas each one shows
//! (spec section 7), and the inventory itself (207 areas, each with what it
//! shows, its checks in words, where its data is, its real references and
//! what is stored nowhere yet).

pub struct Page {
    pub id: &'static str,
    pub title: &'static str,
    pub areas: &'static str,
}

pub const PAGES: &[Page] = &[
    Page { id: "overview", title: "Overview", areas: "" },
    Page { id: "people", title: "People", areas: "P1-P24 P27-P32 T8" },
    Page { id: "recruiting", title: "Recruiting", areas: "C18 C28-C41 P25 P26 T7" },
    Page { id: "programs", title: "Programs", areas: "C1-C17 C19-C22 C42 T13" },
    Page { id: "high-school", title: "High school", areas: "C23-C27 G7 G27 N32" },
    Page { id: "coaches", title: "Coaches", areas: "S1-S31 S33" },
    Page { id: "games", title: "Games and stats", areas: "G1-G6 G8-G26 G28-G38" },
    Page { id: "nfl", title: "NFL", areas: "N1-N29 S34" },
    Page { id: "money", title: "Money", areas: "N30 N31 N33 C14 C41" },
    Page { id: "world", title: "World and time", areas: "N34-N44 N48 T9" },
    Page { id: "career", title: "Your career", areas: "N45-N47 S32" },
    Page { id: "tools", title: "Tools", areas: "T1-T6 T10-T12" },
];

/// The inventory as the reader was built with it.
pub const INVENTORY: &str = include_str!("../data/inventory.json");

/// Expands "P1-P3 T8" into P1, P2, P3, T8.
pub fn expand(areas: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in areas.split_whitespace() {
        match part.split_once('-') {
            Some((a, b)) => {
                let (pa, na) = split(a);
                let (_, nb) = split(b);
                for n in na..=nb {
                    out.push(format!("{pa}{n}"));
                }
            }
            None => out.push(part.to_string()),
        }
    }
    out
}

fn split(code: &str) -> (&str, u32) {
    let i = code.find(|c: char| c.is_ascii_digit()).unwrap_or(code.len());
    (&code[..i], code[i..].parse().unwrap_or(0))
}

/// The page an area is shown on.
pub fn page_of(area: &str) -> &'static str {
    PAGES.iter().find(|p| expand(p.areas).iter().any(|a| a == area)).map_or("tools", |p| p.id)
}

/// Every inventory area with its code: (code, area JSON).
pub fn areas() -> Vec<(String, serde_json::Value)> {
    let inv: serde_json::Value = serde_json::from_str(INVENTORY).unwrap_or_default();
    let mut out = Vec::new();
    let letters = ["P", "C", "S", "G", "N"];
    for (i, part) in inv.get("parts").and_then(|p| p.as_array()).into_iter().flatten().enumerate() {
        let l = letters.get(i).copied().unwrap_or("X");
        for (j, a) in part.get("areas").and_then(|a| a.as_array()).into_iter().flatten().enumerate() {
            out.push((format!("{l}{}", j + 1), a.clone()));
        }
    }
    for (j, a) in inv.pointer("/missing/areas").and_then(|a| a.as_array()).into_iter().flatten().enumerate() {
        out.push((format!("T{}", j + 1), a.clone()));
    }
    out
}
