//! The parts of section 6b around the practice week: the offseason on
//! their own and report day (topic 9), other sports and the heat in high
//! school (topic 10), breaking the rules and being found out (topic 3),
//! and camp as something the media watch (topic 8). Your team's side runs
//! from `seat::week`; other sports also move every high schooler's week in
//! the world's pass (`week::monday`).

use super::rules::{Block, Tier};
use super::{FAMILIES, Family};
use crate::calendar::Date;
use crate::player::{Player, Trait};
use crate::rng::clamp;

/// A high schooler's other sport (people topic 6, decided: other sports
/// split a high schooler's training time).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Sport {
    Basketball,
    Wrestling,
    Track,
    Baseball,
}

impl Sport {
    pub fn name(self) -> &'static str {
        ["basketball", "wrestling", "track", "baseball"][self as usize]
    }

    /// The sport's high school season (the usual NFHS calendar; states
    /// differ by a few weeks).
    pub fn in_season(self, d: Date) -> bool {
        let (_, m, day) = d.parts();
        let md = m * 100 + day;
        match self {
            Sport::Basketball => !(311..1115).contains(&md),
            Sport::Wrestling => !(301..1115).contains(&md),
            Sport::Track => (301..=531).contains(&md),
            Sport::Baseball => (215..=531).contains(&md),
        }
    }

    /// What a week of the sport does to his football training, family by
    /// family (proposed, not confirmed: basketball costs the winter
    /// lifting and keeps him quick and fit; wrestling builds strength,
    /// hand fighting and conditioning; track grows speed and costs
    /// strength work; baseball competes with spring ball).
    pub fn ratio(self) -> [f64; FAMILIES] {
        // strength, speed, conditioning, technique, contact, kicking, mental
        match self {
            Sport::Basketball => [0.65, 1.06, 1.12, 0.75, 0.8, 1.0, 0.9],
            Sport::Wrestling => [1.04, 1.0, 1.15, 0.8, 1.08, 1.0, 0.9],
            Sport::Track => [0.8, 1.15, 1.1, 0.75, 0.85, 1.0, 0.9],
            Sport::Baseball => [0.85, 1.0, 0.95, 0.65, 0.8, 1.0, 0.85],
        }
    }
}

