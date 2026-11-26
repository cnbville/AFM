-- panel ids and titles, except C and G
select id || ' | ' || title || ' | ' || kind from panels where id not like 'C%' and id not like 'G%' order by id
-- tables except calendar and refs
select name from sqlite_master where type='table' and name not like 'calendar%' and name not like '%refs' order by name
-- schemas
select 'finance__league', group_concat(name || ':' || type, ', ') from pragma_table_info('finance__league')
select 'market__years', group_concat(name || ':' || type, ', ') from pragma_table_info('market__years')
select 'programs__seasons', group_concat(name || ':' || type, ', ') from pragma_table_info('programs__seasons')
select 'programs', group_concat(name || ':' || type, ', ') from pragma_table_info('programs')
select 'people__player__history', group_concat(name || ':' || type, ', ') from pragma_table_info('people__player__history')
select 'alumni__pro__seasons', group_concat(name || ':' || type, ', ') from pragma_table_info('alumni__pro__seasons')
select 'checks', group_concat(name || ':' || type, ', ') from pragma_table_info('checks')
-- money tables whole
select * from finance__league
select * from market__years
-- checks on NIL, revenue, cap, ratings
select * from checks where id like 'C14%' or id like 'C41%' or id like 'N30%' or id like 'N31%' or id like 'N35%' or id like 'N32%' or id like 'C23%' or id like 'P8.%' or id like 'P14%'
-- year panels
select id, cols, substr(rows,1,9000) as rows from panels where id in ('G3.by_year','G2.by_year','G1.points_by_year','C6.strength','C23.prestige_years','C1.levels_by_year','G18.drift','C2.spread','P8.level_ratings','P14.kept_starting','C5.rolling_most','C5.rolling_distinct','G5.champions','G7.yearly')
