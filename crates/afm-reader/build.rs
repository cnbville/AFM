//! The knob table (world reader inventory T2), read at build time from the
//! "Where the knobs are" section of every how-it-works page and the code
//! they point to: each constant's value as written, its file and whether
//! its comment says it is proposed, confirmed or taken from real data.
//! Written to `$OUT_DIR/knobs.json` with the commit it was read at.

use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let pages = root.join("docs/reference/how-it-works");
    println!("cargo:rerun-if-changed={}", pages.display());
    println!("cargo:rerun-if-changed={}", root.join("crates/afm-engine/src").display());
    println!("cargo:rerun-if-env-changed=AFM_COMMIT");
    let mut knobs = Vec::new();
    let mut names: Vec<PathBuf> =
        std::fs::read_dir(&pages).map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).collect()).unwrap_or_default();
    names.sort();
    for page in names {
        if page.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = std::fs::read_to_string(&page).unwrap_or_default();
        let Some(start) = text.find("## Where the knobs are") else { continue };
        let body = &text[start + 22..];
        let body = &body[..body.find("\n## ").unwrap_or(body.len())];
        let system = page.file_stem().unwrap().to_string_lossy().to_string();
        for line in body.lines().filter(|l| l.trim_start().starts_with("- ")) {
            let ticks: Vec<&str> = line.split('`').skip(1).step_by(2).collect();
            let Some(file) = ticks.first().filter(|f| f.ends_with(".rs") || f.contains('/')) else { continue };
            let path = locate(&root, file);
            let src = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
            for name in &ticks[1..] {
                let ident = name.split(['.', '(', ' ']).next().unwrap_or("");
                if ident.is_empty() || !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    continue;
                }
                let (value, status) = find(&src, ident);
                knobs.push(serde_json(&[
                    ("name", name.to_string()),
                    ("file", path.as_ref().map_or(file.to_string(), |p| rel(&root, p))),
                    ("system", system.clone()),
                    ("value", value),
                    ("status", status),
                    ("note", line.trim_start_matches(['-', ' ']).to_string()),
                ]));
            }
        }
    }
    let commit = std::env::var("AFM_COMMIT").ok().filter(|c| !c.is_empty()).or_else(|| {
        std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    });
    let out = format!("{{\"commit\":{},\"knobs\":[{}]}}", quote(&commit.unwrap_or_default()), knobs.join(","));
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("knobs.json");
    std::fs::write(dest, out).unwrap();
    real_files(&root);
}

/// Every real-data JSON file, and every CSV under `CSV_MAX` bytes, built
/// in, so a book carries the real figures even when it is built on a Mac
/// with no copy of the repository.
fn real_files(root: &Path) {
    // The biggest CSVs (town climates, every high school town) stay out.
    const CSV_MAX: u64 = 250_000;
    let dir = root.join("data/real-world");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<PathBuf> =
        std::fs::read_dir(&dir).map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).collect()).unwrap_or_default();
    files.retain(|p| match p.extension().and_then(|e| e.to_str()) {
        Some("json") => true,
        Some("csv") => std::fs::metadata(p).is_ok_and(|m| m.len() <= CSV_MAX),
        _ => false,
    });
    files.sort();
    let mut out = String::from("pub const FILES: &[(&str, &str)] = &[\n");
    for f in files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let abs = f.canonicalize().unwrap_or(f.clone());
        out.push_str(&format!("    ({name:?}, include_str!({:?})),\n", abs.display().to_string()));
    }
    out.push_str("];\n");
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("real_files.rs"), out).unwrap();
}

fn locate(root: &Path, file: &str) -> Option<PathBuf> {
    let file = file.trim_end_matches(':');
    for base in ["", "crates/afm-engine/src/", "crates/afm-game/src/", "crates/"] {
        let p = root.join(base).join(file);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn rel(root: &Path, p: &Path) -> String {
    let r = root.canonicalize().unwrap_or(root.to_path_buf());
    let p = p.canonicalize().unwrap_or(p.to_path_buf());
    p.strip_prefix(&r).map_or_else(|_| p.display().to_string(), |x| x.display().to_string())
}

/// A constant's value as written and what its comment says about it.
fn find(src: &str, ident: &str) -> (String, String) {
    let lines: Vec<&str> = src.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        let t = t.strip_prefix("pub(crate) ").or_else(|| t.strip_prefix("pub ")).unwrap_or(t);
        let is_def = (t.starts_with("const ") || t.starts_with("static ") || t.starts_with("fn "))
            && t.split([' ', ':', '(', '<']).nth(1) == Some(ident);
        if !is_def {
            continue;
        }
        let comment: String = lines[i.saturating_sub(8)..i]
            .iter()
            .rev()
            .take_while(|l| l.trim_start().starts_with("//") || l.trim_start().starts_with("#["))
            .map(|l| l.trim_start().trim_start_matches('/').trim())
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let status = if comment.contains("proposed") || comment.contains("not confirmed") {
            "proposed, not confirmed"
        } else if comment.contains("raichel")
            || comment.contains("cain")
            || comment.contains("agreed")
            || comment.contains("confirmed")
        {
            "confirmed"
        } else if ["nflverse", "cfbd", "cfbfastr", "real ", "source", "census", "ncaa", "espn", "247"]
            .iter()
            .any(|k| comment.contains(k))
        {
            "from real data"
        } else {
            "not stated"
        };
        if t.starts_with("fn ") {
            return ("function".into(), status.into());
        }
        let mut v = String::new();
        for l in &lines[i..lines.len().min(i + 40)] {
            v.push_str(l.trim());
            v.push(' ');
            if l.trim_end().ends_with(';') {
                break;
            }
        }
        let v = v.split_once('=').map_or(v.as_str(), |(_, x)| x).trim().trim_end_matches(';').trim();
        let v: String = v.chars().take(400).collect();
        return (v, status.into());
    }
    ("not found in the file".into(), "not stated".into())
}

fn quote(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn serde_json(fields: &[(&str, String)]) -> String {
    let parts: Vec<String> = fields.iter().map(|(k, v)| format!("{}:{}", quote(k), quote(v))).collect();
    format!("{{{}}}", parts.join(","))
}
