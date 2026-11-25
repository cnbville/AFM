-- tables
select name from sqlite_master where type='table' and name not like 'sqlite_%' and name not like '%fts%' order by name
-- the book's overview
page overview limit=150
-- odd things
page odd limit=100
-- known issues
page issues limit=60
-- money
page money limit=400
