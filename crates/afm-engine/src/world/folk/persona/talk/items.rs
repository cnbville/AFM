//! His real items a line can touch (section 5.2), in words for the ear's
//! card: his wants, what's on his mind from his fact sheet, his pride,
//! your promises to him, his close friends, his family, and what he told
//! you before. The ear must name one of these for a hit to count. A worry
//! you don't know is on the card too, since he feels it whether you know
//! or not; "Heard as" names it only once you know it (question 2).

use super::super::Them;
use super::super::ear::Item;
use super::super::wants::{Want, WantKind};
use crate::world::World;
use crate::world::folk::{self as folk, claims, talk_points as tp, ties};

/// The most fact sheet points the card lists. Proposed, not confirmed.
pub const MAX_POINTS: usize = 8;
/// The most friends the card lists. Proposed, not confirmed.
pub const MAX_FRIENDS: usize = 4;

/// A want in words.
pub fn want_words(k: WantKind) -> &'static str {
    match k {
        WantKind::Role => "his role: playing time, the job he sees for himself",
        WantKind::Money => "money: NIL, his pay",
        WantKind::Security => "security: a spot that lasts",
        WantKind::Winning => "winning, and being somewhere that matters",
        WantKind::Place => "being close to home",
        WantKind::Development => "getting better, a path to the next level",
        WantKind::Academics => "his classes and his degree",
        WantKind::Name => "the program's name",
        WantKind::Loyalty => "loyalty to this team",
        WantKind::Family => "his family's good",
        WantKind::Friends => "his friends on the team",
        WantKind::Fee => "his fee",
        WantKind::Career => "his own next step",
        WantKind::Scheme => "his say in the scheme",
        WantKind::Club => "his club's plans",
        WantKind::Goals => "his goals for you",
        WantKind::Booster(_) => "what his money buys",
        WantKind::Kids => "his kids' futures",
    }
}

/// Every item on his card, with its words, in a fixed order.
pub fn items(w: &World, him: Them, wants: &[Want]) -> Vec<(Item, String)> {
    let mut out = Vec::new();
    for (k, x) in wants.iter().enumerate() {
        let met = if x.met { " (he has it)" } else { "" };
        out.push((Item::Want(k as u8), format!("what he wants: {}{met}", want_words(x.kind))));
    }
    let Them::Player(id) = him else {
        out.push((Item::Pride, "his pride".into()));
        return out;
    };
    let Some(i) = folk::person(w, id) else { return out };
    let sheet = claims::player(w, i);
    let mut seen = Vec::new();
    for p in sheet.points.iter().filter(|p| p.whose != claims::Whose::Yours && tp::talkable(p.kind)) {
        if seen.contains(&p.kind) {
            continue;
        }
        seen.push(p.kind);
        out.push((Item::Point(p.kind.key().to_string()), format!("on his mind: {}", folk::plain(&tp::worry_words(p)))));
        if seen.len() == MAX_POINTS {
            break;
        }
    }
    out.push((Item::Pride, "his pride".into()));
    let to = him.promise_to();
    for (k, p) in w.folk.promises.iter().enumerate().filter(|(_, p)| Some(p.to) == to) {
        let state = match p.kept {
            Some(true) => "kept",
            Some(false) => "broken",
            None => "open",
        };
        if p.kept == Some(true) && p.made.days_to(w.today) > 365 {
            continue;
        }
        out.push((Item::Promise(k as u32), format!("your promise to him ({state}): {}", p.kind.words())));
    }
    for f in ties::friends(w, id).into_iter().take(MAX_FRIENDS) {
        if let Some(j) = folk::person(w, f) {
            out.push((Item::Friend(f), format!("his friend {}", w.people[j].name)));
        }
    }
    out.push((Item::Family, "his family".into()));
    if w.folk.live.persona.talks.iter().any(|t| t.with == him && t.lines.iter().any(|l| !l.his.is_empty())) {
        out.push((Item::HisWord, "what he told you before".into()));
    }
    out
}

/// Whether you already know this item about him, so "Heard as" may name
/// it (question 2): a point only once it's known to you; your promises,
/// his friends, his family and his pride always; his wants only through
/// what your read of him shows, so never by name here.
pub fn known(w: &World, him: Them, item: &Item) -> bool {
    match item {
        // Your own school's point to a recruit: yours to know (lane N5a).
        Item::Point(key) if pitch_point(key).is_some() => true,
        Item::Point(key) => {
            let (Them::Player(id), Some(k)) = (him, claims::Kind::from_key(key)) else { return false };
            folk::person(w, id).and_then(|i| claims::player(w, i).get(k).map(|p| p.known)).unwrap_or(false)
        }
        Item::Want(_) => false,
        _ => true,
    }
}

/// An item's short name for "Heard as", once you know it.
pub fn short(w: &World, him: Them, item: &Item) -> String {
    match item {
        Item::Point(key) if pitch_point(key).is_some() => {
            pitch_point(key).and_then(|k| k.group()).map_or(String::new(), |g| g.name().to_lowercase())
        }
        Item::Point(key) => {
            let (Them::Player(id), Some(k)) = (him, claims::Kind::from_key(key)) else { return String::new() };
            folk::person(w, id)
                .and_then(|i| claims::player(w, i).get(k).map(|p| folk::plain(&tp::chip_name(p)).to_lowercase()))
                .unwrap_or_default()
        }
        Item::Pride => "his pride".into(),
        Item::Promise(k) => w.folk.promises.get(*k as usize).map_or("your promise".into(), |p| p.kind.words()),
        Item::Friend(f) => folk::person(w, *f).map_or("his friend".into(), |j| w.people[j].name.clone()),
        Item::Family => "his family".into(),
        Item::HisWord => "what he told you".into(),
        Item::Want(_) => String::new(),
    }
}

/// A point of your school's case to a recruit (`Family::Pitch`), by key.
fn pitch_point(key: &str) -> Option<claims::Kind> {
    claims::Kind::from_key(key).filter(|k| matches!(k.family(), claims::Family::Pitch(_)))
}
