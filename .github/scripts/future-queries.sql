-- END state (1 Jan 2036): FBS college players by last role
select p.last_role role, count(*) n, round(avg(p.cal__lean),4) mean_lean, round(sum(p.cal__lean),1) lean_sum, round(avg(p.college_seasons),2) seasons, round(avg(p.walk_on),3) walk_on, round(avg(p.x_overall),2) ovr from people p join programs pr on pr._row = p.college where p.stage = 'College' and pr.division = 'FBS' group by p.last_role
-- END: FBS roster size by program
select count(*) programs, round(avg(n),1) mean, min(n) lo, max(n) hi from (select pr._row, count(*) n from people p join programs pr on pr._row = p.college where p.stage = 'College' and pr.division = 'FBS' group by pr._row)
-- END: FBS players by seasons used and role
select p.college_seasons seasons, p.last_role role, count(*) n, round(avg(p.cal__lean),4) mean_lean from people p join programs pr on pr._row = p.college where p.stage = 'College' and pr.division = 'FBS' group by 1, 2 order by 1, 2
-- END: FBS players by class year and role
select p.class_of, p.last_role role, count(*) n, round(avg(p.cal__lean),4) mean_lean, round(avg(p.x_overall),1) ovr from people p join programs pr on pr._row = p.college where p.stage = 'College' and pr.division = 'FBS' group by 1, 2 order by 1, 2
-- START state (1 Jan 2026): the same four
@start select p.last_role role, count(*) n, round(avg(p.cal__lean),4) mean_lean, round(sum(p.cal__lean),1) lean_sum, round(avg(p.college_seasons),2) seasons, round(avg(p.walk_on),3) walk_on, round(avg(p.x_overall),2) ovr from people p join programs pr on pr._row = p.college where p.stage = 'College' and pr.division = 'FBS' group by p.last_role
@start select count(*) programs, round(avg(n),1) mean, min(n) lo, max(n) hi from (select pr._row, count(*) n from people p join programs pr on pr._row = p.college where p.stage = 'College' and pr.division = 'FBS' group by pr._row)
@start select p.college_seasons seasons, p.last_role role, count(*) n, round(avg(p.cal__lean),4) mean_lean from people p join programs pr on pr._row = p.college where p.stage = 'College' and pr.division = 'FBS' group by 1, 2 order by 1, 2
@start select p.class_of, p.last_role role, count(*) n, round(avg(p.cal__lean),4) mean_lean, round(avg(p.x_overall),1) ovr from people p join programs pr on pr._row = p.college where p.stage = 'College' and pr.division = 'FBS' group by 1, 2 order by 1, 2
