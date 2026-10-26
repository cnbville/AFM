//! The filter builder (FS1): any field as a filter, at least, at most,
//! between or is, the filters joined by "and" and "or".
//!
//! The screen sends a list of chips, each joined to the one before by "and"
//! (a new group) or "or" (the same group): `(A or B) and C`. Every chip is
//! read back with its words, so the page shows the search as the game read
//! it.

use serde_json::{Value, json};

use super::fields::{Ctx, FKind, Field, Group, Val, field_of, value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    AtLeast,
    AtMost,
    Between,
    Is,
}

impl Op {
    fn parse(s: &str) -> Op {
        match s {
            "ge" | "at least" => Op::AtLeast,
            "le" | "at most" => Op::AtMost,
            "between" => Op::Between,
            _ => Op::Is,
        }
    }
    fn key(self) -> &'static str {
        match self {
            Op::AtLeast => "ge",
            Op::AtMost => "le",
            Op::Between => "between",
            Op::Is => "is",
        }
    }
}

/// One filter: a field, a test and its value (numbers for numbers, words
/// and yes or no; text for a name or code).
#[derive(Clone, Debug)]
pub struct Term {
    pub field: Field,
    pub op: Op,
    pub a: f64,
    pub b: f64,
    pub text: String,
    /// Joined to the filter before by "or".
    pub or: bool,
    /// A text filter's name in lower case, its words single-spaced.
    pub want: String,
    /// Whether `want` is a whole name somewhere (a program, a conference
    /// or a value in the pool): then only that name matches, so "Michigan"
    /// never finds Michigan State. Set by `Query::settled`.
    pub exact: bool,
}

/// A name in lower case with its words single-spaced.
pub fn norm(s: &str) -> String {
    s.split_whitespace().map(str::to_lowercase).collect::<Vec<_>>().join(" ")
}

/// Whether a line holds the name `want` (both as `norm` gives them). The
/// whole name always matches. When `want` is no whole name anywhere
/// (`exact` off), its words match the line's words from any word start,
/// the last one as far as it was typed: "ohio st" finds Ohio State and
/// "lyle" finds Andre Lyle, while "chigan" finds nothing.
pub fn names_match(line: &str, want: &str, exact: bool) -> bool {
    let line = norm(line);
    if want.is_empty() {
        return false;
    }
    if line == want || exact {
        return line == want;
    }
    let have: Vec<&str> = line.split(' ').collect();
    let typed: Vec<&str> = want.split(' ').collect();
    (0..have.len()).any(|s| {
        s + typed.len() <= have.len()
            && typed.iter().enumerate().all(|(k, t)| {
                let h = have[s + k];
                if k + 1 == typed.len() { h.starts_with(t) } else { h == *t }
            })
    })
}

impl Term {
    /// The filter in words: "Composite stars at least 3", "State is SC".
    pub fn words(&self) -> String {
        let f = &self.field;
        let shown = |x: f64| match f.kind {
            FKind::Word => f.opts.get(x as usize).copied().unwrap_or("?").to_string(),
            FKind::Flag => if x >= 0.5 { "yes" } else { "no" }.to_string(),
            _ => fmt_num(x, f.dp),
        };
        let v = match (f.kind, self.op) {
            (FKind::Text, _) => format!("is {}", self.text),
            (_, Op::AtLeast) => format!("at least {}", shown(self.a)),
            (_, Op::AtMost) => format!("at most {}", shown(self.a)),
            (_, Op::Between) => format!("between {} and {}", shown(self.a), shown(self.b)),
            (_, Op::Is) => format!("is {}", shown(self.a)),
        };
        format!("{} {v}", f.label)
    }

    pub fn json(&self) -> Value {
        let v: Value = if self.field.kind == FKind::Text { json!(self.text) } else { json!(self.a) };
        json!({ "k": self.field.k, "op": self.op.key(), "v": v, "v2": self.b, "or": self.or, "label": self.field.label, "words": self.words(), "kind": match self.field.kind { FKind::Num => "num", FKind::Word => "word", FKind::Flag => "flag", FKind::Text => "text" } })
    }

    fn pass(&self, ctx: &Ctx, i: usize) -> bool {
        match value(ctx, i, &self.field.k) {
            Val::Unseen => ctx.unseen,
            Val::None => false,
            Val::N(x) => match self.op {
                Op::AtLeast => x >= self.a - 1e-9,
                Op::AtMost => x <= self.a + 1e-9,
                Op::Between => x >= self.a.min(self.b) - 1e-9 && x <= self.a.max(self.b) + 1e-9,
                Op::Is => (x - self.a).abs() < 1e-6,
            },
            Val::T(s) => s.lines().any(|l| names_match(l, &self.want, self.exact)),
        }
    }
}

/// A number as the page shows it.
pub fn fmt_num(x: f64, dp: u8) -> String {
    if dp == 0 {
        let n = x.round() as i64;
        if n.abs() >= 10_000 { thousands(n) } else { n.to_string() }
    } else {
        format!("{x:.*}", dp as usize)
    }
}

/// 104318 -> "104,318" (minor bug 13).
pub fn thousands(n: i64) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// A search's filters: groups joined by "and", each a list joined by "or".
#[derive(Clone, Debug, Default)]
pub struct Query {
    pub terms: Vec<Term>,
}

