-- panel titles on money, NIL, collectives, prestige and ratings
select id || ' | ' || title from panels where title like '%NIL%' or title like '%ollective%' or title like '%evenue share%' or title like '%restige%' or title like '%ayroll%' or title like '%alary cap%' or title like '%level rating%' or title like '%tarters%' or title like '%rating%' or title like '%roster%' or title like '%hall of%' order by id limit 150
-- the book's overview
page overview limit=120
-- known issues
page issues limit=40
-- NIL collectives
panels 4 %ollective%
-- NIL
panels 6 %NIL%
-- revenue share and money by level
panels 6 %evenue share%
panels 4 %money by level%
-- prestige
panels 4 %restige%
-- ratings over time
panels 6 %starters%
panels 4 %level rating%
-- stop
