"""Build the proof page: every headshot embedded as a small JPEG, with the readings under it.
Usage: build_page.py <out.html> [notes.json]"""
import base64
import io
import json
import os
import sys

from PIL import Image

OUT = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'out')


def b64(path, size=320):
    im = Image.open(path).convert('RGB')
    im.thumbnail((size, size))
    buf = io.BytesIO()
    im.save(buf, 'JPEG', quality=84)
    return 'data:image/jpeg;base64,' + base64.b64encode(buf.getvalue()).decode()


def main():
    res = json.load(open(os.path.join(OUT, 'results.json')))
    notes = json.load(open(sys.argv[2])) if len(sys.argv) > 2 else {}
    her_label = {'black': 'Black', 'white': 'White', 'latino': 'Hispanic', 'asian': 'Asian'}
    cards = []
    for p in res:
        cells = []
        for r in p['rows']:
            sp = r['same_person']
            sp_txt = 'reference' if r['age'] == 22 else (f'{sp:.2f}' if sp is not None else 'no face found')
            cells.append(f'''<figure class="shot"><img src="{b64(os.path.join(OUT, 'shots', r['file']))}" alt="{p['name']} at {r['age']}" width="320" height="320">
<figcaption><b>{r['age']}</b> · {r['team']} · {r['weight']} lb
<span class="m">reads as {r['read_age'] if r['read_age'] is not None else '?'} years · same person {sp_txt}</span></figcaption></figure>''')
        note = notes.get(p['key'], '')
        cards.append(f'''<section class="player"><header><h2>{p['name']}</h2><p class="sub">{p['pos']} · {p['ht']} · name and face both {her_label.get(p['heritage'], p['heritage'])}</p></header>
<div class="row">{''.join(cells)}</div>{f'<p class="why">{note}</p>' if note else ''}</section>''')
    head = notes.get('head', '')
    tail = notes.get('tail', '')
    html = f'''<title>AFM Face Proof</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Barlow+Condensed:wght@600;700&family=Source+Sans+3:wght@400;600&display=swap">
<style>
/* Layout: one band per player, four media-day headshots across, readings under each */
:root{{--bg:#eef0f2;--panel:#fff;--fg:#1b2026;--muted:#5b6570;--line:#d5dade;--accent:#c8102e;
--display:"Barlow Condensed","Arial Narrow",sans-serif;--body:"Source Sans 3",system-ui,sans-serif}}
@media (prefers-color-scheme:dark){{:root:not([data-theme="light"]){{--bg:#14181c;--panel:#1d2329;--fg:#e8ecef;--muted:#9aa4ae;--line:#2e363e;--accent:#ff4d5e;color-scheme:dark}}}}
:root[data-theme="dark"]{{--bg:#14181c;--panel:#1d2329;--fg:#e8ecef;--muted:#9aa4ae;--line:#2e363e;--accent:#ff4d5e;color-scheme:dark}}
body{{background:var(--bg);color:var(--fg);font-family:var(--body);font-size:15px;line-height:1.5}}
.wrap{{max-width:1100px;margin:0 auto;padding-block:28px 48px;padding-left:max(16px,env(safe-area-inset-left,0px));padding-right:max(16px,env(safe-area-inset-right,0px))}}
h1,h2{{font-family:var(--display);text-transform:uppercase;letter-spacing:.02em;margin:0;text-wrap:balance}}
h1{{font-size:40px;line-height:1}} h2{{font-size:26px}}
.lead,.why,.tail p{{max-width:70ch;color:var(--muted)}}
.lead{{margin:8px 0 0}}
.player{{border-top:1px solid var(--line);padding-block:22px;display:grid;gap:12px}}
.sub{{margin:2px 0 0;color:var(--muted);font-size:14px}}
.row{{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px}}
.shot{{margin:0;background:var(--panel);border:1px solid var(--line);border-radius:6px;overflow:hidden}}
.shot img{{display:block;width:100%;height:auto;max-width:100%;aspect-ratio:1/1}}
figcaption{{padding:8px 10px 10px;font-size:13px;font-variant-numeric:tabular-nums}}
figcaption b{{font-family:var(--display);font-size:18px;color:var(--accent)}}
.m{{display:block;color:var(--muted);font-size:12px}}
.why{{margin:0;font-size:14px}}
.tail{{border-top:1px solid var(--line);padding-top:22px}}
.tail h2{{margin-bottom:6px}}
td{{padding:4px 10px;border-top:1px solid var(--line)}} td:first-child{{padding-left:0}}
@media (max-width:760px){{.row{{grid-template-columns:repeat(2,minmax(0,1fr))}}}}
</style>
<div class="wrap">
<h1>Face proof</h1>
<p class="lead">{head}</p>
{''.join(cards)}
<div class="tail">{tail}</div>
</div>'''
    open(sys.argv[1], 'w').write(html)
    print('wrote', sys.argv[1], round(len(html) / 1e6, 2), 'MB')


if __name__ == '__main__':
    main()
