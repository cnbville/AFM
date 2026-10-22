//! Lane commands: faces (docs/final-build/plan.md, faces F3-F14, S4, X4).
//! `call` answers the commands this lane owns and returns None for any
//! other.
//!
//! The engine keeps one looks number per person (`world/looks.rs`); this
//! side turns it into what the screens show:
//! - `face`: one person's face: his genes and looks in words, his current
//!   team headshot, your photo if you gave him one, the drawn head when it
//!   is saved, and the ready AI prompt (F9);
//! - `faces`: the small round faces for a list (squad, depth chart,
//!   recruiting), with initials and team colours for the silhouette (S4);
//! - your photos (F7, F8): set, clear, and the "new team photo" switch;
//! - the folder import (X4): match file names to players, apply a batch,
//!   undo it; and "Export AI prompts", one text file per player;
//! - the drawn heads' switch (off by default) and their saved pictures.
//!
//! Everything lives in the save's own faces folder, `<save>.faces/` beside
//! `<save>.afmsave`, or `faces/unsaved/` in the game folder before the
//! first save. Pictures travel to the page as data URLs (the Mac app has
//! one `call` command and no file server).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use afm_engine::positions::Position;
use afm_engine::world::ethnicity::Ethnicity;
use afm_engine::world::looks::{Face, PhotoTeam};
use afm_engine::world::{Person, Stage, World, names};
use serde_json::{Value, json};

use crate::views::{TeamRef, team_colors, team_name};
use crate::{Game, Reply};

/// Bumped when the drawn heads change, so saved pictures are drawn again.
pub const RENDER_VERSION: u32 = 1;
/// Your photos: at most this many bytes each (a 4:5 crop, about 1 MB).
const MAX_PHOTO: usize = 6 << 20;

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "face" => face(game, args),
        "faces" => faces(game, args),
        "face_photo_set" => photo_set(game, args),
        "face_photo_clear" => photo_clear(game, args),
        "face_photo_mode" => photo_mode(game, args),
        "face_prompt" => prompt_cmd(game, args),
        "face_settings" => Ok(settings(game)),
        "face_settings_set" => settings_set(game, args),
        "face_drawn_store" => drawn_store(game, args),
        "faces_import_match" => import_match(game, args),
        "faces_import_apply" => import_apply(game, args),
        "faces_import_undo" => import_undo(game, args),
        "faces_import_batches" => Ok(import_batches(game)),
        "faces_export_prompts" => export_prompts(game, args),
        _ => return None,
    })
}

// --- The save's faces folder -------------------------------------------------------------------

/// The faces folder that goes with a save file.
pub fn folder_for(save: &Path) -> PathBuf {
    save.with_extension("faces")
}

fn unsaved(home: &Path) -> PathBuf {
    home.join("faces").join("unsaved")
}

fn dir(g: &Game) -> PathBuf {
    match &g.save_path {
        Some(p) => folder_for(p),
        None => unsaved(&g.home),
    }
}

/// A new world or a loaded save: the photos of a career never saved go.
pub fn on_open(g: &Game) {
    let _ = std::fs::remove_dir_all(unsaved(&g.home));
}

/// After a save to `to`: the faces go with it. A first save moves the
/// unsaved folder; a "save as" copies the old save's folder.
pub fn on_save(home: &Path, from: Option<&Path>, to: &Path) {
    let dest = folder_for(to);
    match from {
        None => {
            let src = unsaved(home);
            if src.exists() {
                let _ = std::fs::remove_dir_all(&dest);
                if std::fs::rename(&src, &dest).is_err() {
                    copy_dir(&src, &dest);
                    let _ = std::fs::remove_dir_all(&src);
                }
            }
        }
        Some(f) if f != to => {
            let src = folder_for(f);
            if src.exists() {
                let _ = std::fs::remove_dir_all(&dest);
                copy_dir(&src, &dest);
            }
        }
        _ => {}
    }
}

/// A save renamed: its faces folder too.
pub fn on_rename(from: &Path, to: &Path) {
    let src = folder_for(from);
    if src.exists() {
        let _ = std::fs::rename(src, folder_for(to));
    }
}

/// A save deleted: its faces folder too.
pub fn on_delete(save: &Path) {
    let _ = std::fs::remove_dir_all(folder_for(save));
}

fn copy_dir(src: &Path, dest: &Path) {
    let _ = std::fs::create_dir_all(dest);
    if let Ok(rd) = std::fs::read_dir(src) {
        for e in rd.flatten() {
            let to = dest.join(e.file_name());
            if e.path().is_dir() {
                copy_dir(&e.path(), &to);
            } else {
                let _ = std::fs::copy(e.path(), to);
            }
        }
    }
}

