# The three players at 17, 22, 28, 34 as media-day headshots.
P = {
 "prude": dict(seed=1107, eth="Black", cls="man", pos="offensive tackle", h="6-6",
   ages={17:(268,"Bessemer City High School","purple and gold high school football jersey","no beard, clean-shaven"),
         22:(311,"Alabama","crimson red Alabama college football jersey with white numbers","light short beard"),
         28:(320,"Baltimore Ravens","purple Baltimore Ravens NFL jersey","full thick beard"),
         34:(306,"Kansas City Chiefs","red Kansas City Chiefs NFL jersey","full beard, some gray")}),
 "mcspadden": dict(seed=2202, eth="white", cls="man", pos="quarterback", h="6-4",
   ages={17:(192,"Waukee High School","purple and gold high school football jersey","clean-shaven, full head of hair"),
         22:(217,"Iowa State","cardinal red Iowa State college football jersey with gold trim","clean-shaven, full head of hair"),
         28:(224,"Green Bay Packers","dark green Green Bay Packers NFL jersey","clean-shaven, slightly receding hairline"),
         34:(228,"New York Jets","green and white New York Jets NFL jersey","light stubble, clearly receding hairline")}),
 "meza": dict(seed=3303, eth="Mexican-American Hispanic", cls="man", pos="linebacker", h="6-2",
   ages={17:(214,"Edinburg North High School","navy and gold high school football jersey","clean-shaven"),
         22:(237,"Texas A&M","maroon Texas A&M college football jersey with white numbers","short trimmed goatee"),
         28:(245,"Houston Texans","deep navy blue Houston Texans NFL jersey with red trim","short trimmed beard"),
         34:(241,"Dallas Cowboys","white and navy Dallas Cowboys NFL jersey","short trimmed beard")}),
}
def build(w, pos):
    if pos=="offensive tackle": return "huge heavy lineman build, very thick neck, broad round face" if w>290 else "big tall frame, thick neck"
    if pos=="quarterback": return "tall athletic build" if w<210 else "tall solid athletic build"
    return "muscular thick athletic build, thick neck"
def prompt(k, age, trigger):
    p=P[k]; w,team,jersey,face=p["ages"][age]
    who = "teenage boy" if age==17 else p["cls"]
    agew = {17:"17 year old high school junior, youthful face",22:"22 year old college player",28:"28 year old man",34:"34 year old man, mature face"}[age]
    return (f"media day headshot photo, {p['eth']} {who}{' img' if trigger else ''}, {agew}, {p['pos']} {p['h']} {w} lb, "
            f"{build(w,p['pos'])}, {face}, short hair, {jersey}, shoulder pads, studio lighting, team backdrop, neutral expression, sharp, natural skin")
NEG = "cartoon, illustration, 3d render, painting, deformed, blurry, helmet, hat, sunglasses, text, watermark, extra people, open mouth, smiling teeth"
