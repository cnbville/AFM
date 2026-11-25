-- panel ids and titles
select id || ' | ' || title || ' | ' || kind from panels order by id
-- panel rows: NIL, collectives, revenue share, money
select id, cols, substr(rows,1,6000) as rows, substr(says,1,500) as says from panels where id in ('C14.budgets','C14.by_year','C14.trend','C41.p4_bands','C41.by_position','C41.top')
select id, cols, substr(rows,1,6000) as rows, substr(says,1,500) as says from panels where id like 'N31.%' or id like 'N30.%' or id like 'N35.%' or id like 'C23.%' or id like 'P18.%'
-- panel rows: prestige and ratings
select id, cols, substr(rows,1,6000) as rows, substr(says,1,500) as says from panels where id in ('C2.spread','P8.level_ratings','P14.kept_starting','P14.starter_ages','C2.vs_wins','C2.top10_by_decade','C18.reach')
-- tables
select name from sqlite_master where type='table' order by name