fn read_json(p: &Path) -> Value {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null)
}

fn write_json(p: &Path, v: &Value) -> Result<(), String> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("Could not make the faces folder: {e}"))?;
    }
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).unwrap_or_default())
        .and_then(|_| std::fs::rename(&tmp, p))
        .map_err(|e| format!("Could not write {}: {e}", p.display()))
}

// --- Data URLs ------------------------------------------------------------------------------------

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                out.push(B64[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let mut map = [255u8; 256];
    for (i, &c) in B64.iter().enumerate() {
        map[c as usize] = i as u8;
    }
    let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for c in bytes.chunks(4) {
        let mut n = 0u32;
        for (k, &b) in c.iter().enumerate() {
            let v = map[b as usize];
            if v == 255 {
                return None;
            }
            n |= (v as u32) << (18 - 6 * k);
        }
        let take = c.len().saturating_sub(1);
        for k in 0..take {
            out.push((n >> (16 - 8 * k)) as u8);
        }
    }
    Some(out)
}

/// A picture from the page: (bytes, file extension). Only PNG, JPEG and
/// WebP are kept (the page turns HEIC into JPEG where it can read it).
fn from_data_url(s: &str) -> Result<(Vec<u8>, &'static str), String> {
    let (head, body) = s.split_once(',').ok_or("That isn't a picture.")?;
    let ext = match head {
        h if h.starts_with("data:image/png") => "png",
        h if h.starts_with("data:image/jpeg") || h.starts_with("data:image/jpg") => "jpg",
        h if h.starts_with("data:image/webp") => "webp",
        _ => return Err("Only PNG, JPEG and WebP pictures can be kept.".into()),
    };
    let bytes = b64_decode(body).ok_or("The picture is damaged.")?;
    if bytes.len() > MAX_PHOTO {
        return Err("The picture is too big (6 MB at most after cropping).".into());
    }
    let ok = match ext {
        "png" => bytes.starts_with(b"\x89PNG"),
        "jpg" => bytes.starts_with(&[0xFF, 0xD8]),
        _ => bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
    };
    if !ok {
        return Err("The picture's contents don't match its type.".into());
    }
    Ok((bytes, ext))
}

fn to_data_url(p: &Path) -> Option<String> {
    let bytes = std::fs::read(p).ok()?;
    let mime = match p.extension()?.to_str()? {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return None,
    };
    Some(format!("data:{mime};base64,{}", b64_encode(&bytes)))
}

// --- Settings ------------------------------------------------------------------------------------------

fn settings_path(g: &Game) -> PathBuf {
    g.home.join("faces-settings.json")
}

/// Drawn heads are behind a switch, off until Cain okays the comparison
/// picture (final build B5).
fn drawn_on(g: &Game) -> bool {
    read_json(&settings_path(g)).get("drawn").and_then(Value::as_bool).unwrap_or(false)
}

fn settings(g: &Game) -> Value {
    json!({ "drawn": drawn_on(g), "render_version": RENDER_VERSION, "folder": dir(g).display().to_string() })
}

fn settings_set(g: &mut Game, args: &Value) -> Reply {
    let mut v = read_json(&settings_path(g));
    if !v.is_object() {
        v = json!({});
    }
    if let Some(d) = args.get("drawn").and_then(Value::as_bool) {
        v["drawn"] = json!(d);
    }
    write_json(&settings_path(g), &v)?;
    Ok(settings(g))
}

// --- Your photos -------------------------------------------------------------------------------------

/// The photos index: player id -> {file, mode, key, set, batch}. `mode` is
/// "photo" (yours shows) or "team" (you switched to the new team photo);
/// `key` is the team headshot that was current when you set it.
fn index(g: &Game) -> serde_json::Map<String, Value> {
    match read_json(&dir(g).join("photos.json")) {
        Value::Object(m) => m,
        _ => serde_json::Map::new(),
    }
}

fn save_index(g: &Game, m: serde_json::Map<String, Value>) -> Result<(), String> {
    write_json(&dir(g).join("photos.json"), &Value::Object(m))
}

fn find(w: &World, id: u32) -> Result<&Person, String> {
    w.people.iter().find(|p| p.player.id == id).ok_or_else(|| format!("No player {id}."))
}

fn id_arg(args: &Value) -> Result<u32, String> {
    args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".into())
}

fn store_photo(g: &Game, id: u32, data: &str, key: &str, batch: Option<u64>) -> Result<Option<Value>, String> {
    let (bytes, ext) = from_data_url(data)?;
    let d = dir(g).join("photos");
    std::fs::create_dir_all(&d).map_err(|e| format!("Could not make the photos folder: {e}"))?;
    let mut m = index(g);
    let prev = m.get(&id.to_string()).cloned();
    // A replaced photo is kept aside so a batch can be undone.
    if let Some(old) = prev.as_ref().and_then(|e| e.get("file")).and_then(Value::as_str) {
        let from = d.join(old);
        if from.exists() {
            let _ = std::fs::create_dir_all(d.join("replaced"));
            let _ = std::fs::rename(&from, d.join("replaced").join(old));
        }
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let file = format!("{id}-{stamp}.{ext}");
    std::fs::write(d.join(&file), bytes).map_err(|e| format!("Could not keep the photo: {e}"))?;
    let w = g.world.as_ref();
    m.insert(
        id.to_string(),
        json!({ "file": file, "mode": "photo", "key": key, "set": w.map(|w| crate::views::date_text(w.today)), "batch": batch }),
    );
    save_index(g, m)?;
    Ok(prev)
}

fn photo_set(g: &mut Game, args: &Value) -> Reply {
    let id = id_arg(args)?;
    let data = args.get("data").and_then(Value::as_str).ok_or("Missing picture.")?;
    let key = {
        let w = g.world.as_ref().ok_or("No world is loaded.")?;
        let p = find(w, id)?;
        w.photos(p).last().map(|ph| ph.key.clone()).unwrap_or_default()
    };
    store_photo(g, id, data, &key, None)?;
    face(g, args)
}

fn photo_clear(g: &mut Game, args: &Value) -> Reply {
    let id = id_arg(args)?;
    let mut m = index(g);
    if let Some(e) = m.remove(&id.to_string())
        && let Some(f) = e.get("file").and_then(Value::as_str)
    {
        let _ = std::fs::remove_file(dir(g).join("photos").join(f));
    }
    save_index(g, m)?;
    face(g, args)
}

/// F8: media day keeps your photo; this switches to the new team photo
/// ("team") or back to yours ("photo").
fn photo_mode(g: &mut Game, args: &Value) -> Reply {
    let id = id_arg(args)?;
    let mode = match args.get("mode").and_then(Value::as_str) {
        Some("team") => "team",
        _ => "photo",
    };
    let key = {
        let w = g.world.as_ref().ok_or("No world is loaded.")?;
        w.photos(find(w, id)?).last().map(|ph| ph.key.clone()).unwrap_or_default()
    };
    let mut m = index(g);
    let e = m.get_mut(&id.to_string()).ok_or("He has no photo of yours.")?;
    e["mode"] = json!(mode);
    if mode == "photo" {
        // Back to yours: the note stays quiet until the next new headshot.
        e["key"] = json!(key);
    }
    save_index(g, m)?;
    face(g, args)
}

// --- Words ----------------------------------------------------------------------------------------------

fn pos_words(p: Position) -> &'static str {
    match p {
        Position::QB => "quarterback",
        Position::RB => "running back",
        Position::WR => "wide receiver",
        Position::TE => "tight end",
        Position::OT => "offensive tackle",
        Position::IOL => "offensive guard",
        Position::EDGE => "defensive end",
        Position::DT => "defensive tackle",
        Position::LB => "linebacker",
        Position::CB => "cornerback",
        Position::S => "safety",
        Position::K => "kicker",
        Position::P => "punter",
        Position::LS => "long snapper",
    }
}

/// His heritage in words, with the family's origin where the surname
/// tells it.
fn heritage_words(p_name: &str, f: &Face) -> String {
    let g = &f.genes;
    let main = match g.heritage {
        Ethnicity::White => "White American".to_string(),
        Ethnicity::Black => "Black American".to_string(),
        Ethnicity::Hispanic => "Hispanic American".to_string(),
        Ethnicity::Polynesian => "Polynesian American (Samoan or Tongan heritage)".to_string(),
        e => match names::asian_origin_of(names::surname_of(p_name)) {
            Some(o) => format!("{o} American"),
            None => format!("{} American", e.name()),
        },
    };
    match g.mixed {
        Some((o, _)) => format!("{main}, of mixed {} and {} heritage", short(g.heritage), short(o)),
        None => main,
    }
}

fn short(e: Ethnicity) -> &'static str {
    match e {
        Ethnicity::White => "White",
        Ethnicity::Black => "Black",
        Ethnicity::Hispanic => "Hispanic",
        Ethnicity::Polynesian => "Polynesian",
        Ethnicity::EastAsian => "East Asian",
        Ethnicity::SoutheastAsian => "Southeast Asian",
        Ethnicity::SouthAsian => "South Asian",
    }
}

