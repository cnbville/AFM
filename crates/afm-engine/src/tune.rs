//! Temporary knobs for the program fortunes measuring pass
//! (`docs/design/program-fortunes.md`): `AFM_TUNE=name=value,name=value`
//! overrides a size for one run, so one compile serves every arm. Unset,
//! every knob is its default and a seed gives the same world.

/// A knob's value: `AFM_TUNE`'s when it names it, else `default`.
pub fn read(name: &str, default: f64) -> f64 {
    std::env::var("AFM_TUNE")
        .ok()
        .and_then(|s| {
            s.split(',').find_map(|kv| {
                let (k, v) = kv.split_once('=')?;
                (k.trim() == name).then(|| v.trim().parse::<f64>().ok()).flatten()
            })
        })
        .unwrap_or(default)
}

/// A knob read once: `knob!("name", default)`.
#[macro_export]
macro_rules! knob {
    ($name:literal, $default:expr) => {{
        static V: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
        *V.get_or_init(|| $crate::tune::read($name, $default))
    }};
}
