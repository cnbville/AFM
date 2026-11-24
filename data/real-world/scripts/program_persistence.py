"""Real FBS program persistence, 2001-2025 (rise and fall of programs, docs/design/program-fortunes.md).

Reads sportsdataverse-data cfb_schedules_YEAR.parquet 2001-2025 in DIR
(https://github.com/sportsdataverse/sportsdataverse-data/releases/tag/cfb_schedules).
FBS team seasons with 8+ games. Usage: python3 program_persistence.py DIR
"""
import sys, glob, numpy as np, pandas as pd
D=sys.argv[1]
s=pd.concat([pd.read_parquet(f) for f in sorted(glob.glob(D+"/cfb_schedules_*.parquet"))])
s=s[(s.completed==True)&s.home_points.notna()&s.away_points.notna()]
rows=[]
for side,o in (("home","away"),("away","home")):
    t=pd.DataFrame({"season":s.season,"team":s[side+"_team"],"div":s[side+"_division"],"opp":s[o+"_team"],"odiv":s[o+"_division"],
      "pf":s[side+"_points"],"pa":s[o+"_points"],"neutral":s.neutral_site,"home":side=="home","date":s.start_date,"stype":s.season_type})
    rows.append(t)
g=pd.concat(rows); g=g[g["div"]=="fbs"]
g["w"]=(g.pf>g.pa).astype(float); g["l"]=(g.pf<g.pa).astype(float)
rec=g.groupby(["team","season"]).agg(w=("w","sum"),l=("l","sum"),n=("w","size"))
rec["wp"]=rec.w/(rec.w+rec.l)
# simple SRS: FBS v FBS, margin capped 28, home 2.5
srs={}
for y,gy in g[(g.odiv=="fbs")].groupby("season"):
    gy=gy[gy.home|gy.neutral]  # one row per game
    teams=sorted(set(gy.team)|set(gy.opp)); ix={t:i for i,t in enumerate(teams)}
    A=np.zeros((len(gy)+1,len(teams))); b=np.zeros(len(gy)+1)
    for k,(t,o,m,neu) in enumerate(zip(gy.team,gy.opp,(gy.pf-gy.pa).clip(-28,28),gy.neutral)):
        A[k,ix[t]]=1;A[k,ix[o]]=-1;b[k]=m-(0 if neu else 2.5)
    A[-1,:]=1
    x=np.linalg.lstsq(A,b,rcond=None)[0]
    for t,i in ix.items(): srs[(t,y)]=x[i]
rec["srs"]=pd.Series(srs)
rec=rec[rec.n>=8].reset_index()
def lagc(col,L):
    a=rec[["team","season",col]].copy(); b=a.copy(); b["season"]-=L
    m=a.merge(b,on=["team","season"]).dropna()
    return m[col+"_x"].corr(m[col+"_y"]), len(m)
for L in (1,2,5,10,15,20,24):
    print("lag",L,"srs r %.2f (n %d)"%lagc("srs",L),"win share r %.2f (n %d)"%lagc("wp",L))
tot=rec.groupby("team").agg(w=("w","sum"),l=("l","sum"),seasons=("season","size"))
tot["pct"]=tot.w/(tot.w+tot.l)
full=tot[tot.seasons>=22].sort_values("pct")
print("programs 22+ FBS seasons:",len(full))
print("top 5:",[(t,int(r.w),int(r.l),round(r.pct,3)) for t,r in full[::-1].head(5).iterrows()])
print("bottom 8:",[(t,int(r.w),int(r.l),round(r.pct,3)) for t,r in full.head(8).iterrows()])
print("at .300 or under:",(full.pct<=.300).sum(),", under .350:",(full.pct<.350).sum(),", over .750:",(full.pct>.75).sum())
# winning seasons
rec["winning"]=rec.w>rec.l
ws=rec.groupby("team").winning.agg(["sum","size"])
ws=ws[ws["size"]>=22]
print("22+ season programs with 0 winning seasons:",(ws["sum"]==0).sum(),"with <=2:",(ws["sum"]<=2).sum(), list(ws[ws["sum"]<=2].index))
# longest stretch without a winning season
def longest(x):
    best=cur=0
    for v in x:
        cur=0 if v else cur+1; best=max(best,cur)
    return best
dry=rec.sort_values("season").groupby("team").winning.apply(longest).sort_values(ascending=False)
print("longest droughts:",dry.head(6).to_dict())
# unbeaten
unb=rec[(rec.l==0)&(rec.w>=10)]
print("unbeaten a year: %.2f"%(len(unb)/rec.season.nunique()), unb.groupby("season").size().to_dict())
# win streak
gg=g.sort_values("date")
best=("",0)
for t,x in gg.groupby("team"):
    cur=0
    for w,l in zip(x.w,x.l):
        if w: cur+=1
        elif l: cur=0
        if cur>best[1]: best=(t,cur)
print("longest win streak:",best)
# share of programs whose 5-yr SRS rank moved 30+ places
rec["rank"]=rec.groupby("season").srs.rank(ascending=False)
p=rec.pivot(index="team",columns="season",values="rank")
def era(a,b): return p.loc[:,a:b].mean(axis=1)
e1=era(2001,2005);e2=era(2011,2015);e3=era(2021,2025)
d=(e3-e1).dropna()
print("5-yr mean SRS rank 2001-05 vs 2021-25: moved 30+ places %.0f%% (n %d); r %.2f"%(100*(d.abs()>=30).mean(),len(d),e1.corr(e3)))
top=e1.sort_values().head(10).index; print("2001-05 top 10 still top 10 in 2021-25:",sum(e3.rank()[top]<=10),"; in top 25:",sum(e3.rank()[top]<=25))


# 5-year blocks: rises and falls
rec["blk"]=(rec.season-2001)//5
b=rec.groupby(["team","blk"]).agg(w=("w","sum"),l=("l","sum"),n=("season","size"))
b=b[b.n>=4]; b["wp"]=b.w/(b.w+b.l); b=b.reset_index()
m=b.merge(b.assign(blk=b.blk-1),on=["team","blk"])
d=m.wp_y-m.wp_x
print("5-yr block to next: r %.2f; moved .150+ %.0f%%, .250+ %.0f%% (n %d)"%(m.wp_x.corr(m.wp_y),100*(d.abs()>=.15).mean(),100*(d.abs()>=.25).mean(),len(m)))
for k in (5,10): pass
# programs that were bottom quarter in a block and top quarter in a later block (rise), and the reverse
b["q"]=b.groupby("blk").wp.rank(pct=True)
p=b.pivot(index="team",columns="blk",values="q")
rise=((p.min(axis=1)<=.25)&(p.max(axis=1)>=.75)); print("programs in both bottom and top quarter of 5-yr blocks:",int(rise.sum()),"of",p.dropna(thresh=4).shape[0], list(p[rise].index)[:20])

# Mean margin a game (all games, schedule not taken out)
x = g.groupby(["team", "season"]).agg(m=("pf", "mean"), n=("pf", "size")).reset_index()
x["m"] = g.assign(d=g.pf - g.pa).groupby(["team", "season"]).d.mean().values
x = x[x.n >= 8]
for L in (1, 5, 10, 15, 20):
    mm = x.merge(x.assign(season=x.season - L), on=["team", "season"])
    print("lag", L, "margin r %.2f (n %d)" % (mm.m_x.corr(mm.m_y), len(mm)))
