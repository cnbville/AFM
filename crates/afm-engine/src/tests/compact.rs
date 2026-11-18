//! Memory compaction (final build lane 1): the compact stores read back
//! every value bit for bit and write the same bytes as the plain forms, so
//! results and world files don't move.

use crate::player::OptF64;
use crate::world::records::{STAT_COUNT, StatSet};

#[derive(serde::Serialize, serde::Deserialize)]
struct Plain {
    #[serde(with = "serde_big_array::BigArray")]
    s: [f32; STAT_COUNT],
    o: Option<f64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Compact {
    s: StatSet,
    o: OptF64,
}

#[test]
fn a_stat_row_keeps_every_value_and_the_file_shape() {
    let mut a = [0f32; STAT_COUNT];
    a[0] = 41.0;
    a[3] = -0.0;
    a[7] = 312.5;
    a[STAT_COUNT - 1] = 1e-9;
    let s = StatSet::from(a);
    for k in 0..STAT_COUNT {
        assert_eq!(s.get(k).to_bits(), a[k].to_bits(), "stat {k}");
    }
    assert_eq!(StatSet::default().to_array(), [0f32; STAT_COUNT]);
    // Written exactly as the plain array, and read back from it.
    for o in [None, Some(0.0), Some(-3.25), Some(87.123456789)] {
        let plain = rmp_serde::to_vec_named(&Plain { s: a, o }).unwrap();
        let compact = rmp_serde::to_vec_named(&Compact { s: s.clone(), o: o.into() }).unwrap();
        assert_eq!(plain, compact);
        let back: Compact = rmp_serde::from_slice(&plain).unwrap();
        assert_eq!(back.s.to_array().map(f32::to_bits), a.map(f32::to_bits));
        assert_eq!(back.o.get().map(f64::to_bits), o.map(f64::to_bits));
    }
}

/// A world file from before the world reader's stats (65 a row) still
/// loads: the stats it lacks read as zero.
#[test]
fn an_older_shorter_stat_row_still_reads() {
    #[derive(serde::Serialize)]
    struct Old {
        s: Vec<f32>,
    }
    #[derive(serde::Deserialize)]
    struct New {
        s: StatSet,
    }
    let old: Vec<f32> = (0..65).map(|k| k as f32 + 0.5).collect();
    let bytes = rmp_serde::to_vec_named(&Old { s: old.clone() }).unwrap();
    let back: New = rmp_serde::from_slice(&bytes).unwrap();
    for k in 0..STAT_COUNT {
        assert_eq!(back.s.get(k), old.get(k).copied().unwrap_or(0.0), "stat {k}");
    }
}

#[test]
fn season_rows_grow_by_one_not_by_doubling() {
    use crate::development::{SeasonContext, develop_season};
    use crate::player::{Level, PlayingTime};
    let mut rng = crate::rng::Rng::new(7);
    let mut p = crate::player::generate_prospect(&mut rng, Default::default());
    for _ in 0..5 {
        develop_season(&mut p, &SeasonContext::new(Level::HS, 10.0, PlayingTime::Starter), &mut rng);
        assert_eq!(p.history.capacity(), p.history.len());
    }
}

/// The game log builds its rows lean (`wlog/lean.rs`): one pass that keeps
/// only the fields that are not zero, false or empty, rounded the log's
/// way. It has to hold the same as the full value compacted afterwards.
#[test]
fn lean_rows_match_the_full_value_compacted() {
    use crate::game::drives::drives;
    use crate::game::quick_game;
    use crate::game::synthetic::synthetic_league;
    use crate::player::Level;
    use crate::world::wlog::games::compact;
    use crate::world::wlog::lean::lean;
    // Lean drops every empty field as it goes; only a variant keeps its
    // name over an empty value (`{"Challenge": 0}`: the home side
    // challenged), which compacting afterwards lost.
    fn no_empty(v: &serde_json::Value, what: &str) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    let variant = k.starts_with(|c: char| c.is_ascii_uppercase());
                    assert!(variant || !crate::world::wlog::games::empty(x), "{what} kept {k}: {x}");
                    no_empty(x, what);
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| no_empty(x, what)),
            _ => {}
        }
    }
    let same = |what: &str, full: serde_json::Value, lean: serde_json::Value| {
        no_empty(&lean, what);
        let (mut a, mut b) = (full, lean);
        compact(&mut a);
        compact(&mut b);
        assert_eq!(a, b, "{what}");
    };
    for level in [Level::HS, Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 4, 11);
        for g in 0..3u64 {
            let r = quick_game(&teams[g as usize], &teams[3 - g as usize], level, 9_100 + g);
            let s = &r.stats;
            for side in 0..2 {
                same("team box", serde_json::to_value(&s.team[side]).unwrap(), lean(&s.team[side]));
                for (id, l) in &s.players[side] {
                    same(&format!("player {id}"), serde_json::to_value(l).unwrap(), lean(l));
                }
            }
            for p in &s.plays {
                same(&format!("{level:?} snap {}", p.n), serde_json::to_value(p).unwrap(), lean(p));
            }
            same("drives", serde_json::to_value(drives(s)).unwrap(), lean(&drives(s)));
        }
    }
}

