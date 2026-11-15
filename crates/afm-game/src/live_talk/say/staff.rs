//! The staff meeting typed (live talks lane 6, LT17). A typed line lands
//! through the core's generic reader on one of the meeting's options: his
//! case agreed (951), halfway (952), overruled (953), or your own case
//! instead (954-959, `dialogue::own_moves`). The option is read back as a
//! game plan ask on `Heard.plan`: which setting, which way, how far. Your
//! own case is argued against his numbers and checked after the next game
//! (`dialogue.rs`, `Case.mine`, `check_case`).

use afm_engine::world::World;
use afm_engine::world::folk::dialogue;
use afm_engine::world::folk::scene::{Heard, Kind, PlanAsk, Scene};
use serde_json::Value;

use crate::Game;
use crate::live_talk::card::Card;

/// Adds this lane's own box to a reply (`live_talk::after`, only with a
/// usable key). The meeting's talk box is the core's.
pub fn flag(_g: &Game, _cmd: &str, _v: &mut Value) {}

/// Nothing beyond the meeting's own options, which name each move.
pub fn reader(_w: &World, _s: &Scene, _card: &mut Card) {}

/// The game plan ask the option means (`plan_of`).
pub fn heard(_w: &World, s: &Scene, _reading: &Value, heard: &mut Heard) {
    heard.plan = plan_of(s, heard.pick);
}

/// The game plan ask an option of the staff meeting means: his case or
/// yours, one full step (agreeing, your own) or half (step 0, halfway);
/// none for overruling him.
pub fn plan_of(s: &Scene, pick: u32) -> Option<PlanAsk> {
    if s.kind != Kind::StaffMeeting {
        return None;
    }
    let his = (s.b as u8).min(3);
    let (m, step) = match pick {
        951 => (his, 1),
        952 => (his, 0),
        954..=959 => ((pick - 954) as u8, 1),
        _ => return None,
    };
    let (setting, direction) = dialogue::move_plan(m);
    Some(PlanAsk { setting: setting.to_string(), direction, step })
}

/// The writer's card: on your own case he argues from his numbers.
pub fn writer(_w: &World, s: &Scene, heard: &Heard, card: &mut Card) {
    if s.kind == Kind::StaffMeeting && (954..=959).contains(&heard.pick) {
        card.released.push(format!(
            "The head coach went his own way: {}. He disagrees and points to the numbers from the last game.",
            dialogue::MOVES[(heard.pick - 954) as usize]
        ));
    }
}

/// This lane's own reader rules: none, the core's generic ones fit.
pub fn rules(_s: &Scene) -> Option<(&'static str, Value)> {
    None
}
