-- play json sample
rsql select substr(j.value, 1, 1800) from log_plays lp, json_each(more('log_plays', lp._more), '$.plays') j where lp.stream = 'nfl' and lp.season = 2026 limit 3
-- game json keys
rsql select substr(more('log_plays', lp._more), 1, 1200) from log_plays lp where lp.stream = 'nfl' and lp.season = 2026 limit 1
