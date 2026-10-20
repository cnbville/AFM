//! Plays AFM in a browser: serves the app's screens (`app/ui`) and the
//! game's commands on localhost, the same commands the desktop app calls.
//! For testing and for playing where the desktop app isn't built.
//! Usage: afm_play [port] [ui folder] [game folder]

use afm_game::Game;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(7700);
    let ui = std::path::PathBuf::from(args.get(2).cloned().unwrap_or_else(|| "app/ui".into()));
    let home = args.get(3).map(std::path::PathBuf::from).unwrap_or_else(afm_game::default_home);
    let mut game = Game::new(home);
    let server = tiny_http::Server::http(("127.0.0.1", port)).expect("port in use");
    println!("AFM on http://127.0.0.1:{port}");
    for mut req in server.incoming_requests() {
        let url = req.url().to_string();
        if let Some(cmd) = url.strip_prefix("/api/") {
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            let args: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
            let t = std::time::Instant::now();
            let reply = match game.call(cmd, &args) {
                Ok(v) => serde_json::json!({ "ok": v }),
                Err(e) => serde_json::json!({ "err": e }),
            };
            eprintln!("{cmd} {:.0} ms", t.elapsed().as_secs_f64() * 1000.0);
            let resp = tiny_http::Response::from_string(reply.to_string())
                .with_header("Content-Type: application/json".parse::<tiny_http::Header>().unwrap());
            let _ = req.respond(resp);
            continue;
        }
        let rel = url.split('?').next().unwrap_or("/").trim_start_matches('/');
        let rel = if rel.is_empty() { "index.html" } else { rel };
        let path = ui.join(rel);
        let ok = path.canonicalize().ok().zip(ui.canonicalize().ok()).is_some_and(|(p, u)| p.starts_with(u));
        match std::fs::read(&path) {
            Ok(bytes) if ok => {
                let ct = match path.extension().and_then(|e| e.to_str()) {
                    Some("html") => "text/html; charset=utf-8",
                    Some("css") => "text/css",
                    Some("js") => "text/javascript",
                    Some("svg") => "image/svg+xml",
                    Some("woff2") => "font/woff2",
                    Some("ttf") => "font/ttf",
                    Some("png") => "image/png",
                    _ => "application/octet-stream",
                };
                let resp = tiny_http::Response::from_data(bytes)
                    .with_header(format!("Content-Type: {ct}").parse::<tiny_http::Header>().unwrap());
                let _ = req.respond(resp);
            }
            _ => {
                let _ = req.respond(tiny_http::Response::from_string("not found").with_status_code(404));
            }
        }
    }
}