fn skin_words(d: f32) -> &'static str {
    match d {
        d if d < 0.12 => "fair skin",
        d if d < 0.28 => "light skin",
        d if d < 0.42 => "light tan skin",
        d if d < 0.56 => "medium brown skin",
        d if d < 0.72 => "brown skin",
        _ => "deep brown skin",
    }
}

/// The nearest plain colour name for a hex colour.
fn colour_name(hex: &str) -> &'static str {
    const NAMED: [(&str, [u8; 3]); 22] = [
        ("black", [20, 20, 20]),
        ("white", [245, 245, 245]),
        ("silver", [170, 170, 175]),
        ("grey", [110, 110, 115]),
        ("navy blue", [20, 35, 75]),
        ("royal blue", [30, 70, 170]),
        ("light blue", [110, 165, 220]),
        ("teal", [0, 120, 125]),
        ("dark green", [20, 75, 45]),
        ("kelly green", [30, 140, 60]),
        ("maroon", [110, 25, 40]),
        ("crimson", [160, 20, 40]),
        ("red", [210, 30, 40]),
        ("burnt orange", [190, 85, 20]),
        ("orange", [245, 120, 30]),
        ("gold", [200, 160, 60]),
        ("yellow", [250, 205, 20]),
        ("vegas gold", [180, 160, 110]),
        ("purple", [80, 40, 130]),
        ("brown", [90, 55, 30]),
        ("cardinal", [140, 20, 30]),
        ("powder blue", [150, 190, 230]),
    ];
    let h = hex.trim_start_matches('#');
    let c = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as f64;
    let (r, g, b) = (c(0), c(2), c(4));
    NAMED
        .iter()
        .min_by(|a, b2| {
            let d = |n: &[u8; 3]| {
                let (dr, dg, db) = (r - n[0] as f64, g - n[1] as f64, b - n[2] as f64);
                2.0 * dr * dr + 4.0 * dg * dg + 3.0 * db * db
            };
            d(&a.1).total_cmp(&d(&b2.1))
        })
        .map(|n| n.0)
        .unwrap_or("grey")
}

