"""Readers for generated faces: insightface (detection, 106 landmarks, pose, sex, age, identity embedding),
deepface's VGG race and age models (weights loaded into PyTorch), and landmark-based build, beard and smile scores."""
import os

import cv2
import h5py
import numpy as np
import torch
from torch import nn
from torch.nn import functional as F

HERE = os.path.dirname(__file__)
MODELS = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'models')
RACES = ['asian', 'indian', 'black', 'white', 'middle eastern', 'latino hispanic']


class VGGHead(nn.Module):
    """deepface VGG-Face base with a 1x1 conv head (race: 6 classes, age: 101 classes)."""

    def __init__(self, path, n_out):
        super().__init__()
        cfg = [64, 64, 'M', 128, 128, 'M', 256, 256, 256, 'M', 512, 512, 512, 'M', 512, 512, 512, 'M']
        self.convs = nn.ModuleList()
        self.plan = []
        c = 3
        for v in cfg:
            if v == 'M':
                self.plan.append('M')
            else:
                self.convs.append(nn.Conv2d(c, v, 3))
                self.plan.append(len(self.convs) - 1)
                c = v
        self.fc14 = nn.Conv2d(512, 4096, 7)
        self.fc15 = nn.Conv2d(4096, 4096, 1)
        self.pred = nn.Conv2d(4096, n_out, 1)
        f = h5py.File(path, 'r')

        def load(mod, name):
            g = f[name]
            ks = [k.decode() for k in g.attrs['weight_names']]
            k = np.array(g[ks[0]])
            b = np.array(g[ks[1]])
            mod.weight.data = torch.from_numpy(k.transpose(3, 2, 0, 1).copy())
            mod.bias.data = torch.from_numpy(b.copy())

        for i, conv in enumerate(self.convs):
            load(conv, f'conv2d_{i + 1}')
        load(self.fc14, 'conv2d_14')
        load(self.fc15, 'conv2d_15')
        load(self.pred, 'predictions')
        self.eval()

    def forward(self, x):
        for p in self.plan:
            if p == 'M':
                x = F.max_pool2d(x, 2, 2)
            else:
                x = F.relu(self.convs[p](F.pad(x, (1, 1, 1, 1))))
        x = F.relu(self.fc14(x))
        x = F.relu(self.fc15(x))
        return F.softmax(self.pred(x).flatten(1), 1)


class Reader:

    def __init__(self):
        from insightface.app import FaceAnalysis
        self.app = FaceAnalysis(name='buffalo_l', root=os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'ins'),
                                providers=['CPUExecutionProvider'])
        self.app.prepare(ctx_id=-1, det_size=(320, 320))
        self.race = VGGHead(os.path.join(MODELS, 'race.h5'), 6)
        self.agem = VGGHead(os.path.join(MODELS, 'age.h5'), 101)

    def vgg_crop(self, bgr, bbox):
        x0, y0, x1, y1 = bbox
        w, h = x1 - x0, y1 - y0
        m = 0.1
        x0, x1 = int(max(0, x0 - m * w)), int(min(bgr.shape[1], x1 + m * w))
        y0, y1 = int(max(0, y0 - m * h)), int(min(bgr.shape[0], y1 + m * h))
        crop = bgr[y0:y1, x0:x1]
        s = max(crop.shape[:2])
        pad = np.zeros((s, s, 3), np.uint8)
        oy, ox = (s - crop.shape[0]) // 2, (s - crop.shape[1]) // 2
        pad[oy:oy + crop.shape[0], ox:ox + crop.shape[1]] = crop
        img = cv2.resize(pad, (224, 224)).astype(np.float32) / 255.0  # deepface feeds BGR / 255
        return torch.from_numpy(img.transpose(2, 0, 1)).unsqueeze(0)

    def read(self, rgb):
        """rgb: HxWx3 uint8. Returns a dict of readings, or None when no face is found."""
        bgr = cv2.cvtColor(np.ascontiguousarray(rgb), cv2.COLOR_RGB2BGR)
        faces = self.app.get(bgr)
        if not faces:
            return None
        f = max(faces, key=lambda q: (q.bbox[2] - q.bbox[0]) * (q.bbox[3] - q.bbox[1]))
        with torch.no_grad():
            x = self.vgg_crop(bgr, f.bbox)
            race = self.race(x)[0].numpy()
            agep = self.agem(x)[0].numpy()
        lm = f.landmark_2d_106
        kps = f.kps  # eyes, nose, mouth corners
        iod = np.linalg.norm(kps[0] - kps[1])
        out = {
            'sex_m': 1.0 if f.sex == 'M' else 0.0,
            'age_if': float(f.age),
            'age_df': float((agep * np.arange(101)).sum()),
            'race': race.astype(np.float32),
            'yaw': float(f.pose[1]),
            'pitch': float(f.pose[0]),
            'emb': f.normed_embedding.astype(np.float32),
            'det': float(f.det_score),
        }
        out.update(shape_scores(rgb, lm, kps, iod))
        return out


