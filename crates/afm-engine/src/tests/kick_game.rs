//! The kicking game (engine gaps lane EG3): the staff's field goal estimate
//! moves with the distance and the weather as the kick does, and the coach's
//! punt value reads where a real punt from there leaves the ball.

use crate::game::Conditions;
use crate::game::calling::punt_start;
use crate::game::flow::fg_make_in;
use crate::game::kicking::{fg_all_out, fg_reach};
use crate::game::synthetic::synthetic_league;
use crate::player::Level;
use crate::positions::Position;

fn windy(wind: f64) -> Conditions {
    Conditions { wind, ..Conditions::default() }
}

/// The estimate falls with the distance, and a 15 mph wind in his face
/// costs him reach and makes against the same wind at his back and against
/// a calm day (engine gaps items 11 and 14).
#[test]
fn the_field_goal_estimate_reads_distance_and_wind() {
    for level in [Level::HS, Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 2, 7);
        let k = teams[0].players.iter().find(|p| p.position == Position::K).expect("a kicker");
        let calm = Conditions::default();
        let make: Vec<f64> = [25.0, 40.0, 50.0, 58.0].iter().map(|&d| fg_make_in(k, d, &calm, 0.0)).collect();
        assert!(make.windows(2).all(|w| w[0] > w[1]), "{level:?}: estimate by distance {make:?}");
        let blow = windy(15.0);
        let into = fg_make_in(k, 50.0, &blow, -1.0);
        let behind = fg_make_in(k, 50.0, &blow, 1.0);
        assert!(
            into < behind && into < make[2],
            "{level:?}: 50 yards into {into:.3}, calm {:.3}, behind {behind:.3}",
            make[2]
        );
        assert!(fg_reach(k, &blow, -1.0) < fg_reach(k, &calm, 0.0), "{level:?}: a headwind shortens his reach");
    }
}

/// The coach's punt value starts the other team where a punt from there
/// really leaves them (nflverse 2024-25): a 41-yard net from deep in his
/// end, and about 88 yards out once he punts from their side of midfield,
/// not a touchback.
#[test]
fn the_punt_value_starts_where_punts_land() {
    assert_eq!(punt_start(40.0), 88.0);
    assert_eq!(punt_start(53.0), 88.0);
    assert_eq!(punt_start(70.0), 71.0);
    assert!(punt_start(80.0) < punt_start(60.0));
}

/// The rush on a field goal sells out more often the longer the kick, as
/// real blocks rise with the distance; it stays a minority call.
#[test]
fn the_field_goal_rush_sells_out_more_on_long_kicks() {
    assert!(fg_all_out(25.0) < fg_all_out(40.0) && fg_all_out(40.0) < fg_all_out(55.0));
    assert!(fg_all_out(70.0) <= 0.7 && fg_all_out(10.0) >= 0.03);
}