fn team_ref(t: PhotoTeam) -> Option<TeamRef> {
    match t {
        PhotoTeam::HighSchool(i) => Some(TeamRef::HighSchool(i)),
        PhotoTeam::College(i) => Some(TeamRef::College(i)),
        PhotoTeam::Nfl(i) => Some(TeamRef::Nfl(i)),
        PhotoTeam::None => None,
    }
}

fn current_team(w: &World, p: &Person) -> Option<TeamRef> {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => Some(TeamRef::HighSchool(p.hs)),
        Stage::College => p.college.map(TeamRef::College),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map(TeamRef::Nfl),
        _ => None,
    }
    .filter(|t| match t {
        TeamRef::HighSchool(i) => *i < w.high_schools.len(),
        _ => true,
    })
}

fn initials_of(name: &str) -> String {
    let first = name.split_whitespace().next().unwrap_or("");
    let last = names::surname_of(name);
    let a = first.chars().next().unwrap_or(' ');
    let b = last.chars().find(|c| c.is_uppercase()).or(last.chars().next()).unwrap_or(' ');
    format!("{a}{b}").trim().to_uppercase()
}

/// The ready image prompt (F9): heritage, age, height, weight, position,
/// team colours, hair, beard and his marks, as of his current headshot.
pub fn prompt(w: &World, p: &Person, f: &Face) -> String {
    let a = &f.appearance;
    let g = &f.genes;
    let team = f.photo.as_ref().and_then(|ph| team_ref(ph.team)).or_else(|| current_team(w, p));
    let level = match f.photo.as_ref().map(|ph| ph.team) {
        Some(PhotoTeam::HighSchool(_)) => "high school football player",
        Some(PhotoTeam::College(_)) => "college football player",
        Some(PhotoTeam::Nfl(_)) => "professional football player",
        _ => "football player",
    };
    let ht = afm_engine::body::height_text(a.height as f64);
    let pos = pos_words(p.player.position);
    let mut s = format!(
        "Official team headshot photo of a {}-year-old {} {level}, a {pos}, {ht} and {:.0} lb with a {} build.",
        a.age,
        heritage_words(&p.name, f),
        a.weight,
        a.build.word()
    );
    let fat = if a.face_fat >= 0.7 {
        " A full, heavy face with a thick neck."
    } else if a.face_fat >= 0.45 {
        " A full face and a thick neck."
    } else if a.face_fat <= 0.12 {
        " A lean, angular face."
    } else {
        ""
    };
    s.push_str(&format!(" {}, {}.{fat}", capital(skin_words(g.darkness)), g.eye.phrase()));
    s.push_str(&format!(" {}.", capital(&a.hair_words(g))));
    s.push_str(&format!(" {}.", capital(&a.beard_words(g))));
    let marks = a.marks_words(g);
    if !marks.is_empty() {
        s.push_str(&format!(" {}.", capital(&marks.join(", "))));
    }
    if let Some(t) = team {
        let [c1, c2] = team_colors(w, t);
        s.push_str(&format!(
            " Wearing a plain {} football jersey with {} trim over shoulder pads, no logos, no numbers, no text.",
            colour_name(&c1),
            colour_name(&c2)
        ));
    } else {
        s.push_str(" Wearing a plain grey training shirt.");
    }
    let expr = f.photo.as_ref().map(|ph| ph.expression.phrase()).unwrap_or("a slight smile");
    s.push_str(&format!(
        " Head and shoulders, facing the camera, {expr}. Plain dark grey studio backdrop, soft key light from the left with gentle shadow on the right cheek, sharp focus, natural skin texture, photorealistic, 4:5 portrait."
    ));
    s
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

// --- Faces for the screens ------------------------------------------------------------------------

fn drawn_path(g: &Game, id: u32, key: &str) -> PathBuf {
    dir(g).join("drawn").join(format!("{id}-{key}-v{RENDER_VERSION}.webp"))
}

/// What the drawn head needs: genes, looks, headshot and team colours.
fn spec(w: &World, f: &Face, colors: &[String; 2]) -> Value {
    json!({
        "genes": f.genes,
        "look": f.appearance,
        "expression": f.photo.as_ref().map(|ph| ph.expression),
        "colors": colors,
        "version": RENDER_VERSION,
        "season": w.photo_season(),
    })
}

/// One person's face, as the player page shows it.
fn face(g: &mut Game, args: &Value) -> Reply {
    let id = id_arg(args)?;
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let p = find(w, id)?;
    let f = w.face_of(p, None);
    let key = f.photo.as_ref().map(|ph| ph.key.clone()).unwrap_or_default();
    let team = f.photo.as_ref().and_then(|ph| team_ref(ph.team)).or_else(|| current_team(w, p));
    let colors = team.map(|t| team_colors(w, t)).unwrap_or(["#3a3f4b".into(), "#9aa3b2".into()]);
    let m = index(g);
    let mine = m.get(&id.to_string()).map(|e| {
        let file = e.get("file").and_then(Value::as_str).unwrap_or("");
        let set_key = e.get("key").and_then(Value::as_str).unwrap_or("");
        json!({
            "data": to_data_url(&dir(g).join("photos").join(file)),
            "mode": e.get("mode").cloned().unwrap_or(json!("photo")),
            "set": e.get("set").cloned().unwrap_or(Value::Null),
            // F8: a team headshot newer than your photo.
            "new_team_photo": !key.is_empty() && set_key != key,
        })
    });
    let drawn = if drawn_on(g) { to_data_url(&drawn_path(g, id, &key)) } else { None };
    let a = &f.appearance;
    let gn = &f.genes;
    let photos: Vec<Value> = f
        .photos
        .iter()
        .map(|ph| {
            json!({
                "key": ph.key, "year": ph.year, "joined": ph.joined,
                "team": team_ref(ph.team).map(|t| team_name(w, t)),
            })
        })
        .collect();
    Ok(json!({
        "id": id,
        "name": p.name,
        "initials": initials_of(&p.name),
        "colors": colors,
        "team": team.map(|t| team_name(w, t)),
        "heritage": gn.heritage.key(),
        "heritage_words": heritage_words(&p.name, &f),
        "photo": f.photo.as_ref().map(|ph| json!({
            "key": ph.key, "year": ph.year, "joined": ph.joined,
            "why": if ph.joined { "when he joined" } else if matches!(ph.team, PhotoTeam::HighSchool(_)) { "school picture day" } else { "media day" },
        })),
        "photos": photos,
        "mine": mine,
        "drawn": drawn,
        "drawn_on": drawn_on(g),
        "spec": spec(w, &f, &colors),
        "looks": {
            "hair": capital(&a.hair_words(gn)),
            "beard": capital(&a.beard_words(gn)),
            "marks": a.marks_words(gn),
            "build": a.build.word(),
            "skin": skin_words(gn.darkness),
            "eyes": gn.eye.phrase(),
        },
        "prompt": prompt(w, p, &f),
    }))
}

/// Faces for a list: initials and colours for the silhouette, your photo
/// or the saved drawn head when there is one, and what to draw otherwise.
fn faces(g: &mut Game, args: &Value) -> Reply {
    let ids: Vec<u32> = args
        .get("ids")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(Value::as_u64).map(|x| x as u32).take(400).collect())
        .unwrap_or_default();
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let want: std::collections::HashSet<u32> = ids.iter().copied().collect();
    let at: HashMap<u32, usize> =
        w.people.iter().enumerate().filter(|(_, p)| want.contains(&p.player.id)).map(|(i, p)| (p.player.id, i)).collect();
    let m = index(g);
    let drawn = drawn_on(g);
    let photos_dir = dir(g).join("photos");
    let out: Vec<Value> = ids
        .iter()
        .filter_map(|id| at.get(id).map(|&i| (id, &w.people[i])))
        .map(|(&id, p)| {
            let mine = m
                .get(&id.to_string())
                .filter(|e| e.get("mode").and_then(Value::as_str) != Some("team"))
                .and_then(|e| e.get("file").and_then(Value::as_str))
                .and_then(|f| to_data_url(&photos_dir.join(f)));
            let team = current_team(w, p);
            let colors = team.map(|t| team_colors(w, t)).unwrap_or(["#3a3f4b".into(), "#9aa3b2".into()]);
            let mut v = json!({ "id": id, "initials": initials_of(&p.name), "colors": colors, "photo": mine });
            if drawn && v["photo"].is_null() {
                let f = w.face_of(p, None);
                let key = f.photo.as_ref().map(|ph| ph.key.clone()).unwrap_or_default();
                v["key"] = json!(key);
                match to_data_url(&drawn_path(g, id, &key)) {
                    Some(d) => v["drawn"] = json!(d),
                    None => v["spec"] = spec(w, &f, &colors),
                }
            }
            v
        })
        .collect();
    Ok(json!({ "faces": out, "drawn_on": drawn }))
}

