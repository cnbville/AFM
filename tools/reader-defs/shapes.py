"""Each rule's failure rows as the book wants them: what, id, year, says (odd.rs reads four columns)."""
import lib
SHAPES = {
 "G34.score_from_scoring": ("'game'", "game", "year", "'final ' || final || ', the last scoring play left ' || last_scoring"),
 "G34.box_points": ("'game'", "game", "year", "side || ': box points ' || points || ' against the final ' || final"),
 "G34.players_attempts": ("'game'", "game", "year", "side || ' (' || team || '): ' || players"),
 "G34.drives_plays": ("'game'", "game", "year", "side || ': drives ' || drive_points || ' plus defensive touchdowns ' || defense_tds || ' against ' || points"),
 "N2.one_coach_one_club": ("'coach'", "coach", "year", "name || ' coached ' || clubs || ' clubs'"),
 "N3.byes_in_window": ("'club'", "team", "(SELECT year FROM nfl_season)", "'a bye in week ' || week"),
 "N4.one_kicker": ("'club'", "team", "NULL", "club || ': ' || k || ' kickers, ' || p || ' punters, ' || ls || ' long snappers'"),
 "N13.four_per_club": ("'club'", "team", "year", "picks || ' compensatory picks'"),
 "N26.real_years": ("'league'", "name", "year", "name || ' played in ' || year || ', real years ' || \"from\" || '-' || \"to\""),
 "N29.mentors": ("'person'", "a", "NULL", "'a mentor tie with ' || b || ' at ' || strength"),
 "N30.no_blackouts_after_2014": ("'club'", "club", "year", "club || ': ' || blackouts || ' blackouts'"),
 "N34.draft_weekday": ("'season'", "season", "season", "'the draft opens ' || date || ', a ' || weekday"),
 "N34.super_bowl_window": ("'season'", "season", "season", "'the Super Bowl on ' || date"),
 "N34.league_year_window": ("'season'", "season", "season", "'the league year opens ' || date"),
 "N34.thanksgiving": ("'season'", "season", "season", "'Thanksgiving on ' || date"),
 "N34.kickoff_thursday": ("'season'", "season", "season", "'kickoff ' || kickoff || ', Labor Day ' || labor_day"),
 "N35.real_history": ("'season'", "rule", "season", "rule || ': world ' || COALESCE(world, 'none') || ', real ' || real"),
 "N36.nfl_dates_once": ("'rule'", "rule", "year", "rule || ' posted ' || posts || ' times'"),
 "N37.domes_dry": ("'game'", "home", "season", "home || ' under a dome on ' || date || ': snow ' || COALESCE(snow, 0) || ', rain ' || COALESCE(rain, 0) || ', wind ' || COALESCE(wind, 0)"),
 "N38.class_fits_birth": ("'person'", "id", "class_of", "name || ', born ' || born || ', class of ' || class_of"),
 "N39.nfl_games_real": ("'season'", "year", "year", "world || ' NFL games, real ' || real"),
 "N42.sec_fines": ("'program'", "home", "year", "home || ' fined ' || fine || ', real tiers ' || \"real tiers\""),
 "N43.hard_knocks_once": ("'season'", "year", "year", "seasons || ' Hard Knocks seasons'"),
}
def apply():
    for r in lib.R:
        s = SHAPES.get(r["id"])
        assert s, "no shape for " + r["id"]
        if r.get("shaped"): continue
        r["failures"] = f"SELECT {s[0]} AS what, {s[1]} AS id, {s[2]} AS year, {s[3]} AS says FROM (\n{r['failures']})"
        r["shaped"] = True
