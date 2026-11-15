//! Channels and who's in the room (sections 5.5 and 5.6). Lane N4 fills
//! what each allows; the contact rules stay `calendar/contact.rs`'s.

use super::Them;

/// Where an in-person talk happens. The setting changes how he talks
/// (3.7): the living room with his grandmother there, the locker room.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Place {
    /// Your office.
    #[default]
    Office,
    /// His home (a home visit).
    Home,
    /// The facility (a visit, a tour).
    Facility,
    /// The practice field.
    Field,
    /// The sideline on game day.
    Sideline,
    /// The locker room.
    LockerRoom,
    /// Somewhere else: a restaurant, a hotel, a combine room.
    Out,
}

/// How you reach him (5.5).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Channel {
    /// In person: film, the facility and other people can be shown or
    /// brought in.
    InPerson(Place),
    /// A phone call: about half a sit-down.
    Phone,
    /// A text: short.
    Text,
    /// A video call, in the era it's allowed: film can be shown.
    Video,
    /// Through someone else (his high school coach, his agent, his parent):
    /// you hear his answer secondhand, with their shade.
    Through(Them),
    /// A letter or an email: formal.
    Letter,
    /// A group: every listener hears every line.
    Group(Group),
}

/// A group you talk to (5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Group {
    /// The whole team.
    Team,
    /// A position room (the position's index).
    Position(u8),
    /// Your staff.
    Staff,
    /// The leadership council.
    Council,
    /// A player's family (the player).
    Family(u32),
}

/// Why someone is in the room (5.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Seat {
    /// The man you came to talk to.
    Across,
    /// His decider: matters most on a home visit.
    Decider,
    /// More of his family.
    Family,
    /// His agent: the player says less and the agent negotiates.
    Agent,
    /// His high school coach: he answers some questions for him.
    HsCoach,
    /// Someone you brought: your coordinator, your position coach.
    YourStaff,
    /// A former player you brought to tell his story.
    FormerPlayer,
    /// A teammate, a listener in a group.
    Listener,
}

/// One person in the room. Each hears every line and takes it his own way,
/// with his own hits.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Present {
    pub who: Them,
    pub seat: Seat,
    /// He may answer for the man across (an agent, a high school coach).
    #[serde(default)]
    pub speaks: bool,
}

/// Who's in the room for a talk.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Room {
    pub channel: Channel,
    /// Everyone there, the man across from you first.
    pub present: Vec<Present>,
}
