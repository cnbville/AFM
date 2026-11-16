//! Ground lane G1a: the areas, each school's area, travel, schools a day,
//! the character words and the map's pins.

use crate::world::ground::areas::{self, Trip};
use crate::world::{World, WorldOptions, geo, high_school};

fn ga(key: &str) -> u16 {
    areas::find(&format!("GA-{key}")).unwrap()
}

#[test]
fn every_state_splits_into_named_areas() {
    let t = areas::table();
    assert!(t.len() >= 190, "{} areas", t.len());
    for s in 0..geo::STATE_CENTERS.len() {
        assert!(!areas::in_state(s).is_empty(), "{} has no area", geo::STATE_CENTERS[s].0);
    }
    let mut codes: Vec<String> = (0..t.len() as u16).map(areas::code).collect();
    codes.sort();
    codes.dedup();
    assert_eq!(codes.len(), t.len(), "an area code repeats");
    for (k, a) in t.iter().enumerate() {
        assert!(!a.name.is_empty() && !a.nickname.is_empty() && !a.codes.is_empty(), "{}", areas::code(k as u16));
        assert!(!a.counties.is_empty() && a.sq_miles > 0.0, "{}", areas::code(k as u16));
    }
    // The names the way coaches talk (GR1).
    assert_eq!(areas::area(areas::find("VA-757").unwrap()).nickname, "the 757");
    assert_eq!(areas::area(areas::find("DC-dc").unwrap()).nickname, "the DMV");
    assert_eq!(areas::area(areas::find("TX-rgv").unwrap()).nickname, "the Valley");
}

#[test]
fn georgia_splits_as_the_mock_up() {
    let ga_state = geo::state_index("GA").unwrap();
    let keys: Vec<&str> = areas::in_state(ga_state).iter().map(|&i| areas::area(i).key).collect();
    assert_eq!(keys, ["south", "404", "gwin", "cobb", "north", "athens", "aug", "macon", "col", "sav", "229", "se"]);
    let counties: usize = areas::in_state(ga_state).iter().map(|&i| areas::area(i).counties.len()).sum();
    // Georgia's 159 counties and the Valley's three in Alabama.
    assert_eq!(counties, 162);
    assert_eq!(areas::area(ga("404")).counties, ["DeKalb", "Fulton"]);
    let al = geo::state_index("AL").unwrap();
    let phenix = high_school::town_place("AL", "Phenix City").unwrap_or((32.471, -85.0));
    assert_eq!(areas::area_for(al, "Russell", phenix), ga("col"));
    assert_eq!(areas::area_for(ga_state, "Muscogee", (32.46, -84.99)), ga("col"));
    assert_eq!(areas::area_for(ga_state, "Bibb", (32.84, -83.63)), ga("macon"));
    assert_eq!(areas::area_for(ga_state, "Lowndes", (30.83, -83.28)), ga("229"));
}

#[test]
fn every_real_town_has_an_area_in_its_state() {
    for (code, _, _) in geo::STATE_CENTERS {
        let s = geo::state_index(code).unwrap();
        for t in high_school::state_towns(code) {
            let a = areas::area_for(s, &t.county, (t.lat as f64, t.lon as f64));
            let home = areas::area(a).state;
            let valley = code == "AL" && a == ga("col");
            assert!(home == s || valley, "{code} {} ({}) in {}", t.town, t.county, areas::code(a));
            // A known county is in its own area, never a nearest guess.
            if !t.county.is_empty() && !valley {
                assert!(
                    areas::area(a).counties.contains(&t.county.as_str())
                        || !areas::table().iter().any(|x| x.state == s && x.counties.contains(&t.county.as_str())),
                    "{code} {}",
                    t.county
                );
            }
        }
    }
}

#[test]
fn every_school_has_its_area_built_and_read() {
    let w = World::empty(WorldOptions { scale: 0.01, history_years: 0, start_year: 2037, ..Default::default() });
    let al = geo::state_index("AL").unwrap();
    for s in &w.high_schools {
        let a = areas::area(s.ground_area);
        assert!(a.state == s.state || (s.state == al && s.ground_area == ga("col")), "{} {}", s.town, s.county);
        if !s.county.is_empty() {
            let named = a.counties.iter().any(|c| *c == s.county || c.ends_with(&format!(":{}", s.county)));
            let alias = !areas::table().iter().any(|x| x.counties.contains(&s.county.as_str()));
            assert!(named || alias, "{} {} in {}", s.town, s.county, areas::code(s.ground_area));
        }
    }
    let by = areas::schools_by_area(&w);
    assert_eq!(by.iter().map(Vec::len).sum::<usize>(), w.high_schools.len());
    // The field isn't saved: a world read back fills it from the table.
    let f = std::env::temp_dir().join(format!("afm-ground-areas-{}.afmworld", std::process::id()));
    crate::save::write_world(&w, &f, "test", "", None).unwrap();
    let r = crate::save::read_world(&f).unwrap();
    let _ = std::fs::remove_file(&f);
    assert!(w.high_schools.iter().zip(&r.high_schools).all(|(a, b)| a.ground_area == b.ground_area));
}

#[test]
fn travel_is_real() {
    let atlanta = (33.753, -84.386);
    // Atlanta to Columbus: about 90 straight miles, a two-hour drive.
    let t = areas::trip(atlanta, (32.461, -84.988));
    assert!(matches!(t, Trip::Drive { .. }) && (1.8..2.6).contains(&t.hours()), "{t:?}");
    assert_eq!(t.words(), "a 2-hour drive");
    // Atlanta to Miami is a flight, about four hours door to door.
    let t = areas::trip(atlanta, (25.774, -80.194));
    assert!(t.flight() && (3.5..4.5).contains(&t.hours()), "{t:?}");
    // Across town.
    assert!(areas::trip(atlanta, (33.8, -84.4)).hours() < 0.3);
    // Five hours on the road is the line: Birmingham drives, Charlotte
    // (226 straight miles, 282 by road) flies.
    assert!(!areas::trip(atlanta, (33.519, -86.810)).flight());
    assert!(areas::trip(atlanta, (35.227, -80.843)).flight());
}

#[test]
fn schools_a_day_by_how_close_they_sit() {
    assert_eq!(areas::schools_a_day(ga("404")), 5);
    assert!(areas::schools_a_day(ga("gwin")) >= 3);
    assert!(areas::schools_a_day(ga("229")) <= 2);
    assert!(areas::schools_a_day(ga("se")) <= 2);
    assert_eq!(areas::schools_a_day(areas::find("NY-nyc").unwrap()), 5);
    assert_eq!(areas::schools_a_day(areas::find("WY-wy").unwrap()), 1);
    for k in 0..areas::table().len() as u16 {
        assert!((1..=5).contains(&areas::schools_a_day(k)));
    }
}

#[test]
fn characters_and_pins_read_the_world() {
    let w = crate::tests::legacy::seat("college");
    let c = areas::characters(&w);
    assert_eq!(c.len(), areas::table().len());
    for (k, x) in c.iter().enumerate() {
        assert!(x.public.contains("in the biggest class"), "{}", areas::code(k as u16));
        assert!(x.words.len() <= 3);
    }
    let p = w.seat.and_then(|s| s.program);
    let pins = areas::pins(&w, p);
    assert_eq!(pins.len(), areas::table().len());
    for x in &pins {
        assert!(x.legends.len() <= areas::LEGENDS && x.pipelines.len() <= areas::PIPELINES);
        assert!(x.pipelines.windows(2).all(|p| p[0].strength >= p[1].strength));
    }
}
