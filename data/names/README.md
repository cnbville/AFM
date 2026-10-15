# Name data

Used by `crates/afm-engine/src/world/names.rs` to give players and coaches names that fit their ethnicity and birth decade (trial 1 note 3; rebuilt 2026-10-14 because names repeated too often, world trial 3 finding 9).

**first_names.csv**: 38,669 male first names, every boy's name the SSA lists (5+ births in a year) from 1940 to 2017.
- `p_white`, `p_black`, `p_hispanic`, `p_asian`: the share of babies with that name in each group.
- `race_source` says where the shares come from:
  - `nyc`, `nyc+tzioumis`, `estimate`: the earlier 2,050-name file, unchanged. New York City baby names by ethnicity 2011-2021 (via `kjhealy/nycbabynames`), reweighted to national birth shares (white 54%, Hispanic 23%, Black 15%, Asian 6%) and blended with Tzioumis (2018). "DeShawn" is a hand estimate.
  - `tzioumis_reweighted`: Tzioumis (2018), first names by race from mortgage records (via the `predictrace` R package's data), reweighted from its sample (white 83%, Black 4.5%, Hispanic 7%, Asian 5.5%) to national birth shares.
  - `trigram_model`: no source has the name. Shares come from a letter-pattern model (2-4 letter pieces) trained on the names above, weighted by log births. Held-out error: the model's shares differ from the real ones by 0.33 on average (total variation), against 0.44 for a flat guess. These are rare names (most under 1,000 births ever). Proposed, not confirmed.
- `male_births_1940s` to `male_births_2010s`: US boys given the name per decade (SSA national baby names, via the `hadley/babynames` R package data, 1880-2017). Coaches born in the 1940s-60s now draw from their own decade.
- Every group keeps a small floor (0.5%).

**surnames.csv**: 167,407 US surnames (every name with 100+ people).
- `count`: people with the name in the 2000 Census (via the fivethirtyeight mirror). Names only in the 2010 Census get 100.
- `pct_*`: the 2010 Census race shares (via `predictrace`), falling back to 2000 where 2010 has none. Suppressed 2000 cells share what is left over.
- Written as people write them: O'Brien, De La Cruz, Del Rio, McNeal (the Census strips apostrophes and spaces).

Set in `names.rs` on top of the data (shares proposed, not confirmed):
- Suffixes: Jr. 4.5%, II 1.2%, III 0.8%, IV 0.1% of players; 1.6 times that among Black players.
- Hyphenated surnames: 0.8% (2.5% among Hispanic players).
- Goes by initials (CJ, AJ, TJ): 3%.
- Names written with an inner capital or apostrophe (DeAndre, Ja'Marr style) when a prefix (De, Da, Ja, La, Le, Ke, Ty, Tre, Ra, Sha, Ka) sits before a common name of 4+ letters: 70% inner capital, 15% apostrophe, 15% as listed. Black and Polynesian players only.

Known gaps:
- Polynesian names have no open source. A hand list of 48 surnames and 30 first names lives in `names.rs` (proposed, not confirmed).
- The official SSA, Census and NYC sites were blocked from here, so R package copies were used.
- The SSA file ends in 2017; boys born 2018 and later use the 2010s column.
