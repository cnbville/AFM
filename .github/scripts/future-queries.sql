-- catalog: tables of interest
select tbl, rows, substr(cols,1,1500) from catalog where tbl in ('log_games','log_games__box','log_team_situations','log_coaches','log_clubs','log_world','coaches') or tbl like 'log_coaches%' or tbl like '%trend%' or tbl like 'coaches%' or tbl like 'front_office%' or tbl like 'staff%'
-- all tables with coach or trend or play in their name
select tbl, rows from catalog where tbl like '%coach%' or tbl like '%trend%' or tbl like '%family%' or tbl like '%playbook%' order by tbl limit 120
-- log_team_situations keys sample
select key, count(*) from log_team_situations where season = 2030 group by key order by 2 desc limit 80
-- teams in situations by season
select season, count(distinct team) teams, count(*) rows from log_team_situations group by season order by season
-- stream names in games
select stream, min(season), max(season), count(*) from log_games group by stream