fn prompt_cmd(g: &mut Game, args: &Value) -> Reply {
    let id = id_arg(args)?;
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let p = find(w, id)?;
    let f = w.face_of(p, None);
    Ok(json!({ "prompt": prompt(w, p, &f) }))
}

/// A drawn head the page rendered, saved for next time (F14).
fn drawn_store(g: &mut Game, args: &Value) -> Reply {
    let id = id_arg(args)?;
    let key = args.get("key").and_then(Value::as_str).ok_or("Missing key.")?;
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("Bad key.".into());
    }
    let data = args.get("data").and_then(Value::as_str).ok_or("Missing picture.")?;
    let (bytes, _) = from_data_url(data)?;
    let p = drawn_path(g, id, key);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, bytes).map_err(|e| format!("Could not keep the drawn head: {e}"))?;
    Ok(json!({ "saved": true }))
}

// --- The folder import (X4) ----------------------------------------------------------------------------

fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '\'' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every team name a player has had (high school, colleges, NFL teams).
fn team_names(w: &World, p: &Person) -> Vec<String> {
    let mut v = vec![norm(w.school_name(p.hs))];
    for &c in &p.path {
        v.push(norm(w.program_name(c)));
    }
    if let Some(pro) = &p.pro {
        for &t in &pro.teams {
            v.push(norm(&w.nfl_team_name(t)));
        }
    }
    v
}

