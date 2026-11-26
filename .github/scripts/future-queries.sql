-- levels in history rows
select level, count(*) from people__player__history group by level
select playing_time, count(*) from people__player__history where level like 'FBS%' group by playing_time
-- FBS roster rows by season and role (people and alumni)
select year, playing_time, count(*) n from (select p.class_of - 18 + h.age year, h.playing_time from people__player__history h join people p on p._row = h._parent where h.level like 'FBS%' union all select a.class_of - 18 + h.age, h.playing_time from alumni__player__history h join alumni a on a._row = h._parent where h.level like 'FBS%') where year between 2021 and 2035 group by playing_time, year order by playing_time, year
-- FBS roster rows by season, all roles, and by age
select year, count(*) n, round(avg(age),2) mean_age from (select p.class_of - 18 + h.age year, h.age from people__player__history h join people p on p._row = h._parent where h.level like 'FBS%' union all select a.class_of - 18 + h.age, h.age from alumni__player__history h join alumni a on a._row = h._parent where h.level like 'FBS%') where year between 2021 and 2035 group by year order by year
-- FBS entrants by year and power group of the school he left
select e.year, case when p.conference in ('SEC','Big Ten','Big 12','ACC','Pac-12') then 'P4' else 'G5' end grp, count(*) n from recruiting_log__portal e join programs p on p._row = e.from_program where e."from" = 'FBS' and e.year between 2021 and 2035 group by grp, e.year order by grp, e.year
-- FBS entrants by year and the classes' age at entry (people only: those still in the book)
select year, count(*) n from recruiting_log__portal where "from" = 'FBS' and "to" is not null and year between 2021 and 2035 group by year order by year
