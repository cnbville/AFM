//! Tickets: how a call to the API runs beside the game.
//!
//! The game answers every command behind one lock, and the app and the
//! browser build handle one command at a time, so no API call may run inside
//! a command. A command that needs the API builds its request, `start`s a
//! ticket and returns the ticket's number at once. A worker thread (not the
//! game's) makes the call and leaves the result in the ticket's slot. The
//! screen then asks `ai_poll {ticket}`, and `poll` hands the result to the
//! ticket's next step under the lock, where it may change the world.
//!
//! A ticket can carry a chain: the next step may answer (`Step::Done`) or
//! start a second call on the same ticket (`Step::Call`). A typed line is
//! the reader, then the game's step, then the writer, and the core fills
//! those steps. Lane 1 only runs the chain.
//!
//! `Game.job` holds one job and refuses a second; tickets are many at once
//! (a talk box and a mail's own call), which is why this is its own map.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use afm_ai::{Client, Failure, Reply as AiReply, Request};
use serde_json::{Value, json};

use crate::Game;

/// A ticket nobody asks about for this long is dropped (proposed, not confirmed).
pub const TTL: Duration = Duration::from_secs(180);

/// Most tickets open at once. More are refused with a plain line (proposed, not confirmed).
pub const MAX_OPEN: usize = 32;

/// The step that runs, under the game lock, when a ticket's call is back.
pub type Next = Box<dyn FnOnce(&mut Game, Result<AiReply, Failure>) -> Step + Send>;

/// What a step does with a result.
pub enum Step {
    /// The poll's answer: an object with a `state` (see `live-talks.md`).
    Done(Value),
    /// Another call on the same ticket, then its own next step.
    Call(Request, Next),
}

impl Step {
    /// The answer for a call that failed: the box hides, the chips stay.
    pub fn off(f: &Failure) -> Step {
        Step::Done(off(&f.note()))
    }
}

type Slot = Arc<Mutex<Option<Result<AiReply, Failure>>>>;

enum Kind {
    Call {
        slot: Slot,
        next: Next,
    },
    /// A canned answer after a few waits (`ai_demo`, on the fake only).
    Demo {
        answer: Value,
        waits: u32,
    },
}

struct Ticket {
    born: Instant,
    kind: Kind,
}

#[derive(Default)]
pub struct Tickets {
    next_id: u64,
    map: HashMap<u64, Ticket>,
}

impl Tickets {
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Drops tickets nobody came back for.
    pub fn prune(&mut self) {
        self.prune_older_than(TTL);
    }

    pub fn prune_older_than(&mut self, age: Duration) {
        self.map.retain(|_, t| t.born.elapsed() < age);
    }

    fn take_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}

pub fn wait() -> Value {
    json!({ "state": "wait" })
}

pub fn off(note: &str) -> Value {
    json!({ "state": "off", "note": note })
}

/// Runs the client on its own thread and leaves the result in a slot. A
/// panic in the call becomes a failure, so a ticket never waits for ever.
fn spawn(client: Client, req: Request) -> Result<Slot, Failure> {
    let slot: Slot = Arc::new(Mutex::new(None));
    let put = slot.clone();
    std::thread::Builder::new()
        .name("afm-ai-call".into())
        .spawn(move || {
            let r = catch_unwind(AssertUnwindSafe(|| client.call(&req)))
                .unwrap_or_else(|_| Err(Failure::Api { status: 0, message: "the call stopped after an error".into() }));
            if let Ok(mut s) = put.lock() {
                *s = Some(r);
            }
        })
        .map_err(|e| Failure::Api { status: 0, message: format!("no thread for the call: {e}") })?;
    Ok(slot)
}

/// Starts a ticket: the call goes out on a worker thread and the ticket's
/// number comes back at once. Fails (before anything is sent) with no client,
/// at the cap, after a lasting failure (unless `force`) or with too many
/// tickets open.
pub fn start(g: &mut Game, req: Request, next: Next, force: bool) -> Result<u64, Failure> {
    g.ai.tickets.prune();
    if let Some(f) = g.ai.why_not(force) {
        return Err(f);
    }
    if g.ai.tickets.len() >= MAX_OPEN {
        return Err(Failure::Busy);
    }
    let client = g.ai.client().ok_or(Failure::NoKey)?;
    let slot = spawn(client, req)?;
    let id = g.ai.tickets.take_id();
    g.ai.tickets.map.insert(id, Ticket { born: Instant::now(), kind: Kind::Call { slot, next } });
    Ok(id)
}

/// A ticket that answers with `answer` after `waits` polls of "wait".
pub fn start_demo(g: &mut Game, answer: Value, waits: u32) -> u64 {
    g.ai.tickets.prune();
    let id = g.ai.tickets.take_id();
    g.ai.tickets.map.insert(id, Ticket { born: Instant::now(), kind: Kind::Demo { answer, waits } });
    id
}

/// The poll: "wait" until the call is back, then the next step's answer.
pub fn poll(g: &mut Game, id: u64) -> Value {
    g.ai.tickets.prune();
    let Some(t) = g.ai.tickets.map.remove(&id) else {
        return off("That answer is gone.");
    };
    match t.kind {
        Kind::Demo { answer, waits } => {
            if waits == 0 {
                return answer;
            }
            g.ai.tickets.map.insert(id, Ticket { born: t.born, kind: Kind::Demo { answer, waits: waits - 1 } });
            wait()
        }
        Kind::Call { slot, next } => {
            let ready = slot.lock().ok().and_then(|mut s| s.take());
            let Some(result) = ready else {
                g.ai.tickets.map.insert(id, Ticket { born: t.born, kind: Kind::Call { slot, next } });
                return wait();
            };
            match &result {
                Ok(_) => g.ai.note_success(),
                Err(f) => g.ai.note_failure(f),
            }
            match next(g, result) {
                Step::Done(v) => v,
                Step::Call(req, next2) => {
                    let Some(client) = g.ai.client() else { return off(&Failure::NoKey.note()) };
                    match spawn(client, req) {
                        Ok(slot) => {
                            g.ai.tickets
                                .map
                                .insert(id, Ticket { born: Instant::now(), kind: Kind::Call { slot, next: next2 } });
                            wait()
                        }
                        Err(f) => off(&f.note()),
                    }
                }
            }
        }
    }
}
