-- log_players columns
select tbl, rows, substr(cols,1,2500) from catalog where tbl in ('log_players','log_snapshots','log_history') or tbl like 'log_players%'
-- rule changes by year
select year, rule_changes, nfl_games from log_history order by year
-- one play json
select substr(j.value, 1, 1500) from log_plays lp, json_each(more('log_plays', lp._more), '$.plays') j where lp.stream = 'nfl' and lp.season = 2026 limit 3
-- plays table columns
select tbl, rows, substr(cols,1,800) from catalog where tbl in ('log_plays')
