-- columns
select 'people__player__history', group_concat(name, ', ') from pragma_table_info('people__player__history')
select 'alumni__player__history', group_concat(name, ', ') from pragma_table_info('alumni__player__history')
select 'people__pro__seasons', group_concat(name, ', ') from pragma_table_info('people__pro__seasons')
select 'alumni cols', group_concat(name, ', ') from pragma_table_info('alumni') where name like '%position%' or name like 'class_of' or name like '%body__%' or name like 'pro__%'
select 'people cols pro', group_concat(name, ', ') from pragma_table_info('people') where name like 'pro__%' or name like 'class_of' or name like 'player__spot%'
-- NFL starters (8+ starts) overall by position and year
select year, pos, count(*) n, round(avg(overall),2) mean_overall from (select s.year, p.player__position pos, h.overall from people__pro__seasons s join people p on p._row = s._parent join people__player__history h on h._parent = s._parent and h.age = s.year - p.class_of + 18 where s.starts >= 8 and h.level = 'NFL' union all select s.year, a.player__position, h.overall from alumni__pro__seasons s join alumni a on a._row = s._parent join alumni__player__history h on h._parent = s._parent and h.age = s.year - a.class_of + 18 where s.starts >= 8 and h.level = 'NFL') where year between 2024 and 2035 group by year, pos order by pos, year
