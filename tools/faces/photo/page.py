# Builds the side-by-side page: GAN proof photos vs the new photo model, same three players.
import json, base64, io
from PIL import Image
import sys
# The GAN proof photos go in gan/<key>_<age>.jpg (taken from its page).
OUT = sys.argv[1] if len(sys.argv) > 1 else "out/index.html"
gan = {p["key"]: {r["age"]: r for r in p["rows"]} for p in json.load(open("/home/user/AFM/tools/faces/results.json"))}
sc = json.load(open("out/scores.json")); tm = json.load(open("out/times.json"))
def b64(path, w):
    im = Image.open(path).convert("RGB"); h = round(im.height * w / im.width)
    buf = io.BytesIO(); im.resize((w, h), Image.LANCZOS).save(buf, "JPEG", quality=82)
    return base64.b64encode(buf.getvalue()).decode()
P = [("prude","Caleb Prude","OT · 6-6 · Black",{17:"Bessemer City HS · 268 lb",22:"Alabama · 311 lb",28:"Ravens · 320 lb",34:"Chiefs · 306 lb"}),
     ("mcspadden","Derek McSpadden","QB · 6-4 · white",{17:"Waukee HS · 192 lb",22:"Iowa State · 217 lb",28:"Packers · 224 lb",34:"Jets · 228 lb"}),
     ("meza","Leonardo Meza","LB · 6-2 · Hispanic",{17:"Edinburg North HS · 214 lb",22:"Texas A&amp;M · 237 lb",28:"Texans · 245 lb",34:"Cowboys · 241 lb"})]
def same(v): return "reference" if v >= 0.999 else f"{v:.2f}"
rows = []
for k, name, sub, teams in P:
    g = "".join(f'<figure class="shot gan"><img src="data:image/jpeg;base64,{b64(f"gan/{k}_{a}.jpg",320)}" alt="{name} at {a}, GAN" width="320" height="320"><figcaption><b>{a}</b><span class="m">same person {same(gan[k][a]["same_person"])}</span></figcaption></figure>' for a in (17,22,28,34))
    p = "".join(f'<figure class="shot"><img src="data:image/jpeg;base64,{b64(f"out/{k}_{a}.png",360)}" alt="{name} at {a}, photo model" width="360" height="463"><figcaption><b>{a}</b> · {teams[a]}<span class="m">same person {same(sc[f"{k}_{a}"]["same_person"])} · {tm[f"{k}_{a}"]/60:.1f} min</span></figcaption></figure>' for a in (17,22,28,34))
    rows.append(f'<section class="player"><header><h2>{name}</h2><p class="sub">{sub}</p></header><p class="lbl">New: photo model (RealVisXL + InstantID)</p><div class="row">{p}</div><p class="lbl">Before: GAN proof</p><div class="row small">{g}</div></section>')
notes = json.load(open("notes.json"))
html = open("tpl.html").read().replace("{{ROWS}}", "".join(rows))
for key, val in notes.items(): html = html.replace("{{" + key + "}}", val)
open(OUT, "w").write(html); print(len(html)//1024, "KB")
