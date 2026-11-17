//! A check on the JSON schemas sent as structured output
//! (`output_config.format.schema`). The API refuses the whole call with a 400
//! when a schema breaks its rules, so every schema the game sends is checked
//! in the tests first (`afm-game`, `live_talk::schema_tests`).
//!
//! The rules checked:
//! - an `enum` sits on one plain `type` (never a list like
//!   `["string", "null"]`), and every value in it is of that type. A field
//!   that may also be null is `anyOf` a typed enum and `{"type": "null"}`
//!   (`nullable_enum`);
//! - an object with properties has `additionalProperties: false`;
//! - every `required` name is one of the object's properties.

use serde_json::{Value, json};

/// A field that is one of `values` or null, the way the API takes it.
pub fn nullable_enum(ty: &str, values: Vec<Value>) -> Value {
    json!({ "anyOf": [{ "type": ty, "enum": values }, { "type": "null" }] })
}

/// Every place where `schema` breaks the API's rules, each with its path.
/// Empty when the schema is fine.
pub fn schema_problems(schema: &Value) -> Vec<String> {
    let mut out = Vec::new();
    walk(schema, "$", &mut out);
    out
}

fn of_type(v: &Value, ty: &str) -> bool {
    match ty {
        "string" => v.is_string(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        "object" => v.is_object(),
        "array" => v.is_array(),
        _ => false,
    }
}

fn walk(node: &Value, path: &str, out: &mut Vec<String>) {
    let Some(obj) = node.as_object() else { return };
    if let Some(vals) = obj.get("enum") {
        match obj.get("type") {
            Some(Value::String(ty)) => {
                for v in vals.as_array().into_iter().flatten() {
                    if !of_type(v, ty) {
                        out.push(format!("{path}: enum value {v} is not a {ty}"));
                    }
                }
            }
            Some(Value::Array(_)) => {
                out.push(format!("{path}: an enum on a list of types (use anyOf a typed enum and null)"))
            }
            _ => {}
        }
    }
    let is_object = match obj.get("type") {
        Some(Value::String(t)) => t == "object",
        Some(Value::Array(ts)) => ts.iter().any(|t| t == "object"),
        _ => false,
    };
    if let Some(props) = obj.get("properties").and_then(Value::as_object) {
        if is_object && obj.get("additionalProperties") != Some(&Value::Bool(false)) {
            out.push(format!("{path}: an object without additionalProperties: false"));
        }
        for r in obj.get("required").and_then(Value::as_array).into_iter().flatten() {
            if !r.as_str().is_some_and(|k| props.contains_key(k)) {
                out.push(format!("{path}: required {r} is not a property"));
            }
        }
        for (k, v) in props {
            walk(v, &format!("{path}.{k}"), out);
        }
    }
    if let Some(items) = obj.get("items") {
        walk(items, &format!("{path}[]"), out);
    }
    for key in ["anyOf", "oneOf", "allOf"] {
        for (i, v) in obj.get(key).and_then(Value::as_array).into_iter().flatten().enumerate() {
            walk(v, &format!("{path}.{key}[{i}]"), out);
        }
    }
    for key in ["$defs", "definitions"] {
        for (k, v) in obj.get(key).and_then(Value::as_object).into_iter().flatten() {
            walk(v, &format!("{path}.{key}.{k}"), out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refused_shape_is_caught() {
        // The 400 from "Ask in your words": an enum with null on ["string", "null"].
        let bad = json!({
            "type": "object", "additionalProperties": false, "required": ["level"],
            "properties": { "level": { "type": ["string", "null"], "enum": ["hs", "juco", null] } }
        });
        let p = schema_problems(&bad);
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].starts_with("$.level"), "{p:?}");
    }

    #[test]
    fn the_fixed_shape_passes() {
        let good = json!({
            "type": "object", "additionalProperties": false, "required": ["level"],
            "properties": { "level": nullable_enum("string", vec![json!("hs"), json!("juco")]) }
        });
        assert!(schema_problems(&good).is_empty());
    }

    #[test]
    fn other_breaks_are_caught() {
        let bad = json!({
            "type": "object", "required": ["a", "b"],
            "properties": {
                "a": { "type": "string", "enum": ["x", 3] },
                "c": { "type": "array", "items": { "type": "object", "properties": { "d": { "type": "string" } } } }
            }
        });
        let p = schema_problems(&bad);
        assert_eq!(p.len(), 4, "{p:?}");
    }
}