fn candidate(w: &World, p: &Person) -> Value {
    let team = current_team(w, p).map(|t| team_name(w, t));
    json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "team": team, "class_of": p.class_of })
}

/// Matches file names to players: by id ("41234.jpg"), by name and team
/// ("Jalen Smith - Alabama.png") or year ("Jalen Smith 2027.jpg", his class
/// or draft year), or by name alone when only one active player has it.
fn import_match(g: &mut Game, args: &Value) -> Reply {
    let files: Vec<String> = args
        .get("files")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(Value::as_str).map(String::from).collect())
        .unwrap_or_default();
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    let mut by_id: HashMap<u32, usize> = HashMap::new();
    for (i, p) in w.people.iter().enumerate() {
        by_id.insert(p.player.id, i);
        by_name.entry(norm(&p.name)).or_default().push(i);
        // Without a suffix too: "Kenneth Walton" finds "Kenneth Walton II".
        let first = p.name.split_whitespace().next().unwrap_or("");
        let plain = norm(&format!("{first} {}", names::surname_of(&p.name)));
        if plain != norm(&p.name) {
            by_name.entry(plain).or_default().push(i);
        }
    }
    let mine = crate::views::mine(w);
    let out: Vec<Value> = files
        .iter()
        .map(|file| {
            let stem = Path::new(file).file_stem().and_then(|s| s.to_str()).unwrap_or(file);
            let n = norm(stem);
            if let Ok(id) = n.parse::<u32>() {
                return match by_id.get(&id) {
                    Some(&i) => json!({ "file": file, "how": "id", "match": candidate(w, &w.people[i]) }),
                    None => json!({ "file": file, "how": "none", "why": format!("No player has id {id}.") }),
                };
            }
            let words: Vec<&str> = n.split(' ').collect();
            let year = words.iter().filter_map(|t| t.parse::<i32>().ok()).find(|y| (1950..=2150).contains(y));
            let words: Vec<&str> = words.into_iter().filter(|t| t.parse::<i32>().is_err()).collect();
            // The longest run of leading words that is someone's name.
            let mut found: Option<(&Vec<usize>, usize)> = None;
            for k in (2..=words.len().min(6)).rev() {
                if let Some(v) = by_name.get(&words[..k].join(" ")) {
                    found = Some((v, k));
                    break;
                }
            }
            let Some((cands, k)) = found else {
                return json!({ "file": file, "how": "none", "why": "No player with that name." });
            };
            let rest = words[k..].join(" ");
            let mut list: Vec<usize> = cands.clone();
            list.dedup();
            let mut how = "name";
            if !rest.is_empty() {
                let t: Vec<usize> =
                    list.iter().copied().filter(|&i| team_names(w, &w.people[i]).iter().any(|tn| tn.contains(&rest))).collect();
                if !t.is_empty() {
                    list = t;
                    how = "name_team";
                }
            }
            if let Some(y) = year {
                let t: Vec<usize> = list
                    .iter()
                    .copied()
                    .filter(|&i| {
                        let p = &w.people[i];
                        p.class_of == y || p.pro.as_ref().and_then(|c| c.draft).is_some_and(|d| d.0 == y)
                    })
                    .collect();
                if !t.is_empty() {
                    list = t;
                    how = if how == "name_team" { "name_team" } else { "name_year" };
                }
            }
            if list.len() > 1 {
                // Name alone: your own players first, then the active ones.
                let on_mine: Vec<usize> =
                    list.iter().copied().filter(|&i| current_team(w, &w.people[i]).is_some() && current_team(w, &w.people[i]) == mine).collect();
                if on_mine.len() == 1 {
                    list = on_mine;
                } else {
                    let active: Vec<usize> = list.iter().copied().filter(|&i| w.people[i].stage != Stage::Done).collect();
                    if active.len() == 1 {
                        list = active;
                    }
                }
            }
            if list.len() == 1 {
                json!({ "file": file, "how": how, "match": candidate(w, &w.people[list[0]]) })
            } else {
                let c: Vec<Value> = list.iter().take(6).map(|&i| candidate(w, &w.people[i])).collect();
                json!({ "file": file, "how": "ambiguous", "candidates": c, "why": format!("{} players have that name.", list.len()) })
            }
        })
        .collect();
    Ok(json!({ "files": out }))
}

