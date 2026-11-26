-- how many checks carry a year series
select count(*), sum(years_sql is not null and years_sql <> '') from checks
-- schemas
select 'nil_book', group_concat(name, ', ') from pragma_table_info('nil_book')
select 'finance__college__years', group_concat(name, ', ') from pragma_table_info('finance__college__years')
select 'finance__nfl__years', group_concat(name, ', ') from pragma_table_info('finance__nfl__years')
select 'people', group_concat(name, ', ') from pragma_table_info('people')
select 'portal_market__medians', group_concat(name, ', ') from pragma_table_info('portal_market__medians')
select * from nil_book limit 4
select * from portal_market__medians limit 6
-- rating drift: mean overall and share at 90+ by level and playing year
select year, level, count(*) n, round(avg(overall),2) mean_overall, round(100.0*avg(overall>=90),2) pct90, round(100.0*avg(overall>=80),2) pct80, round(avg(level_rating),2) mean_lr from (select p.class_of + h.age - 18 as year, h.level, h.overall, h.level_rating from people__player__history h join people p on p._row = h._parent union all select a.class_of + h.age - 18, h.level, h.overall, h.level_rating from alumni__player__history h join alumni a on a._row = h._parent) where year between 2018 and 2035 and level in ('NFL','FBS') group by level, year order by level, year
-- rating drift by age, NFL: the 2022-2025 seasons against the 2032-2035 seasons
select age, period, count(*) n, round(avg(overall),2) mean_overall, round(avg(overall - overall_start),2) mean_gain from (select h.age, case when p.class_of + h.age - 18 between 2022 and 2025 then 'a 2022-25' when p.class_of + h.age - 18 between 2032 and 2035 then 'b 2032-35' end as period, h.overall, h.overall_start, h.level from people__player__history h join people p on p._row = h._parent union all select h.age, case when a.class_of + h.age - 18 between 2022 and 2025 then 'a 2022-25' when a.class_of + h.age - 18 between 2032 and 2035 then 'b 2032-35' end, h.overall, h.overall_start, h.level from alumni__player__history h join alumni a on a._row = h._parent) where period is not null and level = 'NFL' and age between 21 and 36 group by age, period order by age, period
-- panels: league money, club money, ratings by age
select id, cols, substr(rows,1,7000) as rows from panels where id in ('N30.league','N30.per_club','N30.ticket','N9.flow','P12.gain_by_age','P8.overall_by_level','P8.position_overall')
-- every check's own year series
years %
