# Tries face-lock settings on Prude at 17 and 28 with the light model.
import sys, json, time, numpy as np, cv2, torch
sys.argv = ["x", "8", "5", "0.7", "0.6"]
src = open("light.py").read()
exec(src.split("# Age 22:")[0])
SEED = {}
exec(src.split("# Face lock for the other ages.")[1].split("for k in P:\n    t = time.time()")[0])
from diffusers import DPMSolverMultistepScheduler
ddim = pipe.scheduler; dpm = DPMSolverMultistepScheduler.from_config(ddim.config, use_karras_sigmas=True, timestep_spacing="leading")
ref = Image.open("light/prude_22.png").convert("RGB"); f = face(ref)
idv = torch.from_numpy(f.normed_embedding).reshape(1, 1, 512)
crop = Image.fromarray(face_align.norm_crop(np.array(ref), landmark=f.kps, image_size=224))
proj.clip_embeds = pipe.prepare_ip_adapter_image_embeds([crop], None, "cpu", 1, True)[0]; proj.shortcut = True
res = {}
for name, steps, cfg, hyper, lora, ips in [("A",8,3.0,1.0,0.5,0.6),("B",8,2.0,1.0,0.6,0.7),("C",20,4.5,0.0,0.6,0.7)]:
    pipe.scheduler = ddim if hyper else dpm
    pipe.set_adapters(["hyper","faceid"], adapter_weights=[hyper, lora]); pipe.set_ip_adapter_scale(ips)
    for age in (17, 28):
        STEPS, CFG = steps, cfg
        t = time.time(); run("prude", age, ip_adapter_image_embeds=[torch.cat([torch.zeros_like(idv), idv])])
        import shutil; shutil.move(f"light/prude_{age}.png", f"light/test{name}_{age}.png")
        g = face(Image.open(f"light/test{name}_{age}.png"))
        res[f"{name}_{age}"] = (round(float(np.dot(g.normed_embedding, f.normed_embedding)),3), round(time.time()-t))
        print(name, age, res[f"{name}_{age}"], flush=True)