fn imports_path(g: &Game) -> PathBuf {
    dir(g).join("imports.json")
}

/// Keeps a batch of photos: `{batch?, items: [{id, data}]}`. The first call
/// of a batch leaves `batch` out and gets its number back.
fn import_apply(g: &mut Game, args: &Value) -> Reply {
    let mut log = read_json(&imports_path(g));
    if !log.is_object() {
        log = json!({ "batches": [] });
    }
    let batch = match args.get("batch").and_then(Value::as_u64) {
        Some(b) => b,
        None => {
            let next = log["batches"].as_array().map(|b| b.len() as u64 + 1).unwrap_or(1);
            let date = g.world.as_ref().map(|w| crate::views::date_text(w.today));
            log["batches"].as_array_mut().ok_or("Bad import log.")?.push(json!({ "batch": next, "date": date, "items": [] }));
            next
        }
    };
    let items = args.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut done = Vec::new();
    let mut errors = Vec::new();
    for it in items {
        let Some(id) = it.get("id").and_then(Value::as_u64).map(|v| v as u32) else { continue };
        let Some(data) = it.get("data").and_then(Value::as_str) else { continue };
        let key = {
            let w = g.world.as_ref().ok_or("No world is loaded.")?;
            match find(w, id) {
                Ok(p) => w.photos(p).last().map(|ph| ph.key.clone()).unwrap_or_default(),
                Err(e) => {
                    errors.push(json!({ "id": id, "error": e }));
                    continue;
                }
            }
        };
        match store_photo(g, id, data, &key, Some(batch)) {
            Ok(prev) => done.push(json!({ "id": id, "prev": prev })),
            Err(e) => errors.push(json!({ "id": id, "error": e })),
        }
    }
    if let Some(b) = log["batches"].as_array_mut().and_then(|v| v.iter_mut().find(|b| b["batch"] == json!(batch)))
        && let Some(list) = b["items"].as_array_mut()
    {
        list.extend(done.iter().cloned());
    }
    write_json(&imports_path(g), &log)?;
    Ok(json!({ "batch": batch, "kept": done.len(), "errors": errors }))
}

