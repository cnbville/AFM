"""How much does the head coach move offense, once the quarterback is held fixed? (research area 1)

Reads nflverse play-by-play 2006-2025 (pbp/play_by_play_YEAR.parquet). For every regular-season
team game it takes offensive EPA per play (runs and dropbacks, no kneels or spikes), the main QB
(most dropbacks) and the team's head coach. Then it fits
    EPA/play = season + QB + head coach + home
by least squares, so a coach is judged only against what his QBs did elsewhere.
Noise check: the same fit on odd and even games; the correlation of the two coach estimates is the
share of the spread that is real, and the true spread = raw spread x sqrt(that share).

Usage: python3 coach_effect.py DIR [offense|defense] > coach_effect_MODE.json
Limits: in the NFL the head coach is often but not always the play caller; the "coach" effect also
holds everything that moves with him (his coordinators, scheme, the roster his GM builds).
"""
import json, sys
import numpy as np
import pandas as pd
from scipy.sparse import csr_matrix
from scipy.sparse.linalg import lsqr

D = sys.argv[1]
MODE = sys.argv[2] if len(sys.argv) > 2 else "offense"  # "defense": coach of the defending team, opponent QB held fixed
rows = []
for y in range(2006, 2026):
    p = pd.read_parquet(f"{D}/play_by_play_{y}.parquet",
                        columns=["game_id", "season", "season_type", "posteam", "home_team", "home_coach", "away_coach",
                                 "defteam", "play_type", "qb_dropback", "qb_kneel", "qb_spike", "passer_player_id", "epa"])
    p = p[(p.season_type == "REG") & p.play_type.isin(["pass", "run"]) & (p.qb_kneel == 0) & (p.qb_spike == 0) & p.epa.notna()]
    side = "posteam" if MODE == "offense" else "defteam"
    p["coach"] = np.where(p[side] == p.home_team, p.home_coach, p.away_coach)
    p["home"] = (p[side] == p.home_team).astype(int)
    g = p.groupby(["game_id", "posteam"]).agg(season=("season", "first"), coach=("coach", "first"), home=("home", "first"),
                                              epa=("epa", "mean"), n=("epa", "size"))
    qb = p[p.qb_dropback == 1].groupby(["game_id", "posteam"]).passer_player_id.agg(lambda x: x.value_counts().index[0] if len(x.dropna()) else None)
    g = g.join(qb.rename("qb")).reset_index()
    rows.append(g)
df = pd.concat(rows).dropna(subset=["qb", "coach"])
df["gnum"] = df.groupby(["season", "posteam"]).cumcount()


def fit(d):
    cols = {}
    def idx(k):
        if k not in cols:
            cols[k] = len(cols)
        return cols[k]
    r_, c_, v_ = [], [], []
    for i, row in enumerate(d.itertuples()):
        for k in (("s", row.season), ("q", row.qb), ("c", row.coach)):
            r_.append(i); c_.append(idx(k)); v_.append(1.0)
        r_.append(i); c_.append(idx(("h",))); v_.append(float(row.home))
    X = csr_matrix((v_, (r_, c_)), shape=(len(d), len(cols)))
    w = np.sqrt(d.n.values)
    sol = lsqr(X.multiply(w[:, None]).tocsr(), d.epa.values * w, damp=0.05)[0]
    return {k[1]: sol[j] for k, j in cols.items() if k[0] == "c"}, sol[cols[("h",)]]


full, home = fit(df)
odd, _ = fit(df[df.gnum % 2 == 1])
even, _ = fit(df[df.gnum % 2 == 0])
games = df.groupby("coach").size()
keep = [c for c in full if games.get(c, 0) >= 48 and c in odd and c in even]
fe = pd.Series({c: full[c] for c in keep})
fe -= fe.mean()
rel = float(np.corrcoef([odd[c] for c in keep], [even[c] for c in keep])[0, 1])
plays = float(df.n.mean())
true_sd = float(fe.std() * np.sqrt(max(rel, 0)))
out = {"source": "nflverse play-by-play 2006-2025, regular season", "team_games": int(len(df)),
       "coaches_with_48_plus_games": len(keep), "offensive_plays_per_team_game": round(plays, 1),
       "raw_sd_epa_per_play": round(float(fe.std()), 4), "split_half_reliability": round(rel, 3),
       "true_sd_epa_per_play": round(true_sd, 4),
       "true_sd_points_per_game": round(true_sd * plays, 2),
       "p10_to_p90_points_per_game_if_normal": round(2.563 * true_sd * plays, 2),
       "home_edge_epa_per_play": round(float(home), 4),
       "top5_raw": {k: round(v, 3) for k, v in fe.sort_values(ascending=False).head(5).items()},
       "bottom5_raw": {k: round(v, 3) for k, v in fe.sort_values().head(5).items()},
       "mode": MODE, "note": "Offense: the coach of the team with the ball. Defense: the coach of the defending team (lower EPA allowed is better, so the sign flips). Effects are relative to the average coach. Top and bottom lists are raw estimates (noisier than the true spread)."}
json.dump(out, sys.stdout, indent=1)
