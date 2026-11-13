//! `Game.ai`: the key (in memory only), the client built from it, the
//! month's spend book and the tickets in flight.
//!
//! The key is never written anywhere by the game: not the world, not a
//! save, not the game folder, not a log. The desktop app keeps its own copy
//! in the macOS Keychain (`app/src-tauri`) and hands it over at start. The
//! browser build reads `AFM_AI_KEY` (`bin/afm_play.rs`). `Debug` shows only
//! the last four characters.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use afm_ai::spend::usd;
use afm_ai::{CAP_USD, Client, Failure, Fake, Key, Real, Spend, month_now};
use serde_json::{Value, json};

use super::tickets::Tickets;

pub struct Ai {
    key: Option<Key>,
    fake: Option<Arc<Fake>>,
    client: Option<Client>,
    spend: Arc<Mutex<Spend>>,
    home: PathBuf,
    /// The last call that failed: its kind and the plain line.
    last: Option<(&'static str, String)>,
    /// A failure that will repeat until something changes (a key the API
    /// refused, credit gone): the boxes stay hidden until a new key or a
    /// good test.
    blocked: Option<Failure>,
    /// Where the key is kept besides memory: "keychain" when the app has
    /// saved it in the macOS Keychain, else "memory". Set by the app.
    kept: &'static str,
    /// Counts the changes of key or client. A ticket remembers the number it
    /// started under, so a late answer from an old key does not mark the new one.
    generation: u64,
    /// The fake is the browser build's (`AFM_AI_FAKE=1`): the core's
    /// stand-in reader answers there, so a typed line lands where its words
    /// point (`say::stand_in`).
    browser: bool,
    pub tickets: Tickets,
    /// Typed lines whose reading waits for you to confirm (the core).
    pub waiting: super::say::Waiting,
}

impl fmt::Debug for Ai {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ai {{ key: ")?;
        match &self.key {
            Some(k) => write!(f, "...{}", k.tail())?,
            None => write!(f, "none")?,
        }
        write!(f, ", fake: {}, tickets: {} }}", self.fake.is_some(), self.tickets.len())
    }
}

impl Ai {
    pub fn new(home: &Path) -> Ai {
        Ai {
            key: None,
            fake: None,
            client: None,
            spend: Arc::new(Mutex::new(Spend::open(home.join("ai-spend.json")))),
            home: home.to_path_buf(),
            last: None,
            blocked: None,
            kept: "memory",
            generation: 0,
            browser: false,
            tickets: Tickets::default(),
            waiting: Default::default(),
        }
    }

    /// Keeps a key in memory (after checking its shape) and builds the real
    /// client from it. With the fake in use the key is kept and the fake
    /// stays the client. The new key is in memory only until the app says it
    /// has saved it in the Keychain (`set_kept`).
    pub fn set_key(&mut self, raw: &str) -> Result<(), String> {
        self.key = Some(Key::parse(raw)?);
        self.kept = "memory";
        self.blocked = None;
        self.last = None;
        self.rebuild();
        Ok(())
    }

    pub fn clear_key(&mut self) {
        self.key = None;
        self.kept = "memory";
        self.blocked = None;
        self.last = None;
        self.rebuild();
    }

    /// The number of the current key and client (see the field).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Plays every call on this fake instead of the network. Its spend goes
    /// to a book of its own, so a test run never counts against the real
    /// month. With the fake in use the boxes are on without a key.
    pub fn use_fake(&mut self, fake: Arc<Fake>) {
        self.fake = Some(fake);
        self.browser = false;
        self.spend = Arc::new(Mutex::new(Spend::open(self.home.join("ai-spend-fake.json"))));
        self.rebuild();
    }

    /// The fake with a little latency, for the browser build.
    pub fn use_browser_fake(&mut self) -> Arc<Fake> {
        let f = Arc::new(Fake::for_browser());
        self.use_fake(f.clone());
        self.browser = true;
        f
    }

    /// The browser build's fake, when it is the client in use.
    pub fn browser_fake(&self) -> Option<Arc<Fake>> {
        self.fake.clone().filter(|_| self.browser)
    }

    fn rebuild(&mut self) {
        self.generation += 1;
        self.client = match (&self.fake, &self.key) {
            (Some(f), _) => Some(Client::new(f.clone(), self.spend.clone())),
            (None, Some(k)) => Some(Client::new(Arc::new(Real::new(k.clone())), self.spend.clone())),
            (None, None) => None,
        };
    }

    /// The app says where it has put the key besides memory ("keychain").
    pub fn set_kept(&mut self, kept: &'static str) {
        self.kept = kept;
    }

    pub fn is_fake(&self) -> bool {
        self.fake.is_some()
    }

    pub fn has_client(&self) -> bool {
        self.client.is_some()
    }

    /// True only when the client that would be used is the real one.
    pub fn has_real_client(&self) -> bool {
        self.client.as_ref().is_some_and(|c| !c.is_fake())
    }

    pub fn key_tail(&self) -> Option<String> {
        self.key.as_ref().map(Key::tail)
    }

    pub fn client(&self) -> Option<Client> {
        self.client.clone()
    }

    fn cap_reached(&self) -> bool {
        self.spend.lock().is_ok_and(|s| s.cap_reached(&month_now()))
    }

    /// Why a call would not be made now, or none if it would. `force` skips a
    /// lasting failure: the settings card's Test is how that one is cleared.
    pub fn why_not(&self, force: bool) -> Option<Failure> {
        if self.client.is_none() {
            return Some(Failure::NoKey);
        }
        if self.cap_reached() {
            return Some(Failure::CapReached);
        }
        if !force {
            return self.blocked.clone();
        }
        None
    }

    /// True when a box may be shown: a client is there, the cap is not
    /// reached and the last lasting failure has not been left standing.
    /// (The core's `live_talk::after` adds a box to a reply only when this
    /// is true.)
    pub fn usable(&self) -> bool {
        self.why_not(false).is_none()
    }

    pub fn note_failure(&mut self, f: &Failure) {
        self.last = Some((f.kind(), f.note()));
        if f.is_lasting() {
            self.blocked = Some(f.clone());
        }
    }

    pub fn note_success(&mut self) {
        self.last = None;
        self.blocked = None;
    }

    /// What the settings card shows: the key's tail (never more), this
    /// month's spend against the cap, and one plain note if something is wrong.
    pub fn status(&self) -> Value {
        let month = month_now();
        let (book, cap_nano, file_note) = match self.spend.lock() {
            Ok(s) => (s.book(&month), s.cap(), s.note.clone()),
            Err(_) => (Default::default(), 0, None),
        };
        let cap_reached = book.nano >= cap_nano;
        let note = self
            .blocked
            .as_ref()
            .map(Failure::note)
            .or_else(|| cap_reached.then(|| Failure::CapReached.note()))
            .or_else(|| self.last.as_ref().map(|(_, n)| n.clone()))
            .or(file_note);
        // Six decimals: a talk costs a fraction of a cent.
        let round = |x: f64| (x * 1e6).round() / 1e6;
        json!({
            "key_set": self.key.is_some(),
            "key_ending": self.key.as_ref().map(|k| format!("...{}", k.tail())),
            "kept": self.key.as_ref().map(|_| self.kept),
            "fake": self.fake.is_some(),
            "usable": self.usable(),
            "month": month,
            "spent": round(usd(book.nano)),
            "cap": CAP_USD,
            "cap_reached": cap_reached,
            "calls": book.calls,
            "note": note,
        })
    }
}