def shape_scores(rgb, lm, kps, iod):
    """Build (face fullness), beard and smile from landmarks and pixels. Heuristics, scaled by eye distance."""
    eye_y = (kps[0][1] + kps[1][1]) / 2
    mouth_y = (kps[3][1] + kps[4][1]) / 2
    contour = lm[0:33]
    below = contour[contour[:, 1] > eye_y]
    hull = cv2.convexHull(below.astype(np.float32))
    area = cv2.contourArea(hull) / (iod ** 2)
    band = contour[(contour[:, 1] > mouth_y - 0.1 * iod) & (contour[:, 1] < mouth_y + 0.3 * iod)]
    jaw_w = (band[:, 0].max() - band[:, 0].min()) / iod if len(band) >= 2 else np.nan
    chin_y = contour[:, 1].max()
    smile = (np.linalg.norm(kps[3] - kps[4]) / iod)
    # beard: chin and jaw patch below the mouth against a cheek patch under each eye
    lab = cv2.cvtColor(np.ascontiguousarray(rgb), cv2.COLOR_RGB2LAB).astype(np.float32)
    L = lab[:, :, 0]
    gray = cv2.cvtColor(np.ascontiguousarray(rgb), cv2.COLOR_RGB2GRAY).astype(np.float32)
    lap = np.abs(cv2.Laplacian(gray, cv2.CV_32F, ksize=3))
    H, W = L.shape

    def patch(cx, cy, r):
        x0, x1 = int(max(0, cx - r)), int(min(W, cx + r))
        y0, y1 = int(max(0, cy - r)), int(min(H, cy + r))
        if x1 <= x0 or y1 <= y0:
            return np.nan, np.nan
        return L[y0:y1, x0:x1].mean(), lap[y0:y1, x0:x1].mean()

    r = 0.12 * iod
    mx = (kps[3][0] + kps[4][0]) / 2
    chin = patch(mx, (mouth_y + chin_y) / 2 + 0.05 * iod, r)
    jl = patch(kps[3][0] - 0.05 * iod, mouth_y + 0.35 * iod, r)
    jr = patch(kps[4][0] + 0.05 * iod, mouth_y + 0.35 * iod, r)
    cl = patch(kps[0][0], eye_y + 0.55 * iod, r)
    cr = patch(kps[1][0], eye_y + 0.55 * iod, r)
    low_L = np.nanmean([chin[0], jl[0], jr[0]])
    low_T = np.nanmean([chin[1], jl[1], jr[1]])
    up_L = np.nanmean([cl[0], cr[0]])
    up_T = np.nanmean([cl[1], cr[1]])
    beard = (up_L - low_L) / max(up_L, 1) + 0.5 * np.log((low_T + 1) / (up_T + 1))
    return {'fullness': float(area), 'jaw_w': float(jaw_w), 'smile': float(smile), 'beard': float(beard),
            'skin_L': float(up_L)}
