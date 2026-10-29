//! The world reader's book (world reader lane R4): one SQLite file per
//! world, built from the world file, its logs when there are any and the
//! real index, with every record as a table, the precomputed
//! distributions, every check against real, the consistency checks, the
//! odd things, the known issues, the knobs and the coverage map. See
//! `docs/world-reader.md`.
//!
//! - `make::build(world, log, out, say)` builds a book.
//! - `Book::open(path)`, then `book.text(page, &Filters)` (what `afm_read`
//!   prints and "Copy for Claude" gives) or `book.page(page, &Filters)`
//!   (the same page as JSON for the app).

pub mod book;
pub mod checks;
pub mod coverage;
pub mod defs;
pub mod defs_list;
pub mod dist;
pub mod issues;
pub mod json;
pub mod knobs;
pub mod log_in;
pub mod make;
pub mod marks;
pub mod odd;
pub mod pages;
pub mod real_in;
pub mod search;
pub mod sink;
pub mod stats;
pub mod text;
pub mod view;
pub mod world_in;

pub use book::Book;
pub use view::{Filters, View};
