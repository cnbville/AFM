//! The one builder for every card (live talks, the core): what the reader
//! and the writer are given, and nothing else. A card is a whitelist type:
//! words only, with no field for potential, true ratings, true wants,
//! hidden trait numbers or debug data, so nothing of the game's insides can
//! reach a call by mistake. Everything on it is something your seat sees:
//! the meeting's own lines, the chips' words, names, roles and class years,
//! and what your staff's read caught, in words. A person's own wants reach
//! a card only as the line the game picked for him. Once the fact sheet
//! fills a talk's facts (`folk::claims`), they come from it alone.
//!
//! The ids the reader sees (o1, p1) map back to the game's codes and
//! people through `Keys`, which never leaves the game.

use afm_engine::world::World;
use afm_engine::world::folk::scene::{self, Scene, Who};
use afm_engine::world::folk::{self as folk, Team, cast, facts};

/// The most options a reader's card lists (o1 to o12): the schema's list is
/// fixed, so its cache holds for every scene. Proposed, not confirmed.
pub const MAX_OPTIONS: usize = 12;

/// The most lines of the talk so far a card carries. Proposed, not
/// confirmed.
pub const SO_FAR: usize = 6;

/// A short tag the reader answers with, and its words.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tagged {
    pub tag: String,
    pub text: String,
}

/// One line of the talk so far.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spoken {
    pub who: String,
    pub text: String,
}

/// A card: every field is words a seat may see.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Card {
    /// What meeting, where, when, at what level.
    pub setting: String,
    /// You.
    pub coach: String,
    /// Who is across from you: name, role, class year.
    pub them: String,
    /// The people the line may name, tagged p1, p2...
    pub may_name: Vec<Tagged>,
    /// The options this exchange can reach, tagged o1, o2... (the reader's).
    pub options: Vec<Tagged>,
    pub so_far: Vec<Spoken>,
    /// The line the game picked for him for this answer (the writer's).
    pub his_line: String,
    /// How he comes across as he says it, as your read caught it.
    pub tell: String,
    /// What the game released with this answer, in words (the writer's).
    pub released: Vec<String>,
    /// Facts the talk may use, from the fact sheet.
    pub facts: Vec<String>,
    /// Lines from players or staff like him, for his register.
    pub examples: Vec<String>,
}

/// The reader's ids and what they stand for. Never sent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keys {
    pub options: Vec<(String, u32)>,
    pub people: Vec<(String, u32)>,
}

impl Keys {
    pub fn option(&self, tag: &str) -> Option<u32> {
        self.options.iter().find(|(t, _)| t == tag).map(|x| x.1)
    }
    pub fn option_tag(&self, code: u32) -> Option<&str> {
        self.options.iter().find(|(_, c)| *c == code).map(|x| x.0.as_str())
    }
    pub fn person(&self, tag: &str) -> Option<u32> {
        self.people.iter().find(|(t, _)| t == tag).map(|x| x.1)
    }
}

/// Typed words as they go on a card: the tag marks taken out, so a line
/// can't close its own tag.
pub fn quoted(words: &str) -> String {
    words.replace(['<', '>'], "").trim().to_string()
}

/// A person in plain words: name, then role, position and class year.
pub fn person(w: &World, who: &Who) -> String {
    match who {
        Who::Player(id) => match folk::person(w, *id) {
            Some(i) => {
                let p = &w.people[i];
                format!("{}, {}, {}", p.name, p.player.position.name(), facts::class_words(w, i))
            }
            None => "a former player".into(),
        },
        Who::Coach(c) => match w.coaches.get(*c) {
            Some(co) => {
                let at = co.job.map(|j| format!(" at {}", Team::from_employer(j.employer).name(w))).unwrap_or_default();
                format!("{}, {}{at}", co.name, facts::coach_role(w, *c))
            }
            None => "a coach".into(),
        },
        Who::Cast(tok) => match cast::from_token(w, tok) {
            Some(c) => format!("{}, {}", folk::plain(&c.name), c.title),
            None => "someone".into(),
        },
        Who::Group(g) => folk::plain(g),
    }
}

/// The meeting in one line: what, where, when, at what level.
fn setting(w: &World, s: &Scene) -> String {
    let mut out = s.kind.name().to_string();
    if !s.place.is_empty() {
        out.push_str(", ");
        out.push_str(&folk::plain(&s.place));
    }
    out.push_str(", ");
    out.push_str(&crate::views::date_text(s.date));
    if let Some(t) = Team::seat(w) {
        out.push_str(", ");
        out.push_str(&format!("{:?}", t.level()).to_lowercase());
    }
    out
}

fn coach(w: &World) -> String {
    let name = scene::your_name(w);
    match Team::seat(w) {
        Some(t) => format!("{name}, head coach at {}", t.name(w)),
        None => name,
    }
}

