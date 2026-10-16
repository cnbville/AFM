"""Writes data/real-world/nfl_schedule_2026.csv from nflverse's games.csv.

Usage: python3 nfl_schedule_2026.py path/to/games.csv > ../nfl_schedule_2026.csv
Source: https://github.com/nflverse/nfldata (data/games.csv).
"""
import csv
import sys

TEAMS = {
    "ARI": "Arizona Cardinals", "ATL": "Atlanta Falcons", "BAL": "Baltimore Ravens", "BUF": "Buffalo Bills",
    "CAR": "Carolina Panthers", "CHI": "Chicago Bears", "CIN": "Cincinnati Bengals", "CLE": "Cleveland Browns",
    "DAL": "Dallas Cowboys", "DEN": "Denver Broncos", "DET": "Detroit Lions", "GB": "Green Bay Packers",
    "HOU": "Houston Texans", "IND": "Indianapolis Colts", "JAX": "Jacksonville Jaguars", "KC": "Kansas City Chiefs",
    "LA": "Los Angeles Rams", "LAC": "Los Angeles Chargers", "LV": "Las Vegas Raiders", "MIA": "Miami Dolphins",
    "MIN": "Minnesota Vikings", "NE": "New England Patriots", "NO": "New Orleans Saints", "NYG": "New York Giants",
    "NYJ": "New York Jets", "PHI": "Philadelphia Eagles", "PIT": "Pittsburgh Steelers", "SEA": "Seattle Seahawks",
    "SF": "San Francisco 49ers", "TB": "Tampa Bay Buccaneers", "TEN": "Tennessee Titans", "WAS": "Washington Commanders",
}

out = csv.writer(sys.stdout, lineterminator="\n")
out.writerow(["week", "date", "time_et", "away", "home", "neutral_site"])
for r in csv.DictReader(open(sys.argv[1], newline="")):
    if r["season"] != "2026" or r["game_type"] != "REG":
        continue
    site = r["stadium"] if r["location"] != "Home" else ""
    out.writerow([r["week"], r["gameday"], r["gametime"], TEAMS[r["away_team"]], TEAMS[r["home_team"]], site])
