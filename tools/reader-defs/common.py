from lib import *

# Shared SQL pieces.
CS = real("college_scores.json")
CH = real("college_history.json")
CP = real("college_plays.json")
CPL = real("college_players.json")
CGT = real("college_game_targets.json")
NGT = real("nfl_game_targets.json")
NH = real("nfl_history.json")
NC = real("nfl_careers.json")
ET = real("engine_targets.json")
GT = real("garbage_time.json")
NS = real("nfl_situations.json")
DI = real("development_injuries.json")
PV = real("position_value.json")
T3 = real("trial3_baselines.json")
NR = real("nfl_rules.json")
MC = real("memory_checks.json")
CPE = real("college_people.json")

# A college program's name.
def prog(alias="p"):
    return f"COALESCE({alias}.real_name, {alias}.fictional_name)"

# The real data's key for a division.
REALDIV = "(CASE {d} WHEN 'FBS' THEN 'fbs' WHEN 'FCS' THEN 'fcs' WHEN 'D2' THEN 'ii' WHEN 'D3' THEN 'iii' END)"
def realdiv(d): return REALDIV.format(d=d)

# Engine games (no stand-in), one row per game.
ENGINE = "COALESCE(g.standin, 0) = 0 AND g.home_pts IS NOT NULL"
REG = "COALESCE(g.kind, 'regular') = 'regular'"

# Division order, highest first.
DIVORD = "(CASE {d} WHEN 'FBS' THEN 1 WHEN 'FCS' THEN 2 WHEN 'D2' THEN 3 WHEN 'NAIA' THEN 4 WHEN 'D3' THEN 5 WHEN 'JUCO' THEN 6 ELSE 7 END)"
def divord(d): return DIVORD.format(d=d)

# Team games in the college log: year, division, opponent division, points for and against, home or away, game row.
COLLEGE_TG = f"""tg AS (
     SELECT g._row AS gid, g.season AS year, json_extract(g.divisions, '$[0]') AS div, json_extract(g.divisions, '$[1]') AS odiv,
            g.home_pts AS pf, g.away_pts AS pa, 1 AS home, COALESCE(g.neutral, 0) AS neutral, COALESCE(g.ot, 0) AS ot, g.kind AS kind, g.home AS team
       FROM log_games g WHERE g.stream = 'college' AND {ENGINE}
     UNION ALL
     SELECT g._row, g.season, json_extract(g.divisions, '$[1]'), json_extract(g.divisions, '$[0]'), g.away_pts, g.home_pts, 0, COALESCE(g.neutral, 0), COALESCE(g.ot, 0), g.kind, g.away
       FROM log_games g WHERE g.stream = 'college' AND {ENGINE})"""

# NFL club name in a season (by era), from the engine's franchise table, today's name otherwise.
def club(t, y):
    return (f"COALESCE((SELECT CASE WHEN (SELECT fictional_league FROM options) = 1 THEN f.fictional ELSE f.name END FROM engine_franchises f "
            f"WHERE f.team = {t} AND {y} BETWEEN f.\"from\" AND f.\"to\"), (SELECT name FROM nfl WHERE _row = {t}))")

def cname(t):
    """Today's club name."""
    return f"(SELECT name FROM nfl WHERE _row = {t})"

# A person's name by Player::id, wherever he is kept (active, alumni, or the world log's people rows).
def pname(idexpr):
    return (f"COALESCE((SELECT name FROM people WHERE player__id = {idexpr}), (SELECT name FROM alumni WHERE player__id = {idexpr}), "
            f"(SELECT name FROM log_people WHERE id = {idexpr} LIMIT 1), '#' || {idexpr})")

for c_ in ["neutral", "ot", "kind", "standin", "line", "attendance", "tags", "loud", "huge_for", "rivalry", "conference_game", "divisions",
           "weather__snow", "weather__rain", "weather__temperature", "weather__wind", "weather__indoor", "weather__delay", "weather__lightning",
           "conditions__temperature", "conditions__wind", "conditions__rain", "conditions__snow", "conditions__turf", "conditions__altitude", "conditions__crowd",
           "officiating__booth_reviews", "officiating__challenges", "officiating__challenges_won", "officiating__overturned", "officiating__wrong",
           "officiating__wrong_left_standing", "officiating__close_calls", "officiating__assists", "toss__choice", "toss__winner",
           "week", "name", "round", "tv", "kickoff", "why", "names", "league", "state", "senior_day", "homecoming", "site__state", "snaps",
           "paid__dollars", "paid__kind", "cross", "neutral"]:
    col("log_games", c_)
