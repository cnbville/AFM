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
    "nil_paid",
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

#[derive(serde::Serialize, serde::Deserialize)]
struct PlainRow {
    team: Option<usize>,
    depth: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CompactRow {
    team: crate::player::OptIdx,
    depth: crate::player::DepthLabel,
}

/// Memory diet item 5: a season row's pro team and depth label are small
/// codes in memory, written, read and printed as the plain options.
#[test]
fn a_season_rows_team_and_depth_keep_the_file_shape() {
    use crate::player::{DepthLabel, OptIdx};
    let cases: [(Option<usize>, Option<&str>); 5] = [
        (None, None),
        (Some(0), Some("starter")),
        (Some(31), Some("WR2")),
        (None, Some("practice squad")),
        (Some(7), None),
    ];
    for (team, depth) in cases {
        let plain = PlainRow { team, depth: depth.map(str::to_string) };
        let compact = CompactRow { team: team.into(), depth: depth.map_or(DepthLabel::NONE, DepthLabel::some) };
        let a = rmp_serde::to_vec_named(&plain).unwrap();
        assert_eq!(a, rmp_serde::to_vec_named(&compact).unwrap());
        let back: CompactRow = rmp_serde::from_slice(&a).unwrap();
        assert_eq!(back.team.get(), team);
        assert_eq!(back.depth.as_deref(), depth);
        assert_eq!(format!("{:?}", back.team), format!("{team:?}"));
        assert_eq!(format!("{:?}", back.depth), format!("{:?}", depth.map(str::to_string)));
        assert_eq!(crate::world::wlog::fp::of(&plain), crate::world::wlog::fp::of(&compact));
    }
    assert!(OptIdx::default().is_none() && DepthLabel::default().is_none());
    // The row itself: 144 bytes before the codes, 104 after.
    assert!(
        std::mem::size_of::<crate::player::SeasonRecord>() <= 104,
        "{}",
        std::mem::size_of::<crate::player::SeasonRecord>()
    );
}

/// A coach's fields in world files, in order (before lane M2's split).
const COACH_FIELDS: &[&str] = &[
    "id",
    "name",
    "ethnicity",
    "age",
    "born",
    "ability",
    "learning_ceiling",
    "reputation",
    "personality",
    "mentor",
    "ex_pro",
    "job",
    "history",
    "recent",
    "wins",
    "losses",
    "titles",
    "career_titles",
    "retired",
    "prove_it",
    "ties",
    "came_up",
    "units",
    "sportsmanship",
    "home_state",
    "decline_from",
    "decline_rate",
    "show_cause",
    "contract",
    "systems",
    "stat_buzz",
    "style_set",
    "freedom_norm",
    "yours",
    "played",
    "renown",
    "fame",
    "junior",
];

/// The working part's fields, which a retired coach is written without.
const WORK_FIELDS: &[&str] = &[
    "learning_ceiling",
    "recent",
    "prove_it",
    "ties",
    "units",
    "sportsmanship",
    "decline_from",
    "decline_rate",
    "contract",
    "systems",
    "stat_buzz",
    "style_set",
    "freedom_norm",
    "yours",
];

/// Memory diet item 9: a working coach is written flat, every field in the
/// old order, and reads back to the same bytes; a retired coach is written
/// without his working part, and one read from an older file (with it)
/// drops it as he loads. What his page reads stays.
#[test]
fn coaches_keep_their_file_shape_and_retired_ones_drop_their_working_part() {
    use crate::world::coach::Coach;
    let w = crate::tests::legacy::seat("college");
    let c = w.coaches.iter().find(|c| !c.retired && c.job.is_some()).expect("a working coach").clone();
    let bytes = rmp_serde::to_vec_named(&c).unwrap();
    let names: Keys = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(names.0, COACH_FIELDS);
    let back: Coach = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(rmp_serde::to_vec_named(&back).unwrap(), bytes);
    assert_eq!(format!("{back:?}"), format!("{c:?}"));
    // Retired: the older file form (every field) loads without the part.
    let mut old = c.clone();
    old.retired = true;
    let old_bytes = rmp_serde::to_vec_named(&old).unwrap();
    let names: Keys = rmp_serde::from_slice(&old_bytes).unwrap();
    assert_eq!(names.0, COACH_FIELDS, "the part is dropped only at the year's cleanup or a load");
    let gone: Coach = rmp_serde::from_slice(&old_bytes).unwrap();
    assert!(gone.work.is_none());
    assert_eq!((gone.name.as_str(), gone.wins, gone.career_titles, gone.mentor), (c.name.as_str(), c.wins, c.career_titles, c.mentor));
    assert_eq!(gone.history.len(), c.history.len());
    let short = rmp_serde::to_vec_named(&gone).unwrap();
    let names: Keys = rmp_serde::from_slice(&short).unwrap();
    let want: Vec<&str> = COACH_FIELDS.iter().copied().filter(|f| !WORK_FIELDS.contains(f)).collect();
    assert_eq!(names.0, want);
    let again: Coach = rmp_serde::from_slice(&short).unwrap();
    assert!(again.work.is_none());
    assert_eq!(rmp_serde::to_vec_named(&again).unwrap(), short);
    // The coach itself is about half the size it was (920 bytes).
    assert!(std::mem::size_of::<Coach>() < 520, "{}", std::mem::size_of::<Coach>());
}

/// Memory diet item 3: an alumnus's packed player reads back whole, and
/// writes and prints as the player did.
#[test]
fn a_kept_player_reads_writes_and_prints_as_the_player() {
    use crate::world::stored::KeptPlayer;
    let mut w = crate::tests::legacy::seat("college");
    w.add_class_aged(17);
    let p = w.people[0].player.clone();
    let k = KeptPlayer::new(&p);
    assert_eq!((k.id, k.position), (p.id, p.position));
    assert_eq!(format!("{:?}", k.get()), format!("{p:?}"));
    assert_eq!(format!("{k:?}"), format!("{p:?}"));
    let bytes = rmp_serde::to_vec_named(&p).unwrap();
    assert_eq!(rmp_serde::to_vec_named(&k).unwrap(), bytes);
    let back: KeptPlayer = rmp_serde::from_slice(&bytes).unwrap();
    assert!(back == k);
    assert_eq!(crate::world::wlog::fp::of(&p), crate::world::wlog::fp::of(&k));
    // Smaller than the player in place before his season rows (about 860
    // bytes against 1,032 for a 17-year-old with a few rows).
    assert!(k.packed_len() < std::mem::size_of::<crate::player::Player>(), "{} bytes packed", k.packed_len());
}

/// Memory diet item 10: an open season's row packed after two idle sweeps
/// reads back the same, takes the next game on top, and the store writes
/// every row as before.
#[test]
fn an_idle_tally_row_packs_and_reads_back_the_same() {
    use crate::calendar::Date;
    use crate::world::tally::Level;
    use crate::world::tally::players::{Game, Open};
    let game = |id: u32| Game {
        id,
        side: 0,
        pos: None,
        played: true,
        started: id == 1,
        snaps: [40, 0, 3],
        depth: vec![(3, 1.0), (17, 2.5)],
        ..Default::default()
    };
    let day = Date::ymd(2026, 9, 5);
    let mut o = Open::default();
    o.add(Level::College, day, 7, 1, game(1));
    o.add(Level::College, day, 7, 1, game(2));
    o.add(Level::Nfl, day, 3, 2, game(9));
    let before: Vec<(u64, Vec<u8>)> =
        o.rows().iter().map(|(k, s)| (*k, crate::world::tally::file::encode(&**s))).collect();
    let written = rmp_serde::to_vec_named(&o).unwrap();
    // Three weekly sweeps on, only man 1 has played: men 2 and 9 pack.
    o.add(Level::College, day.add(7), 7, 3, game(1));
    o.add(Level::College, day.add(14), 7, 4, game(1));
    assert_eq!(o.held().1, 0, "a week off is a bye, not the season's end");
    o.add(Level::College, day.add(21), 7, 5, game(1));
    assert_eq!(o.held().1, 2, "two rows packed");
    for (k, b) in &before[1..] {
        let row = o.rows().into_iter().find(|r| r.0 == *k).unwrap().1;
        assert_eq!(&crate::world::tally::file::encode(&*row), b);
    }
    assert_eq!(o.get(Level::Nfl, 9).unwrap().games, 1);
    let back: Open = rmp_serde::from_slice(&rmp_serde::to_vec_named(&o).unwrap()).unwrap();
    assert_eq!(back.held(), (3, 0, 0));
    // Man 2 plays again: unpacked, his game added on top.
    o.add(Level::College, day.add(22), 7, 6, game(2));
    let s = o.get(Level::College, 2).unwrap();
    assert_eq!((s.games, s.snaps[0], s.played.clone()), (2, 80, vec![1, 6]));
    assert_eq!(o.held().1, 1);
    let _ = written;
}
