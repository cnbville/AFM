//! `afm_read`: builds a world's book and prints any page of it.
//!
//! - `afm_read build <world file> [log folder] [book]`: the log defaults to
//!   `name.afmlog` beside the world file (`-` for none), the book to
//!   `name.afmbook`.
//! - `afm_read <book> [page] [filters]`: a page as short text, the same
//!   text the app's "Copy for Claude" gives. `afm_read <book> help` lists
//!   the pages; filters are `key=value` (`year=2031 level=FBS pos=QB
//!   limit=50 debug`).

use std::path::{Path, PathBuf};

use afm_reader::{Book, Filters, make};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        eprintln!(
            "afm_read build <world file> [log folder] [book]\nafm_read <book> [page] [key=value ...]\nafm_read <book> help"
        );
        std::process::exit(2);
    }
    if args[0] == "build" {
        let Some(world) = args.get(1).map(PathBuf::from) else {
            eprintln!("afm_read build <world file> [log folder] [book]");
            std::process::exit(2);
        };
        // `-` builds from the world file alone, even with a log beside it.
        let log = match args.get(2).map(String::as_str) {
            Some("-") => None,
            Some(l) => Some(PathBuf::from(l)),
            None => make::log_beside(&world),
        };
        let out = args.get(3).map(PathBuf::from).unwrap_or_else(|| world.with_extension("afmbook"));
        match make::build(&world, log.as_deref(), &out, &mut |s| eprintln!("{s}")) {
            Ok(s) => {
                println!(
                    "book {}: {} tables, {} rows, {:.2} GB, {} checks, {:.0}s, peak memory {:.0} MB",
                    out.display(),
                    s.tables,
                    s.rows,
                    s.bytes as f64 / 1e9,
                    s.checks,
                    s.seconds,
                    s.peak_mb
                );
                for m in s.missing {
                    println!("not in this book: {m}");
                }
            }
            Err(e) => {
                eprintln!("could not build the book: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let book = match Book::open(Path::new(&args[0])) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("could not open {}: {e}", args[0]);
            std::process::exit(1);
        }
    };
    // The page is every word before the first key=value; a query is all
    // of it (`sql ... WHERE a = 'b'`).
    let rest = &args[1..];
    let whole = rest.first().is_some_and(|a| a == "sql");
    let split = if whole {
        rest.len()
    } else {
        rest.iter().position(|a| a.contains('=') || a == "debug").unwrap_or(rest.len())
    };
    let (page, filters) = rest.split_at(split);
    let page = if page.is_empty() { "overview".to_string() } else { page.join(" ") };
    let page = if page == "help" { "?".to_string() } else { page };
    print!("{}", book.text(&page, &Filters::parse(filters)));
}