/// A part kept only once used (memory diet, lane M1): an empty one reads
/// as blank, a write makes it, and a part blank again is dropped.
#[test]
fn a_stored_part_is_kept_only_once_it_holds_something() {
    use crate::world::Later;
    use crate::world::stored::Stored;
    let mut s: Stored<Later> = Stored::default();
    assert!(s.stored().is_none());
    assert_eq!(s.college_seasons, 0);
    assert!(s.path.is_empty());
    s.college_seasons = 2;
    assert!(s.stored().is_some());
    s.college_seasons = 0;
    s.tidy();
    assert!(s.stored().is_none());
    assert!(Stored::new(Later::default()).stored().is_none());
    assert!(Stored::new(Later { walk_on: true, ..Default::default() }).stored().is_some());
}

/// People are written flat, exactly as before the split (every field by
/// name, in the old order), and read back to the same bytes; a high
/// schooler who never used his college part keeps none after a load.
#[test]
fn people_keep_their_file_shape_and_only_the_parts_they_use() {
    use crate::world::{Person, Stage};
    // The empty world has no players: two high school classes, and a few
    // of them sent on to college so both kinds of person go through a file.
    let mut w = crate::tests::legacy::seat("college");
    w.add_class_aged(16);
    w.add_class_aged(17);
    for p in w.people.iter_mut().take(6) {
        p.stage = Stage::College;
        p.college_seasons = 1;
        p.walk_on = true;
    }
    let p = &w.people[0];
    let names: Keys = rmp_serde::from_slice(&rmp_serde::to_vec_named(p).unwrap()).unwrap();
    assert_eq!(names.0, PERSON_FIELDS);
    let cal: Keys = rmp_serde::from_slice(&rmp_serde::to_vec_named(&p.cal).unwrap()).unwrap();
    let cal: Vec<&str> = cal.0.iter().map(String::as_str).collect();
    for f in CAL_FIELDS {
        // `declare_why` and `cases` are written only when set, as before.
        assert!(cal.contains(f) || matches!(*f, "declare_why" | "cases"), "cal field {f}");
    }
    assert!(cal.iter().all(|f| CAL_FIELDS.contains(f)));
    let mut kept = 0;
    for p in &w.people {
        let bytes = rmp_serde::to_vec_named(p).unwrap();
        let back: Person = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(rmp_serde::to_vec_named(&back).unwrap(), bytes, "{}", p.name);
        assert_eq!(format!("{back:?}"), format!("{p:?}"));
        if back.stage == Stage::HighSchool && back.later.stored().is_some() {
            kept += 1;
        }
        if back.later.stored().is_none() {
            assert_eq!(back.college_seasons, 0);
            assert!(back.path.is_empty() && back.college_staff.is_empty());
        }
    }
    let hs = w.people.iter().filter(|p| p.stage == Stage::HighSchool).count();
    assert!(hs > 100, "{hs} high schoolers");
    assert!(w.people.iter().filter(|p| p.later.stored().is_some()).count() >= 6);
    assert!(kept * 4 < hs, "{kept} of {hs} high schoolers keep a college part");
}

/// A person's fields in world files, in order (before lane M1's split).
const PERSON_FIELDS: &[&str] = &[
    "name",
    "ethnicity",
    "player",
    "rng",
    "bg",
    "family",
    "home_state",
    "hs",
    "stage",
    "college",
    "class_of",
    "born",
    "followers",
    "spotlight",
    "brand",
    "stars",
    "camp_stars",
    "regular_talent",
    "visibility",
    "college_seasons",
    "redshirt_used",
    "sit_out",
    "steered",
    "nil_income",
    "pending_dip",
    "last_role",
    "hs_lines",
    "last_snaps",
    "college_snaps",
    "reps",
    "last_performance",
    "transfers",
    "juco",
    "path",
    "on_notice",
    "gpa_trend",
    "gpa_at_signing",
    "academic_redshirt",
    "walk_on",
    "transfer_from",
    "college_staff",
    "incident",
    "notes",
    "stay_home",
    "childhood_team",
    "cal",
    "tests",
    "pro",
    "mood",
    "mood_why",
    "trust",
    "trust_in",
    "roads",
    "looks",
];

/// A person's calendar fields in world files, in order (before lane M1).
const CAL_FIELDS: &[&str] = &[
    "count_pending",
    "academic_sit",
    "trouble",
    "spring_ineligible",
    "fall_ineligible",
    "lean",
    "lean_events",
    "whispered",
    "draft_read",
    "pro_position",
    "opted_out",
    "opt_out_on",
    "redshirt_trick",
    "commit",
    "signed",
    "commit_timing",
    "spring_notice",
    "spring_gain_ahead",
    "combine_invite",
    "all_star",
    "hs_all_star",
    "test_score",
    "transfer_sit",
    "sitting_transfer",
    "portal",
    "hs_grade_out",
    "declare_plan",
    "declare_why",
    "graduated",
    "all_state",
    "offers",
    "visits",
    "service_read",
    "early_offers",
    "advisory",
    "may_graduate",
    "payday",
    "nil_role",
    "nil_deal",
    "nil_keep",
    "nil_buyout",
    "nil_school",
    "cut",
    "late",
    "home_talks",
    "cases",
];

/// A map's keys, in the order written.
struct Keys(Vec<String>);

impl<'de> serde::Deserialize<'de> for Keys {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Keys;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut m: A) -> Result<Keys, A::Error> {
                let mut k = Vec::new();
                while let Some(key) = m.next_key::<String>()? {
                    m.next_value::<serde::de::IgnoredAny>()?;
                    k.push(key);
                }
                Ok(Keys(k))
            }
        }
        d.deserialize_map(V)
    }
}
