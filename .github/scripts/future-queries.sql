-- schema: portal log, history rows, log_history
select 'recruiting_log tables', group_concat(name, ', ') from sqlite_master where type='table' and name like 'recruiting_log%'
select 'recruiting_log__portal', group_concat(name, ', ') from pragma_table_info('recruiting_log__portal')
select 'people__player__history', group_concat(name, ', ') from pragma_table_info('people__player__history')
select 'programs', group_concat(name, ', ') from pragma_table_info('programs')
-- FBS entrants by year and why
select year, why, count(*) n from recruiting_log__portal where "from" = 'FBS' and year between 2021 and 2035 group by why, year order by why, year
-- FBS entrants by year and role
select year, role, count(*) n from recruiting_log__portal where "from" = 'FBS' and year between 2021 and 2035 group by role, year order by role, year
-- FBS entrants by year: held, returned, fell through, landed none
select year, count(*) n, sum(held) held, sum(returned) returned, sum(fell_through) fell, sum("to" is null) none, sum("to" = 'FBS') to_fbs, sum("to" = 'FCS') to_fcs from recruiting_log__portal where "from" = 'FBS' and year between 2021 and 2035 group by year order by year
-- every division's entrants by year
select year, "from", count(*) n from recruiting_log__portal where year between 2021 and 2035 group by "from", year order by "from", year
-- the world's yearly flows
select year, commits, signed_early, signed_late, walk_on_scholarships, tryouts, late_pool, graduates, graduate_transfers, declared, early_drafted, portal_entries, portal_landed, portal_returned, portal_unplaced_at_camp, portal_landed_after, college_rolled, college_trouble, sit_out_transfers, opt_outs, redshirt_tricks from log_history where year between 2021 and 2035 order by year
