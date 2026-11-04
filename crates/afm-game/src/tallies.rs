//! The Data section's tallies in the game (lane NB1): each closed season's
//! counts in a folder beside the world file or save (`name.afmdata` next to
//! `name.afmworld` or `name.afmsave`), one file per kind and season
//! (`world::tally`). The season being played is in the save itself.
//!
//! While you play, closed seasons are written into `data/live.afmdata` in
//! the game folder. Opening a library world or a save links its folder
//! there (hard links, no copy) and the career's seasons go on after it.
//! Saving links the live folder beside the save. A season file is never
//! changed once written (a new one replaces it whole), so a save's links
//! keep what it had. The worlds page carries the folder as a tar file
//! (`name.afmdata.tar`, in parts past 2 GB), which the download and the
//! import unpack beside the world.

use std::io::Read;
use std::path::{Path, PathBuf};

use afm_engine::world::World;
use afm_engine::world::tally::EXT;

/// The folder beside a world file or a save.
pub fn beside(path: &Path) -> PathBuf {
    path.with_extension(EXT)
}

fn live(home: &Path) -> PathBuf {
    home.join("data").join(format!("live.{EXT}"))
}

/// Mirrors a folder by hard links (a copy where links can't be made),
/// through a folder beside it so `to` is whole or untouched.
pub fn mirror(from: &Path, to: &Path) -> std::io::Result<()> {
    let tmp = to.with_extension(format!("{EXT}.partial"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    copy_tree(from, &tmp)?;
    if to.exists() {
        std::fs::remove_dir_all(to)?;
    }
    std::fs::rename(&tmp, to)
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if e.file_type()?.is_dir() {
            copy_tree(&src, &dst)?;
        } else if src.extension().is_some_and(|x| x == "tally") && std::fs::hard_link(&src, &dst).is_err() {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

/// A world was opened (`from`: its world file or save; none for a world
/// built in the game): the live folder starts from its tallies, and any
/// seasons the world holds in memory are written there.
pub fn on_open(home: &Path, w: &mut World, from: Option<&Path>) {
    let live = live(home);
    w.detach_tallies();
    if live.exists()
        && let Err(e) = std::fs::remove_dir_all(&live)
    {
        eprintln!("tallies: could not clear {}: {e}", live.display());
        return;
    }
    if let Some(src) = from.map(beside).filter(|d| d.is_dir())
        && let Err(e) = mirror(&src, &live)
    {
        eprintln!("tallies: could not open {} ({e})", src.display());
    }
    if let Err(e) = w.attach_tallies(&live) {
        eprintln!("tallies: kept in memory this session ({e})");
    }
}

/// The career was saved to `to`: its tallies go beside it as they stand.
pub fn on_save(w: &mut World, to: &Path) {
    let Some(dir) = w.tally_dir().map(Path::to_path_buf) else {
        on_delete(to);
        return;
    };
    if let Err(e) = mirror(&dir, &beside(to)) {
        eprintln!("tallies: not saved beside {} ({e})", to.display());
    }
}

/// A save renamed: its tallies too.
pub fn on_rename(from: &Path, to: &Path) {
    let (a, b) = (beside(from), beside(to));
    if a.exists() && a != b {
        let _ = std::fs::rename(a, b);
    }
}

/// A save deleted: its tallies too (the live folder keeps its own links).
pub fn on_delete(path: &Path) {
    let d = beside(path);
    if d.exists() {
        let _ = std::fs::remove_dir_all(d);
    }
}

/// A world's tallies come into the library beside it: from the folder next
/// to `src` (a world file), or its tar file (`name.afmdata.tar`, or in parts
/// `.tar.001`, `.002`, ...). An older build's folder there goes.
pub fn import(src: &Path, to: &Path) {
    let (dir, tar) = (beside(src), PathBuf::from(format!("{}.tar", beside(src).display())));
    let dest = beside(to);
    if dir == dest {
        return;
    }
    let parts: Vec<PathBuf> = if tar.exists() {
        vec![tar.clone()]
    } else {
        (1..).map(|i| PathBuf::from(format!("{}.{i:03}", tar.display()))).take_while(|p| p.exists()).collect()
    };
    if !dir.is_dir() && parts.is_empty() {
        return;
    }
    if dest.exists() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    let r = if dir.is_dir() { mirror(&dir, &dest) } else { untar_parts(&parts, &dest) };
    if let Err(e) = r {
        eprintln!("tallies: not imported ({e})");
        let _ = std::fs::remove_dir_all(&dest);
    }
}

/// Unpacks the worlds page's tar file of a world's tallies (its parts one
/// after another) into `dest`: its one top folder becomes `dest`.
pub fn untar_parts(parts: &[PathBuf], dest: &Path) -> std::io::Result<()> {
    let mut chain: Box<dyn Read> = Box::new(std::io::empty());
    for p in parts {
        chain = Box::new(chain.chain(std::fs::File::open(p)?));
    }
    let tmp = dest.with_extension(format!("{EXT}.partial"));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(&tmp)?;
    untar(std::io::BufReader::new(chain), &tmp)?;
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    std::fs::rename(&tmp, dest)
}

/// Reads a plain tar (ustar or GNU, as `tar -cf` writes it): every file's
/// path under its first folder goes into `dest`. Only `.tally` files under
/// a kind's folder are kept, so nothing can land outside `dest`.
fn untar(mut r: impl Read, dest: &Path) -> std::io::Result<()> {
    let mut head = [0u8; 512];
    let mut long_name: Option<String> = None;
    loop {
        if read_full(&mut r, &mut head)? < 512 || head.iter().all(|&b| b == 0) {
            return Ok(());
        }
        let field = |a: usize, n: usize| {
            let s = &head[a..a + n];
            let end = s.iter().position(|&b| b == 0).unwrap_or(n);
            String::from_utf8_lossy(&s[..end]).to_string()
        };
        let size = u64::from_str_radix(field(124, 12).trim().trim_matches(char::from(0)), 8).unwrap_or(0);
        let kind = head[156];
        let mut name = field(0, 100);
        let prefix = field(345, 155);
        if !prefix.is_empty() && &head[257..262] == b"ustar" && head[263] == b'0' {
            name = format!("{prefix}/{name}");
        }
        if let Some(n) = long_name.take() {
            name = n;
        }
        let padded = size.div_ceil(512) * 512;
        let mut body = vec![0u8; padded as usize];
        read_full(&mut r, &mut body)?;
        body.truncate(size as usize);
        match kind {
            b'L' => long_name = Some(String::from_utf8_lossy(&body).trim_end_matches('\0').to_string()),
            b'0' | 0 => {
                // name: <world>.afmdata/<kind>/<season>.tally
                let parts: Vec<&str> = name.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
                if let [_, kind, file] = parts[..]
                    && afm_engine::world::tally::KINDS.contains(&kind)
                    && file.ends_with(".tally")
                    && !file.contains("..")
                {
                    let p = dest.join(kind).join(file);
                    std::fs::create_dir_all(p.parent().unwrap())?;
                    std::fs::write(p, &body)?;
                }
            }
            _ => {}
        }
    }
}

fn read_full(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        let k = r.read(&mut buf[n..])?;
        if k == 0 {
            break;
        }
        n += k;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tar as GNU tar writes it, read back into the folder.
    #[test]
    fn a_tar_of_the_tallies_unpacks_beside_the_world() {
        let root = std::env::temp_dir().join(format!("afm-tally-tar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let src = root.join("w.afmdata");
        std::fs::create_dir_all(src.join("players")).unwrap();
        std::fs::write(src.join("players").join("2025.tally"), b"hello").unwrap();
        let tar = root.join("w.afmdata.tar");
        let ok = std::process::Command::new("tar")
            .arg("-cf")
            .arg(&tar)
            .arg("-C")
            .arg(&root)
            .arg("w.afmdata")
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            return;
        }
        let dest = root.join("lib").join("w.afmdata");
        std::fs::create_dir_all(root.join("lib")).unwrap();
        untar_parts(&[tar], &dest).unwrap();
        assert_eq!(std::fs::read(dest.join("players").join("2025.tally")).unwrap(), b"hello");
        let _ = std::fs::remove_dir_all(&root);
    }
}
