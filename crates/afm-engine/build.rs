//! Stamps the engine with the time of the commit it was built from
//! (`AFM_CODE_TIME`, "YYYY-MM-DD HH:MM UTC", the same form a world file's
//! header writes; `save::CODE_TIME`). Every world file carries its writer's
//! stamp, so a game that can't read a world can tell a newer game's world
//! (update the game) from a damaged one (`afm-game`, `files::unreadable`).
//! Left out when git isn't there (a build from a copy of the code). A clean
//! build (every Mac and world build) stamps fresh; a local build restamps
//! when the checkout moves (`.git/HEAD`), not on every commit, so the
//! engine isn't rebuilt for a commit alone.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let head = root.join(".git/HEAD");
    if head.exists() {
        println!("cargo:rerun-if-changed={}", head.display());
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
