//! MessagePack values (`rmpv::Value`) as JSON text and back: the world file
//! and the world log are MessagePack, the game log and the book's text are
//! JSON.

use rmpv::Value as V;

/// A value as compact JSON. Map keys that are not text become their JSON
/// text; bytes become an array of numbers.
pub fn to_json(v: &V) -> String {
    let mut s = String::new();
    write(v, &mut s);
    s
}

fn write(v: &V, s: &mut String) {
    use std::fmt::Write;
    match v {
        V::Nil => s.push_str("null"),
        V::Boolean(b) => s.push_str(if *b { "true" } else { "false" }),
        V::Integer(i) => {
            let _ = write!(s, "{i}");
        }
        V::F32(f) => num(*f as f64, Some(*f), s),
        V::F64(f) => num(*f, None, s),
        V::String(t) => quote(t.as_str().unwrap_or(""), s),
        V::Binary(b) => {
            s.push('[');
            for (i, x) in b.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                let _ = write!(s, "{x}");
            }
            s.push(']');
        }
        V::Array(a) => {
            s.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                write(x, s);
            }
            s.push(']');
        }
        V::Map(m) => {
            s.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                quote(&key(k), s);
                s.push(':');
                write(x, s);
            }
            s.push('}');
        }
        V::Ext(_, _) => s.push_str("null"),
    }
}

fn num(f: f64, f32v: Option<f32>, s: &mut String) {
    use std::fmt::Write;
    if !f.is_finite() {
        s.push_str("null");
    } else if let Some(x) = f32v {
        // An f32 with the digits it really has.
        let _ = write!(s, "{x}");
    } else if f.abs() < 1e7 {
        // Seven significant digits, as the world log keeps them.
        let _ = write!(s, "{}", f as f32);
    } else {
        let _ = write!(s, "{f}");
    }
}

/// A map key as text.
pub fn key(k: &V) -> String {
    match k {
        V::String(t) => t.as_str().unwrap_or("").to_string(),
        other => to_json(other),
    }
}

fn quote(t: &str, s: &mut String) {
    s.push('"');
    for c in t.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                let _ = write!(s, "\\u{:04x}", c as u32);
            }
            c => s.push(c),
        }
    }
    s.push('"');
}

/// A JSON value as MessagePack.
pub fn from_json(j: &serde_json::Value) -> V {
    use serde_json::Value as J;
    match j {
        J::Null => V::Nil,
        J::Bool(b) => V::Boolean(*b),
        J::Number(n) => {
            if let Some(i) = n.as_i64() {
                V::from(i)
            } else if let Some(u) = n.as_u64() {
                V::from(u)
            } else {
                V::F64(n.as_f64().unwrap_or(0.0))
            }
        }
        J::String(t) => V::from(t.as_str()),
        J::Array(a) => V::Array(a.iter().map(from_json).collect()),
        J::Object(o) => V::Map(o.iter().map(|(k, x)| (V::from(k.as_str()), from_json(x))).collect()),
    }
}

/// A MessagePack value as a JSON value.
pub fn to_value(v: &V) -> serde_json::Value {
    serde_json::from_str(&to_json(v)).unwrap_or(serde_json::Value::Null)
}

/// A field of a map by name.
pub fn get<'a>(v: &'a V, name: &str) -> Option<&'a V> {
    match v {
        V::Map(m) => m.iter().find(|(k, _)| k.as_str() == Some(name)).map(|(_, x)| x),
        V::Array(a) => a.get(name.parse::<usize>().ok()?),
        _ => None,
    }
}

/// A field down a path of names (`a.b.c`; a number picks a list's item).
pub fn path<'a>(v: &'a V, p: &str) -> Option<&'a V> {
    p.split('.').try_fold(v, |v, k| get(v, k))
}

pub fn as_f64(v: &V) -> Option<f64> {
    match v {
        V::Integer(i) => i.as_f64(),
        V::F32(f) => Some(*f as f64),
        V::F64(f) => Some(*f),
        _ => None,
    }
}

pub fn as_i64(v: &V) -> Option<i64> {
    match v {
        V::Integer(i) => i.as_i64(),
        V::F32(f) => Some(*f as i64),
        V::F64(f) => Some(*f as i64),
        _ => None,
    }
}
