"""Face parsing (BiSeNet, CelebAMask-HQ classes) for glasses, hats, hair amount, and the media-day cut-out."""
import os
import sys

import cv2
import numpy as np
import torch

sys.path.insert(0, os.path.dirname(__file__))
from bisenet import BiSeNet  # noqa: E402

MODELS = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'models')
# 0 bg, 1 skin, 2-3 brows, 4-5 eyes, 6 glasses, 7-8 ears, 9 earring, 10 nose, 11 mouth, 12-13 lips,
# 14 neck, 15 necklace, 16 cloth, 17 hair, 18 hat
MEAN = np.array([0.485, 0.456, 0.406], np.float32)
STD = np.array([0.229, 0.224, 0.225], np.float32)


class Parser:

    def __init__(self):
        self.net = BiSeNet(num_class=19)
        self.net.load_state_dict(torch.load(os.path.join(MODELS, 'parsing_bisenet.pth'), map_location='cpu',
                                            weights_only=True))
        self.net.eval()

    def labels(self, rgb):
        h, w = rgb.shape[:2]
        x = (cv2.resize(rgb, (512, 512)).astype(np.float32) / 255. - MEAN) / STD
        with torch.no_grad():
            out = self.net(torch.from_numpy(x.transpose(2, 0, 1)).unsqueeze(0))[0]
        lab = out.argmax(1)[0].numpy().astype(np.uint8)
        if (h, w) != (512, 512):
            lab = cv2.resize(lab, (w, h), interpolation=cv2.INTER_NEAREST)
        return lab

    def features(self, rgb):
        lab = self.labels(rgb)
        n = lab.size
        face = np.isin(lab, [1, 2, 3, 4, 5, 10, 11, 12, 13]).sum()
        hair = (lab == 17)
        # hair above the brows only: a receding hairline or a shaved head shows up here
        brow_rows = np.where(np.isin(lab, [2, 3]).any(1))[0]
        top_hair = hair[:brow_rows.min()].sum() if len(brow_rows) else hair.sum()
        return {
            'glasses': float((lab == 6).sum()) / max(face, 1),
            'hat': float((lab == 18).sum()) / n,
            'hair': float(hair.sum()) / n,
            'top_hair': float(top_hair) / max(face, 1),
            'earring': float((lab == 9).sum()) / n,
        }


if __name__ == '__main__':
    # Read every pool picture and append parsing features to out/pool_parse.npz
    torch.set_num_threads(4)
    out = os.path.join(os.environ.get('AFM_FACES_DIR', '.'), 'out')
    n = len(np.load(os.path.join(out, 'pool.npz'))['w'])
    files = sorted(os.listdir(os.path.join(out, 'pool')))[:n]
    p = Parser()
    keys = ['glasses', 'hat', 'hair', 'top_hair', 'earring']
    rows = []
    for f in files:
        rgb = cv2.imread(os.path.join(out, 'pool', f))[:, :, ::-1]
        o = p.features(np.ascontiguousarray(rgb))
        rows.append([o[k] for k in keys])
    np.savez(os.path.join(out, 'pool_parse.npz'), vals=np.array(rows, np.float32), keys=np.array(keys))
    print('parsed', len(rows))
