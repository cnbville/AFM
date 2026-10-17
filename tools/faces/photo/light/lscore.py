import json, numpy as np, cv2
from insightface.app import FaceAnalysis
fa = FaceAnalysis(name="buffalo_l", root="ins", providers=["CPUExecutionProvider"]); fa.prepare(ctx_id=-1, det_size=(640,640))
res = {}
for k in ["prude","mcspadden","meza"]:
    def face(a):
        f = fa.get(cv2.imread(f"light/{k}_{a}.png")); return max(f, key=lambda x: x.bbox[2]-x.bbox[0])
    ref = face(22)
    for a in [17,22,28,34]:
        f = face(a); res[f"{k}_{a}"] = dict(same_person=round(float(np.dot(f.normed_embedding, ref.normed_embedding)),3), read_age=int(f.age))
json.dump(res, open("light/scores.json","w"), indent=1); print(json.dumps(res))