/// Undoes a whole batch: each photo it set goes, and a photo it replaced
/// comes back.
fn import_undo(g: &mut Game, args: &Value) -> Reply {
    let batch = args.get("batch").and_then(Value::as_u64).ok_or("Missing batch.")?;
    let mut log = read_json(&imports_path(g));
    let list = log["batches"].as_array_mut().ok_or("No imports to undo.")?;
    let k = list.iter().position(|b| b["batch"] == json!(batch)).ok_or("No such import.")?;
    let b = list.remove(k);
    let photos = dir(g).join("photos");
    let mut m = index(g);
    let mut undone = 0;
    for it in b["items"].as_array().cloned().unwrap_or_default().iter().rev() {
        let Some(id) = it["id"].as_u64() else { continue };
        let key = id.to_string();
        // Only if the photo is still the one this batch set.
        if m.get(&key).and_then(|e| e["batch"].as_u64()) != Some(batch) {
            continue;
        }
        if let Some(f) = m.get(&key).and_then(|e| e["file"].as_str()) {
            let _ = std::fs::remove_file(photos.join(f));
        }
        match it.get("prev").filter(|p| !p.is_null()) {
            Some(prev) => {
                if let Some(f) = prev["file"].as_str() {
                    let _ = std::fs::rename(photos.join("replaced").join(f), photos.join(f));
                }
                m.insert(key, prev.clone());
            }
            None => {
                m.remove(&key);
            }
        }
        undone += 1;
    }
    save_index(g, m)?;
    write_json(&imports_path(g), &log)?;
    Ok(json!({ "undone": undone, "batches": import_batches(g)["batches"] }))
}

fn import_batches(g: &Game) -> Value {
    let log = read_json(&imports_path(g));
    let b: Vec<Value> = log["batches"]
        .as_array()
        .map(|v| {
            v.iter()
                .rev()
                .map(|b| json!({ "batch": b["batch"], "date": b["date"], "count": b["items"].as_array().map_or(0, |i| i.len()) }))
                .collect()
        })
        .unwrap_or_default();
    json!({ "batches": b })
}

/// "Export AI prompts": one text file per player, in a new folder in the
/// game folder: `prompts/<label> <date>/<Name> (<id>).txt`.
fn export_prompts(g: &mut Game, args: &Value) -> Reply {
    let ids: Vec<u32> = args
        .get("ids")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(Value::as_u64).map(|x| x as u32).collect())
        .unwrap_or_default();
    let label = crate::setup::file_safe(args.get("label").and_then(Value::as_str).unwrap_or("Players"));
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let want: std::collections::HashSet<u32> = ids.iter().copied().collect();
    let date = crate::views::date_text(w.today);
    let folder = g.home.join("prompts").join(crate::setup::file_safe(&format!("{label} {date}")));
    std::fs::create_dir_all(&folder).map_err(|e| format!("Could not make {}: {e}", folder.display()))?;
    let mut n = 0;
    for p in w.people.iter().filter(|p| want.contains(&p.player.id)) {
        let f = w.face_of(p, None);
        let file = folder.join(format!("{} ({}).txt", crate::setup::file_safe(&p.name), p.player.id));
        std::fs::write(&file, prompt(w, p, &f) + "\n").map_err(|e| format!("Could not write {}: {e}", file.display()))?;
        n += 1;
    }
    Ok(json!({ "folder": folder.display().to_string(), "count": n }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for n in 0..40usize {
            let data: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(b64_decode(&b64_encode(&data)).unwrap(), data);
        }
        assert_eq!(b64_encode(b"Man"), "TWFu");
        assert_eq!(b64_encode(b"Ma"), "TWE=");
    }

    #[test]
    fn colour_names_are_plain() {
        assert_eq!(colour_name("#154734"), "dark green");
        assert_eq!(colour_name("#9E1B32"), "crimson");
        assert_eq!(colour_name("#FFFFFF"), "white");
        assert_eq!(initials_of("Mark De La Cruz Jr."), "MD");
        assert_eq!(norm("Jalen_Smith - Alabama (2027)"), "jalen smith alabama 2027");
    }
}