impl Query {
    /// Reads the chips the screen sent; unknown fields are dropped.
    pub fn parse(v: &Value, catalog: &[Group]) -> Query {
        let mut terms = Vec::new();
        for c in v.as_array().into_iter().flatten() {
            let Some(k) = c.get("k").and_then(Value::as_str) else { continue };
            let Some(field) = field_of(catalog, k) else { continue };
            let op = Op::parse(c.get("op").and_then(Value::as_str).unwrap_or("is"));
            let num = |x: Option<&Value>| -> f64 {
                match x {
                    Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
                    Some(Value::Bool(b)) => f64::from(*b),
                    Some(Value::String(s)) => match field.kind {
                        FKind::Word => field.opts.iter().position(|o| o.eq_ignore_ascii_case(s)).unwrap_or(0) as f64,
                        FKind::Flag => f64::from(matches!(s.as_str(), "yes" | "true" | "1")),
                        _ => s.trim().parse().unwrap_or(0.0),
                    },
                    _ => 0.0,
                }
            };
            let text = match c.get("v") {
                Some(Value::String(s)) => s.clone(),
                Some(x) => x.to_string(),
                None => String::new(),
            };
            terms.push(Term {
                op: if field.kind == FKind::Text { Op::Is } else { op },
                a: num(c.get("v")),
                b: num(c.get("v2")),
                want: norm(&text),
                exact: false,
                text,
                or: c.get("or").and_then(Value::as_bool).unwrap_or(false),
                field,
            });
        }
        Query { terms }
    }

    pub fn json(&self) -> Value {
        json!(self.terms.iter().map(Term::json).collect::<Vec<_>>())
    }

    /// The search with each text filter told whether its name is a whole
    /// name: a program's or a conference's (as shown or real), or one a
    /// player in the pool carries.
    pub fn settled(&self, ctx: &Ctx, pool: &[usize]) -> Query {
        let w = ctx.w;
        let mut q = self.clone();
        for t in q.terms.iter_mut().filter(|t| t.field.kind == FKind::Text && !t.want.is_empty()) {
            let want = &t.want;
            let known = (0..w.programs.len()).any(|j| norm(w.program_name(j)) == *want)
                || w.programs
                    .iter()
                    .any(|p| norm(&p.conference) == *want || norm(&w.conference_name(&p.conference)) == *want);
            t.exact = known
                || pool
                    .iter()
                    .any(|&i| matches!(value(ctx, i, &t.field.k), Val::T(s) if s.lines().any(|l| norm(l) == *want)));
        }
        q
    }

    /// The fields the search reads (for the row's columns and the chart).
    pub fn keys(&self) -> Vec<String> {
        self.terms.iter().map(|t| t.field.k.clone()).collect()
    }

    /// Whether a player passes: every "and" group has a filter he passes.
    pub fn pass(&self, ctx: &Ctx, i: usize) -> bool {
        let mut group_ok = true;
        let mut any = false;
        for (k, t) in self.terms.iter().enumerate() {
            if k > 0 && !t.or {
                if !group_ok {
                    return false;
                }
                group_ok = true;
                any = false;
            }
            if !any {
                let ok = t.pass(ctx, i);
                if ok {
                    any = true;
                    group_ok = true;
                } else {
                    group_ok = false;
                }
            }
        }
        group_ok
    }

    /// Whether the search reads your staff's reads anywhere.
    pub fn reads_staff(&self) -> bool {
        self.terms.iter().any(|t| t.field.staff)
    }

    /// The filters as the page writes them: "(A or B) and C".
    pub fn words(&self) -> String {
        let mut groups: Vec<Vec<String>> = Vec::new();
        for (k, t) in self.terms.iter().enumerate() {
            if k == 0 || !t.or {
                groups.push(Vec::new());
            }
            if let Some(g) = groups.last_mut() {
                g.push(t.words());
            }
        }
        groups
            .into_iter()
            .map(|g| if g.len() > 1 { format!("({})", g.join(" or ")) } else { g.join("") })
            .collect::<Vec<_>>()
            .join(" and ")
    }
}

#[cfg(test)]
mod tests {
    use super::{names_match, norm, thousands};

    #[test]
    fn school_names_match_whole() {
        let m = |line: &str, typed: &str, exact: bool| names_match(line, &norm(typed), exact);
        // A whole name finds only itself (R5).
        for other in ["Michigan State", "Western Michigan", "Central Michigan", "Michigan Tech"] {
            assert!(!m(other, "Michigan", true), "{other}");
        }
        assert!(m("Michigan", "michigan", true));
        assert!(m("Michigan", "  MICHIGAN ", true));
        assert!(!m("Texas A&M", "Texas", true) && !m("North Texas", "Texas", true) && m("Texas", "Texas", true));
        assert!(!m("Miami (OH)", "Miami", true));
        // A name typed in part matches from word starts only.
        assert!(m("Ohio State", "ohio st", false));
        assert!(m("Andre Lyle", "Lyle", false));
        assert!(!m("Michigan", "chigan", false));
        assert!(!m("Ohio State", "hio", false));
        assert!(!m("Anything", "", false));
    }

    #[test]
    fn thousands_separator() {
        assert_eq!(thousands(104_318), "104,318");
        assert_eq!(thousands(1_284), "1,284");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(-12_000), "-12,000");
    }
}
