"""Each NFL position's top-five share of the cap, league year by league year
(realism audit item 3, 2026-11-15): the five biggest average-a-year deals in
force each season (signed that year or earlier, still running) over that
season's cap, the way OverTheCap lists a position.

Source: OverTheCap's contract records as nflverse publishes them
(`historical_contracts.parquet`, release tag `contracts`).
Usage: python3 nfl_top5_shares.py historical_contracts.parquet > ../nfl_top5_shares.csv
Needs pandas and pyarrow.

Thin years: OverTheCap's records thin out going back (fewer than 40 deals in
force at a position in a year before about 2006-2011). Such a year is the
average of itself and its neighbours. Before a position's first year with 10
deals in force, that first year holds. The `deals` column is the count in
force, so every thin year shows.
"""
import sys

import pandas as pd

CAP = {1994: 34.608, 1995: 37.1, 1996: 40.753, 1997: 41.454, 1998: 52.388, 1999: 57.288,
       2000: 62.172, 2001: 67.405, 2002: 71.101, 2003: 75.007, 2004: 80.582, 2005: 85.5,
       2006: 102.0, 2007: 109.0, 2008: 116.0, 2009: 123.0, 2010: 123.0, 2011: 120.375,
       2012: 120.6, 2013: 123.0, 2014: 133.0, 2015: 143.28, 2016: 155.27, 2017: 167.0,
       2018: 177.2, 2019: 188.2, 2020: 198.2, 2021: 182.5, 2022: 208.2, 2023: 224.8,
       2024: 255.4, 2025: 279.2, 2026: 301.2}
GROUP = {'QB': 'QB', 'RB': 'RB', 'WR': 'WR', 'TE': 'TE', 'LT': 'OT', 'RT': 'OT', 'LG': 'IOL',
         'RG': 'IOL', 'C': 'IOL', 'ED': 'EDGE', 'IDL': 'DT', 'LB': 'LB', 'CB': 'CB', 'S': 'S',
         'K': 'K', 'P': 'P', 'LS': 'LS'}
ORDER = ['QB', 'RB', 'WR', 'TE', 'OT', 'IOL', 'EDGE', 'DT', 'LB', 'CB', 'S', 'K', 'P', 'LS']
MIN_DEALS, FULL = 10, 40


def main(path):
    d = pd.read_parquet(path)
    d['pos'] = d['position'].map(GROUP)
    d = d.dropna(subset=['pos', 'year_signed', 'years', 'apy'])
    d = d[(d.year_signed >= 1990) & (d.years >= 1)]
    print('year,position,top5_share,deals')
    out = {}
    for p in ORDER:
        rows = d[d.pos == p]
        raw, n = {}, {}
        for y in CAP:
            v = rows[(rows.year_signed <= y) & (rows.year_signed + rows.years > y)].apy.sort_values(ascending=False)
            n[y] = len(v)
            if len(v) >= max(MIN_DEALS, 5):
                raw[y] = v.iloc[:5].mean() * 1e6 / (CAP[y] * 1e6)
        first = min(raw)
        for y in CAP:
            if y < first:
                raw[y] = raw[first]
        for y in CAP:
            if n[y] >= FULL:
                out[(y, p)] = (raw[y], n[y])
            else:
                near = [raw[k] for k in (y - 1, y, y + 1) if k in raw]
                out[(y, p)] = (sum(near) / len(near), n[y])
    for y in CAP:
        for p in ORDER:
            s, k = out[(y, p)]
            print(f'{y},{p},{s:.4f},{k}')


if __name__ == '__main__':
    main(sys.argv[1])
