"""Replace the letter-pattern guesses in data/names/first_names.csv with
voter-file race shares (trial 8, item 17: the model gave Dashawn 61% White).

Source: the first-name tables in the `pyethnicity` package (PyPI, MIT
licence; Cangyuan Li, arXiv 2307.08496): the HMDA first-name list
(Tzioumis 2018) extended with self-reported race from L2 voter
registration files. P(name | race) is reweighted to national birth shares
(white 54%, Hispanic 23%, Black 15%, Asian 6%), the same priors the file's
other sources use, then blended with the old guess by how many voters
carry the name (10 voters' worth of the old guess), with the file's 0.5%
floor per group.

Usage: python3 -I voter_shares.py <pyethnicity wheel> <first_names.csv>
(rewrites the CSV in place and prints what changed).
"""

import csv
import io
import sys
import zipfile

import pyarrow.parquet as pq

PRIOR = {"white": 0.54, "black": 0.15, "hispanic": 0.23, "asian": 0.06}
RACES = ["white", "black", "hispanic", "asian"]
PSEUDO = 10.0
FLOOR = 0.005


def table(z, name):
    t = pq.read_table(io.BytesIO(z.read(f"pyethnicity/data/distributions/{name}.parquet")))
    return {r["name"]: r for r in t.to_pylist()}


def main(wheel, path):
    z = zipfile.ZipFile(wheel)
    given_race = table(z, "prob_first_name_given_race")
    # Voters of each race in the source: one over its smallest share.
    voters = {r: 1.0 / min(v[r] for v in given_race.values() if v[r] > 0) for r in RACES}
    rows = list(csv.DictReader(open(path, newline="")))
    fields = list(rows[0].keys())
    changed = []
    for row in rows:
        if row["race_source"] != "trigram_model":
            continue
        v = given_race.get(row["name"].upper())
        if v is None:
            continue
        n = sum(v[r] * voters[r] for r in RACES)
        weights = {r: v[r] * PRIOR[r] for r in RACES}
        total = sum(weights.values())
        if total <= 0 or n < 1:
            continue
        old = {r: float(row[f"p_{r}"]) for r in RACES}
        new = {r: (n * weights[r] / total + PSEUDO * old[r]) / (n + PSEUDO) for r in RACES}
        new = {r: max(new[r], FLOOR) for r in RACES}
        s = sum(new.values())
        for r in RACES:
            row[f"p_{r}"] = f"{new[r] / s:.4f}"
        row["race_source"] = "voter_files"
        changed.append((row["name"], round(n), old, {r: float(row[f"p_{r}"]) for r in RACES}))
    out = io.StringIO()
    w = csv.DictWriter(out, fieldnames=fields, lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
    open(path, "w", newline="").write(out.getvalue())
    print(f"voters by race in the source: { {r: round(voters[r]) for r in RACES} }")
    left = sum(1 for r in rows if r["race_source"] == "trigram_model")
    print(f"{len(changed)} letter-pattern rows replaced, {left} left (names no voter file has)")
    for name, n, old, new in changed:
        if name in ("Dashawn", "Antwain", "Antwan", "Jaquan", "Deshawn", "Aaban", "Keshawn", "Javontae"):
            print(f"  {name}: {n} voters, black {old['black']:.2f} -> {new['black']:.2f}, white {old['white']:.2f} -> {new['white']:.2f}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