/// The talk so far: the last lines, typed words where you typed them.
fn so_far(s: &Scene, upto: usize) -> Vec<Spoken> {
    let lines = &s.lines[..upto.min(s.lines.len())];
    let from = lines.len().saturating_sub(SO_FAR);
    lines[from..]
        .iter()
        .map(|l| Spoken {
            who: match l.by {
                0 => "Scene".to_string(),
                1 => "Coach".to_string(),
                _ => folk::plain(&l.name),
            },
            text: folk::plain(l.typed.as_deref().unwrap_or(&l.text)),
        })
        .collect()
}

/// The reader's card for a scene: its options and who may be named, with
/// the keys back to the game's codes and ids.
pub fn scene(w: &World, s: &Scene) -> (Card, Keys) {
    let mut keys = Keys::default();
    let mut card = Card {
        setting: setting(w, s),
        coach: coach(w),
        them: s.who.first().map(|x| person(w, x)).unwrap_or_default(),
        so_far: so_far(s, s.lines.len()),
        ..Card::default()
    };
    for (k, p) in scene::options(s).into_iter().take(MAX_OPTIONS).enumerate() {
        let tag = format!("o{}", k + 1);
        let mv = if p.mv.is_empty() { String::new() } else { format!(" ({})", p.mv) };
        card.options.push(Tagged { tag: tag.clone(), text: format!("{}{mv}", folk::plain(&p.label)) });
        keys.options.push((tag, p.code));
    }
    for who in &s.who {
        if let Who::Player(id) = who
            && folk::person(w, *id).is_some()
        {
            let tag = format!("p{}", keys.people.len() + 1);
            card.may_name.push(Tagged { tag: tag.clone(), text: person(w, who) });
            keys.people.push((tag, *id));
        }
    }
    (card, keys)
}

/// The writer's card for the line at `line` (his answer, which the game
/// wrote): who he is, the talk up to it, and the line itself, whose meaning
/// and facts he must keep.
pub fn writer(w: &World, s: &Scene, line: usize) -> Card {
    let said = s.lines.get(line);
    Card {
        setting: setting(w, s),
        coach: coach(w),
        them: s.who.first().map(|x| person(w, x)).unwrap_or_default(),
        so_far: so_far(s, line),
        his_line: said.map(|l| folk::plain(&l.text)).unwrap_or_default(),
        tell: said.and_then(|l| l.tell.as_deref()).map(folk::plain).unwrap_or_default(),
        ..Card::default()
    }
}

impl Card {
    fn common(&self, out: &mut String) {
        out.push_str(&format!("Setting: {}.\n", self.setting));
        out.push_str(&format!("Coach: {}.\n", self.coach));
        if !self.them.is_empty() {
            out.push_str(&format!("Across from him: {}.\n", self.them));
        }
        if !self.facts.is_empty() {
            out.push_str("Facts of this talk:\n");
            for f in &self.facts {
                out.push_str(&format!("- {f}\n"));
            }
        }
    }

    fn talk(&self, out: &mut String) {
        out.push_str("<talk_so_far>\n");
        for l in &self.so_far {
            out.push_str(&format!("{}: {}\n", l.who, quoted(&l.text)));
        }
        out.push_str("</talk_so_far>\n");
    }

    /// The reader's text: the card, the talk so far and your line.
    pub fn reader_text(&self, words: &str) -> String {
        let mut out = String::from("<card>\n");
        self.common(&mut out);
        if !self.may_name.is_empty() {
            let names: Vec<String> = self.may_name.iter().map(|t| format!("{} {}", t.tag, t.text)).collect();
            out.push_str(&format!("People he may name: {}.\n", names.join("; ")));
        }
        out.push_str("Options:\n");
        for o in &self.options {
            out.push_str(&format!("- {}: {}\n", o.tag, o.text));
        }
        out.push_str("</card>\n");
        self.talk(&mut out);
        out.push_str(&format!("<coach_line>{}</coach_line>", quoted(words)));
        out
    }

    /// The writer's text: the card, the talk so far and your line.
    pub fn writer_text(&self, words: &str) -> String {
        let mut out = String::from("<card>\n");
        self.common(&mut out);
        out.push_str(&format!("What the game decided he answers (keep its meaning and every fact in it): {}\n", self.his_line));
        if !self.tell.is_empty() {
            out.push_str(&format!("How he comes across: {}\n", self.tell));
        }
        if !self.released.is_empty() {
            out.push_str("What your read of him caught with this answer:\n");
            for r in &self.released {
                out.push_str(&format!("- {r}\n"));
            }
        }
        if !self.examples.is_empty() {
            out.push_str("Example lines from people like him for this moment:\n");
            for e in &self.examples {
                out.push_str(&format!("- {e}\n"));
            }
        }
        out.push_str("</card>\n");
        self.talk(&mut out);
        out.push_str(&format!("<coach_line>{}</coach_line>", quoted(words)));
        out
    }
}