fn hash01(id: u32, salt: u64) -> f64 {
    let mut z = (id as u64) << 16 ^ salt ^ 0x5B0A_7E11;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// His other sport, if any: about four in ten high school football
/// players play one (proposed, not confirmed, from the NFHS's
/// participation counts and surveys of multi-sport athletes), chosen by
/// his body: the fastest run track, the tall and quick play basketball,
/// the strong and heavy wrestle, the rest play baseball.
pub fn other_sport(p: &Player) -> Option<Sport> {
    let u = hash01(p.id, 0x5907);
    if u > 0.42 || p.position.is_specialist() && u > 0.2 {
        return None;
    }
    let a = &p.attributes;
    let speed = a.get(crate::Attr::Speed);
    let agility = a.get(crate::Attr::Agility);
    let strength = a.get(crate::Attr::Strength);
    let (h, wt) = (p.body.height, p.body.weight);
    let pick = hash01(p.id, 0x5908);
    Some(if speed >= 70.0 && pick < 0.7 {
        Sport::Track
    } else if h >= 74.0 && agility >= 55.0 && wt < 240.0 && pick < 0.75 {
        Sport::Basketball
    } else if wt >= 190.0 && strength >= 55.0 && pick < 0.6 {
        Sport::Wrestling
    } else if wt >= 260.0 {
        // Linemen throw the shot and discus.
        Sport::Track
    } else {
        Sport::Baseball
    })
}

/// The other sport's pull on a high schooler's football week now: in its
/// season and outside football's camp and season.
pub fn sport_ratio(p: &Player, block: Block, d: Date) -> Option<(Sport, [f64; FAMILIES])> {
    if !matches!(block, Block::Winter | Block::Spring | Block::Summer | Block::Quiet) {
        return None;
    }
    let s = other_sport(p)?;
    s.in_season(d).then(|| (s, s.ratio()))
}

/// How hard a player really works when nobody makes him (topic 9): his
/// professionalism, ambition and work ethic, a little less for one just
/// paid, more in a contract year. About 1 for the average player.
pub fn own_effort(p: &Player, just_paid: bool, contract_year: bool) -> f64 {
    let t = &p.personality.traits;
    clamp(
        1.0 + 0.12 * t.z(Trait::Professionalism)
            + 0.06 * t.z(Trait::Ambition)
            + 0.01 * (p.personality.work_ethic - 10.0)
            - if just_paid { 0.08 } else { 0.0 }
            + if contract_year { 0.06 } else { 0.0 },
        0.55,
        1.45,
    )
}

/// One sign of a player's offseason work and who it comes from (topic 9):
/// each source is only as true as the person giving it, and a player's
/// own clip shows his best day.
pub struct Sign {
    pub dept: &'static str,
    pub text: String,
}

/// Private trainers by kind (generic: the game names no real trainer).
const TRAINERS: [&str; 8] = [
    "a quarterback coach in Southern California",
    "a pass-rush specialist in Houston",
    "a speed coach in Florida",
    "a strength gym back home",
    "a receivers' academy in Atlanta",
    "an offensive line camp in Arizona",
    "a defensive backs coach in Dallas",
    "his old high school's weight room",
];

/// A sign of how his time off is going, or None this week. `effort` is
/// the truth; `noise` a fixed draw for the week in -1..1; `pick` a
/// uniform for the source.
pub fn sign(tier: Tier, p: &Player, name: &str, effort: f64, noise: f64, pick: f64) -> Option<Sign> {
    let link = format!("[[p:{}|{name}]]", p.id);
    match tier {
        Tier::Pro => {
            if pick < 0.3 {
                // The facility log: who came in, and how often (true).
                let days = clamp((effort - 0.6) * 6.0 + noise * 0.6, 0.0, 5.0).round() as u32;
                Some(Sign {
                    dept: "facility",
                    text: if days == 0 {
                        format!("{link} hasn't been in the building this week.")
                    } else {
                        format!("{link} was in the facility {days} of 5 days this week.")
                    },
                })
            } else if pick < 0.5 {
                // His own workout clip: always his best day.
                let shown = effort + 0.25 + 0.1 * noise;
                Some(Sign {
                    dept: "social",
                    text: if shown > 1.15 {
                        format!("{link} posted a workout clip: sled pushes and route work, looking sharp.")
                    } else {
                        format!("{link} posted a beach clip and a short lifting video.")
                    },
                })
            } else if pick < 0.7 {
                // The strength coach's check-in (fair, a little noisy).
                let read = effort + 0.12 * noise;
                Some(Sign {
                    dept: "strength",
                    text: if read > 1.1 {
                        format!("{link} is sending in his numbers every week; the strength coach likes what he sees.")
                    } else if read > 0.9 {
                        format!("{link} checked in with the strength coach: on his program.")
                    } else {
                        format!("{link} hasn't answered the strength coach's check-ins.")
                    },
                })
            } else if pick < 0.82 {
                let k = (hash01(p.id, 0x7A1) * TRAINERS.len() as f64) as usize % TRAINERS.len();
                Some(Sign {
                    dept: "strength",
                    text: if effort > 0.95 {
                        format!("{link} is training with {} this summer.", TRAINERS[k])
                    } else {
                        format!("{link} told the staff he's working out at home.")
                    },
                })
            } else {
                // A teammate's or an agent's word (the agent always says great).
                let agent = pick > 0.92;
                let read = if agent { effort + 0.3 } else { effort + 0.2 * noise };
                Some(Sign {
                    dept: if agent { "agent" } else { "teammate" },
                    text: if agent {
                        format!("{link}'s agent says he's in the best shape of his life.")
                    } else if read > 1.05 {
                        format!("A teammate says {link} has been throwing with the receivers twice a week.")
                    } else {
                        format!("A teammate says nobody has seen much of {link} this summer.")
                    },
                })
            }
        }
        Tier::College => {
            // The summer hours are the staff's: the strength staff see it all.
            let read = effort + 0.06 * noise;
            let lbs = clamp((read - 0.85) * 60.0, -10.0, 30.0).round() as i32;
            Some(Sign {
                dept: "strength",
                text: if read > 1.05 {
                    format!("{link}: squat up {} pounds this summer, at every session.", lbs.max(5))
                } else if read > 0.85 {
                    format!("{link}: making his summer sessions, numbers steady.")
                } else {
                    format!("{link} has missed summer sessions; his numbers are down.")
                },
            })
        }
        Tier::High => {
            let read = effort + 0.15 * noise;
            Some(Sign {
                dept: "strength",
                text: if read > 1.08 {
                    format!("{link} is at every summer weight session and went to two 7-on-7 tournaments.")
                } else if read > 0.9 {
                    format!("{link} is coming to summer weights most mornings.")
                } else {
                    format!("{link} has barely been to summer weights; people in town say he's working a summer job.")
                },
            })
        }
    }
}

/// The chance he fails report day's conditioning test (topic 9: failing
/// it sends a player to the non-football injury list). About 3% for an
/// average summer, far more for a lost one (proposed, not confirmed).
pub fn fail_chance(effort: f64, stamina: f64, tier: Tier) -> f64 {
    let base = match tier {
        Tier::Pro => 0.03,
        Tier::College => 0.025,
        Tier::High => 0.02,
    };
    clamp(base * (-4.0 * (effort - 1.0)).exp() * (-0.04 * (stamina - 70.0)).exp(), 0.003, 0.6)
}

/// The chance a week of practice outside the rules is found out (topic 3:
/// a player complains to the union or the NCAA, a video, an assistant who
/// leaves), more for each extra breach and when someone got hurt
/// (proposed, not confirmed: most real cases surface within weeks).
pub fn found_out(tier: Tier, breaches: usize, hurt: bool) -> f64 {
    let base = match tier {
        Tier::Pro => 0.3,
        Tier::College => 0.2,
        Tier::High => 0.15,
    };
    clamp(base * (1.0 + 0.25 * breaches.saturating_sub(1) as f64) * if hurt { 1.6 } else { 1.0 }, 0.0, 0.9)
}

/// What a case costs, by level (the real ones: the Seahawks lost OTA days
/// in 2012 and 2014 and Pete Carroll was fined; the Jaguars $200,000 and
/// Urban Meyer $100,000 in 2021; NCAA countable-hours cases cost
/// practice time and a self-imposed reduction; state associations cut
/// practices and put a program on warning). Returns the words and the
/// practices lost in the next block of the same kind.
pub fn penalty(tier: Tier, block: Block, cases_before: usize) -> (String, u16) {
    let again = cases_before > 0;
    match tier {
        Tier::Pro => (
            if again {
                "the league takes away a week of next year's OTAs and fines the club $400,000 and the head coach $200,000".into()
            } else {
                "the league takes away a week of next year's OTAs and fines the club $200,000 and the head coach $100,000".into()
            },
            if block == Block::Season { 1 } else { 3 },
        ),
        Tier::College => (
            if again {
                "the NCAA calls it a Level II violation: practice time cut and two years' probation".into()
            } else {
                "the NCAA calls it a Level III violation: the program gives back twice the extra time in practices"
                    .into()
            },
            if again { 6 } else { 3 },
        ),
        Tier::High => (
            if again {
                "the state association cuts practices and puts the program on probation for a year".into()
            } else {
                "the state association issues a warning and takes away practices".into()
            },
            2,
        ),
    }
}

/// A short reason for each choice in an AI staff's week (topic 11: staffs
/// decide like real people and leave a reason), from its head coach's
/// styles.
pub fn staff_reasons(st: &crate::tendencies::Style, tier: Tier) -> Vec<String> {
    let mut v = Vec::new();
    if st.physical >= 14.0 {
        v.push("We go live in team periods: you learn to tackle by tackling.".to_string());
    } else if st.physical <= 7.0 {
        v.push("We tackle to the ground as little as we can: thud and control, shoulder tackling, and fresh legs on game day.".to_string());
    } else {
        v.push("Most of our contact is thud: full speed, no one on the ground.".to_string());
    }
    if st.freedom <= 7.0 {
        v.push("Individual work and walkthroughs get the most minutes: we coach the details.".to_string());
    } else if st.freedom >= 14.0 {
        v.push("Team periods get the most minutes: the players learn by playing.".to_string());
    }
    if st.risk >= 14.0 {
        v.push("We practise fast, a snap every 15 seconds: more reps, and the players feel it.".to_string());
    }
    match tier {
        Tier::Pro => v.push("Veterans over 30 get a day off midweek once their load is up.".to_string()),
        Tier::College => {
            v.push("The young players get their own period on Sunday or Monday while the starters recover.".to_string())
        }
        Tier::High => v.push("The JV practises with us early in the week and plays on Thursday.".to_string()),
    }
    v
}

/// Words for a share of a family's year (the calendar's part, not his
/// growth): how much of the year's work is behind him.
pub fn year_words(landed: f64) -> &'static str {
    if landed < 0.15 {
        "the year's work is just starting"
    } else if landed < 0.45 {
        "early in the year's work"
    } else if landed < 0.75 {
        "about halfway through the year's work"
    } else if landed < 0.97 {
        "most of the year's work is behind him"
    } else {
        "the year's work is done"
    }
}

/// The family a player's plan pulls on, for the screens.
pub fn focus_family(a: crate::Attr) -> Family {
    super::family_of(a)
}
