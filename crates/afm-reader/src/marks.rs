//! Marks against real: near, off or far off, the Debug Lab's thresholds
//! (`tools/dashboard/index.html`, `status`; proposed, not confirmed): the
//! gap as a share of the real figure (or of `floor` when the real figure is
//! near zero), within `near` is near, within `off` is off, beyond is far off.

#[derive(Clone, Copy, Debug)]
pub struct Tol {
    pub near: f64,
    pub off: f64,
    pub floor: f64,
}

pub const TOL: Tol = Tol { near: 0.08, off: 0.2, floor: 1e-9 };

pub fn gap(world: f64, real: f64, t: Tol) -> f64 {
    (world - real).abs() / real.abs().max(t.floor)
}

pub fn mark(world: f64, real: f64, t: Tol) -> &'static str {
    let d = gap(world, real, t);
    if d <= t.near {
        "near"
    } else if d <= t.off {
        "off"
    } else {
        "far off"
    }
}

/// The threshold in words, for the hover text.
pub fn words(t: Tol) -> String {
    let floor =
        if t.floor > 1e-6 { format!(" (gaps measured against at least {})", num(t.floor)) } else { String::new() };
    format!("near within {:.0}% of real, off within {:.0}%, far off beyond{floor}", t.near * 100.0, t.off * 100.0)
}

/// A number rounded to what matters: whole above 100, one place above 10,
/// two below, three below 1.
pub fn num(x: f64) -> String {
    if !x.is_finite() {
        return "-".into();
    }
    let a = x.abs();
    let s = if a >= 1000.0 {
        let whole = x.round() as i64;
        let t = whole.abs().to_string();
        let mut out = String::new();
        for (i, c) in t.chars().enumerate() {
            if i > 0 && (t.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        if whole < 0 { format!("-{out}") } else { out }
    } else if a >= 100.0 {
        format!("{:.0}", x)
    } else if a >= 10.0 {
        format!("{:.1}", x)
    } else if a >= 1.0 {
        format!("{:.2}", x)
    } else if a == 0.0 {
        "0".into()
    } else {
        format!("{:.3}", x)
    };
    if s.contains('.') && !s.contains(',') {
        let t = s.trim_end_matches('0').trim_end_matches('.');
        if t.is_empty() || t == "-" { "0".into() } else { t.to_string() }
    } else {
        s
    }
}

/// A number with its unit.
pub fn with_unit(x: f64, unit: &str) -> String {
    match unit {
        "" => num(x),
        "%" => format!("{}%", num(x)),
        u => format!("{} {u}", num(x)),
    }
}
