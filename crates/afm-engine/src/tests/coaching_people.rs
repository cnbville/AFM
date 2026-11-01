//! Update 3 lane 5: what playing does for a coach (C1, C2, C10) and the
//! tips' level ranks (SK35).

use crate::positions::Position;
use crate::world::data::Division;
use crate::world::former::{self, Played};

fn played(grade: u8, div: Division) -> Played {
    Played { pos: Position::LB, grade, div: div as u8, college: None, last: 2020, former: None }
}

#[test]
fn an_ex_pro_teaches_his_position_at_the_playing_roads_values() {
    // C6's locked strengths: star 19, Pro Bowler 18, starter 17, journeyman 16.
    for (g, v) in
        [(former::STAR, 19.0), (former::PRO_BOWLER, 18.0), (former::NFL_STARTER, 17.0), (former::JOURNEYMAN, 16.0)]
    {
        assert_eq!(played(g, Division::FBS).floor(), v);
    }
}

#[test]
fn a_former_college_player_gets_a_smaller_edge() {
    // The "former college player" road's 15, a point less for a backup or a
    // smaller college's starter, two for a smaller college's backup.
    assert_eq!(played(former::STARTER, Division::FBS).floor(), 15.0);
    assert_eq!(played(former::STARTER, Division::FCS).floor(), 15.0);
    assert_eq!(played(former::BACKUP, Division::FBS).floor(), 14.0);
    assert_eq!(played(former::STARTER, Division::D2).floor(), 14.0);
    assert_eq!(played(former::BACKUP, Division::D3).floor(), 13.0);
}

#[test]
fn a_name_from_playing_follows_how_good_he_was() {
    let order = [former::STAR, former::PRO_BOWLER, former::NFL_STARTER, former::JOURNEYMAN, former::STARTER];
    for k in 1..order.len() {
        assert!(played(order[k - 1], Division::FBS).fame() > played(order[k], Division::FBS).fame());
    }
    assert_eq!(played(former::STAR, Division::FBS).fame(), 17.0);
}

#[test]
fn most_coaches_played_college_football() {
    use crate::world::coach::CoachTier;
    // C2's research: 92.7% of FBS coaches since 2000.
    assert!((former::college_share(CoachTier::Fbs) - 0.927).abs() < 0.01);
    assert!(former::college_share(CoachTier::HighSchool) < former::college_share(CoachTier::Fcs));
}

#[test]
fn tips_rank_levels_from_fbs_down() {
    use crate::world::folk::tips::rank;
    assert!(rank(Division::FBS) > rank(Division::FCS));
    assert!(rank(Division::FCS) > rank(Division::D2));
    assert!(rank(Division::D2) > rank(Division::D3));
}
