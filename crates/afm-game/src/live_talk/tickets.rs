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

/// A ticket is dropped this long after its call started (a chained second call
/// starts the count again), asked about or not. A call times out long before
/// this, so only a ticket nobody came back for is ever dropped (proposed, not confirmed).
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
    /// `Ai::generation` when the ticket started: an answer from an older key
    /// or client still reaches its next step, but does not mark the new key
    /// as working or refused.
    generation: u64,
    kind: Kind,
}

#[derive(Default)]
pub struct Tickets {
    next_id: u64,
    map: HashMap<u64, Ticket>,
    /// The ticket whose next step is running now (`polling`), so a step can
    /// leave something under its own number (the core's readings that wait
    /// for you to confirm).
    polling: Option<u64>,
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
    let generation = g.ai.generation();
    g.ai.tickets.map.insert(id, Ticket { born: Instant::now(), generation, kind: Kind::Call { slot, next } });
    Ok(id)
}

/// A ticket that answers with `answer` at the first poll: for a result the
/// game already has (a confirmed reading whose writer can't be called).
pub fn ready(g: &mut Game, answer: Value) -> u64 {
    start_demo(g, answer, 0)
}

/// The ticket whose next step is running now, inside `poll`.
pub fn polling(g: &Game) -> Option<u64> {
    g.ai.tickets.polling
}

/// A ticket that answers with `answer` after `waits` polls of "wait".
pub fn start_demo(g: &mut Game, answer: Value, waits: u32) -> u64 {
    g.ai.tickets.prune();
    let id = g.ai.tickets.take_id();
    let generation = g.ai.generation();
    g.ai.tickets.map.insert(id, Ticket { born: Instant::now(), generation, kind: Kind::Demo { answer, waits } });
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
            let kind = Kind::Demo { answer, waits: waits - 1 };
            g.ai.tickets.map.insert(id, Ticket { born: t.born, generation: t.generation, kind });
            wait()
        }
        Kind::Call { slot, next } => {
            let ready = slot.lock().ok().and_then(|mut s| s.take());
            let Some(result) = ready else {
                g.ai.tickets
                    .map
                    .insert(id, Ticket { born: t.born, generation: t.generation, kind: Kind::Call { slot, next } });
                return wait();
            };
            // Only an answer under the key and client in use now says anything
            // about them.
            if t.generation == g.ai.generation() {
                match &result {
                    Ok(_) => g.ai.note_success(),
                    Err(f) => g.ai.note_failure(f),
                }
            }
            g.ai.tickets.polling = Some(id);
            let step = next(g, result);
            g.ai.tickets.polling = None;
            match step {
                Step::Done(v) => v,
                Step::Call(req, next2) => {
                    let Some(client) = g.ai.client() else { return off(&Failure::NoKey.note()) };
                    match spawn(client, req) {
                        Ok(slot) => {
                            // The second call goes out on the client in use now.
                            let generation = g.ai.generation();
                            let kind = Kind::Call { slot, next: next2 };
                            g.ai.tickets.map.insert(id, Ticket { born: Instant::now(), generation, kind });
                            wait()
                        }
                        Err(f) => off(&f.note()),
                    }
                }
            }
        }
    }
}
