//! Stamps the game with the time of the commit it was built from
//! (`AFM_CODE_TIME`, "YYYY-MM-DD HH:MM UTC", the same form a world file's
//! header writes), so a world that this build can't read can be told apart:
//! written after this code, it needs a newer game; written before, it is
//! damaged or this build broke an old file (`files::unreadable`). Left out
//! when git isn't there (a build from a copy of the code).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    // A new commit moves the branch's ref (or HEAD itself when detached).
    for f in [".git/HEAD", ".git/logs/HEAD"] {
        let p = root.join(f);
        if p.exists() {
            println!("cargo:rerun-if-changed={}", p.display());
        }
    }
    let time = std::process::Command::new("git")
        .args(["log", "-1", "--format=%cd", "--date=format-local:%Y-%m-%d %H:%M UTC"])
        .env("TZ", "UTC")
        .current_dir(&root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|t| t.len() == 20);
    if let Some(t) = time {
        println!("cargo:rustc-env=AFM_CODE_TIME={t}");
    }
}
