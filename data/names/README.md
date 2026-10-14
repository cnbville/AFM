# Name data

Used by `crates/afm-engine/src/world/names.rs` to give players and coaches names that fit their ethnicity, state and birth decade (trial 1 note 3).

**first_names.csv**: 2,050 male first names.
- `p_white`, `p_black`, `p_hispanic`, `p_asian`: the share of babies with that name in each group.
- Sources: New York City baby names by ethnicity, 2011-2021 (NYC open data, via the `kjhealy/nycbabynames` R package), and Tzioumis (2018), first names by race from mortgage records (via the `predictrace` R package).
- The NYC shares are reweighted to national birth shares (white 54%, Hispanic 23%, Black 15%, Asian 6%), then blended with Tzioumis (`race_source` says which). Every group keeps a small floor (0.5%).
- `male_births_1970s` to `male_births_2010s`: US boys given the name per decade (SSA national baby names, via the `hadley/babynames` R package, 1880-2017).
- The list covers 96% of boys born in the 1970s and 81% born in the 2010s.

**surnames.csv**: the 25,000 most common US surnames (Census 2000, via the fivethirtyeight mirror), with count and share per group (`pct_api` is Asian and Pacific Islander).

Known gaps:
- Polynesian names have no open source. A hand list of 48 surnames and 30 first names lives in `names.rs` (proposed, not confirmed).
- Tzioumis has few young adults, so names like Jalen come mainly from the NYC data.
- "DeShawn" is in neither source. Added by hand (2026-10-14): 85% Black, birth counts estimated from its SSA peak in the 1990s (`race_source` = `estimate`; proposed, not confirmed).
- Tzioumis comes from mortgage records, which are mostly white. The 1,307 names only it has are reweighted (2026-10-14) from its approximate sample (white 83%, Black 4.5%, Hispanic 7%, Asian 5.5%) to national birth shares (`race_source` = `tzioumis_reweighted`; the sample shares are approximate, proposed, not confirmed). "Trey" moves from 3% to 15% Black.
- The official SSA, Census and NYC sites were blocked from here, so the R package copies were used.
