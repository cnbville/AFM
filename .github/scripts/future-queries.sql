rsql select k.key, substr(k.value,1,200) from log_clubs lc, json_each(more('log_clubs', lc._more), '$') k where lc._row = (select min(_row) from log_clubs where season = 2034)
rsql select season, count(*) from log_clubs group by season
