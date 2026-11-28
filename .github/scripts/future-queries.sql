-- log tables
select name from sqlite_master where type = 'table' and name like 'log_%' order by 1
-- log_players columns
select 'log_players', group_concat(name, ', ') from pragma_table_info('log_players')
-- log_games columns
select 'log_games', group_concat(name, ', ') from pragma_table_info('log_games')
-- log_games__box columns
select 'log_games__box', group_concat(name, ', ') from pragma_table_info('log_games__box')
-- log_games child tables
select name from sqlite_master where type = 'table' and name like 'log_games__%' order by 1
-- log_clubs columns
select 'log_clubs', group_concat(name, ', ') from pragma_table_info('log_clubs')
-- people__pro__seasons columns
select 'people__pro__seasons', group_concat(name, ', ') from pragma_table_info('people__pro__seasons')
-- people__player__history columns
select 'people__player__history', group_concat(name, ', ') from pragma_table_info('people__player__history')
-- one log_players row
rsql select season, level, team, overall, person__player__position, substr(more('log_players', _more), 1, 1800) m from log_players where level = 'NFL' and season = 2030 limit 1
