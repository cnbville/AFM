//! The real index (world reader lane R3, `data/real-world/index.json`) into
//! the book, so a book carries the real figures it was checked against.

use rmpv::Value as V;
use rusqlite::types::Value as S;

use crate::json;
use crate::sink::Writer;

// `FILES`: every `data/real-world/*.json` as the reader was built with it.
include!(concat!(env!("OUT_DIR"), "/real_files.rs"));

/// A real-data file's text by name.
pub fn file(name: &str) -> Option<&'static str> {
    FILES.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
}

/// The real index (`index.json`).
pub fn index() -> serde_json::Value {
    file("index.json").and_then(|t| serde_json::from_str(t).ok()).unwrap_or_default()
}

pub fn read(w: &mut Writer, index: &serde_json::Value) -> rusqlite::Result<u64> {
    w.source = "real index".into();
    let mut n = 0;
    if let Some(figs) = index.get("figures").and_then(|f| f.as_array()) {
        for f in figs {
            let mut fixed: Vec<(String, S)> = Vec::new();
            let mut rest = serde_json::Map::new();
            for (k, x) in f.as_object().into_iter().flatten() {
                match (k.as_str(), x) {
                    ("value", serde_json::Value::Number(v)) => {
                        fixed.push(("value".into(), S::Real(v.as_f64().unwrap_or(0.0))));
                    }
                    ("value", other) => fixed.push(("value_json".into(), S::Text(other.to_string()))),
                    ("areas" | "columns", other) => fixed.push((k.clone(), S::Text(other.to_string()))),
                    _ => {
                        rest.insert(k.clone(), x.clone());
                    }
                }
            }
            w.put("real_figures", fixed, &json::from_json(&serde_json::Value::Object(rest)))?;
            n += 1;
        }
    }
    if let Some(files) = index.get("files").and_then(|f| f.as_object()) {
        for (name, f) in files {
            let mut f = f.clone();
            if let Some(a) = f.get_mut("areas") {
                *a = serde_json::Value::String(a.to_string());
            }
            w.put("real_files", vec![("file".into(), S::Text(name.clone()))], &json::from_json(&f))?;
        }
    }
    for list in ["in_code", "debug_lab", "memory_checks", "memory_notes"] {
        for x in index.get(list).and_then(|f| f.as_array()).into_iter().flatten() {
            let mut x = x.clone();
            if let Some(a) = x.get_mut("areas") {
                *a = serde_json::Value::String(a.to_string());
            }
            w.put(&format!("real_{list}"), Vec::new(), &json::from_json(&x))?;
        }
    }
    let about = V::Map(
        ["about", "status_words", "counts"]
            .iter()
            .filter_map(|k| index.get(*k).map(|x| (V::from(*k), V::from(x.to_string()))))
            .collect(),
    );
    w.put("real_about", Vec::new(), &about)?;
    for (name, text) in FILES {
        w.insert(
            "real_data",
            None,
            vec![("file".into(), S::Text(name.to_string())), ("json".into(), S::Text(text.to_string()))],
        )?;
    }
    w.flush()?;
    Ok(n)
}

/// A real figure's value by its key (`file:path`), and where the index
/// keeps a bigger value, a path into it (`file:path#a.b`).
pub fn figure(index: &serde_json::Value, key: &str) -> Option<(serde_json::Value, serde_json::Value)> {
    let (k, sub) = key.split_once('#').map_or((key, None), |(a, b)| (a, Some(b)));
    let f = index.get("figures")?.as_array()?.iter().find(|f| f.get("key").and_then(|x| x.as_str()) == Some(k))?;
    let mut v = match f.get("value") {
        Some(v) if !v.is_null() => v.clone(),
        // A big figure: read it from its file.
        _ => {
            let (fname, inner) = k.split_once(':')?;
            let mut v: serde_json::Value = serde_json::from_str(file(fname)?).ok()?;
            for p in inner.split('.') {
                v = v.get(p)?.clone();
            }
            v
        }
    };
    if let Some(sub) = sub {
        for p in sub.split('.') {
            v = match &v {
                serde_json::Value::Array(a) => a.get(p.parse::<usize>().ok()?)?.clone(),
                _ => v.get(p)?.clone(),
            };
        }
    }
    Some((v, f.clone()))
}
