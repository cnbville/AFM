//! News and inbox items the world produces. NIL warnings (topic 8, follow-up
//! answer 4) come through three channels at once and repeat as things get worse.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// A message from the program's own staff (seen by that program's coach).
    Inbox,
    /// A public news story.
    News,
    /// The NIL grade shows a falling arrow.
    GradeArrow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewsKind {
    NilWarning {
        level: u32,
    },
    CoachHired,
    CoachFired,
    /// A head coach leaves: hired away, retired, or stepped away (dynasty item 11).
    CoachLeft,
    /// A loyal, successful coach kept for a "prove it" year (note 1).
    CoachExtended,
    /// The yearly diversity report on head coaches (note 2).
    DiversityReport,
    NationalTitle,
    /// NCAA penalties for a booster's payments (dynasty item 11.6).
    NcaaPenalty,
    ConferenceTitle,
    StateTitle,
    Realignment,
}

#[derive(Clone, Debug)]
pub struct NewsItem {
    pub year: i32,
    pub kind: NewsKind,
    pub channel: Channel,
    /// The program or school it is about (college program id, if any).
    pub program: Option<usize>,
    pub text: String,
}

/// The three NIL warning messages for a collective that is drying up.
/// `level` is how many years in a row it has been falling.
pub fn nil_warnings(
    year: i32,
    program: usize,
    name: &str,
    level: u32,
    booster_left: bool,
    change_pct: f64,
) -> Vec<NewsItem> {
    let why = if booster_left {
        "a major booster has stopped giving".to_string()
    } else {
        format!("donations fell {:.0}% this year", change_pct.abs())
    };
    let inbox = match level {
        1 => format!("Our collective is warning that {why}. NIL offers will be tighter next cycle."),
        2 => {
            format!("Second straight year of falling collective money: {why}. We will lose recruiting battles on NIL.")
        }
        _ => format!(
            "The collective is drying up ({why}, {level} years running). Expect transfers out and weaker classes."
        ),
    };
    let news = match level {
        1 => format!("{name}'s NIL collective takes a hit: {why}."),
        2 => format!("Money trouble at {name}: the collective is shrinking for a second year."),
        _ => format!("{name}'s NIL collective is drying up, and rivals are circling its roster."),
    };
    vec![
        NewsItem {
            year,
            kind: NewsKind::NilWarning { level },
            channel: Channel::Inbox,
            program: Some(program),
            text: inbox,
        },
        NewsItem {
            year,
            kind: NewsKind::NilWarning { level },
            channel: Channel::News,
            program: Some(program),
            text: news,
        },
        NewsItem {
            year,
            kind: NewsKind::NilWarning { level },
            channel: Channel::GradeArrow,
            program: Some(program),
            text: format!("NIL grade trending down ({})", "↓".repeat(level.min(3) as usize)),
        },
    ]
}
