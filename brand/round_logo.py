"""Bash-style rounding: take the flat, sharp render (raw.npz from FLAT=1 render_logo.py)
and round the whole silhouette with large curves (opening) plus soft inner fillets (closing).
Usage: python3 round_logo.py raw.npz OUT.png OUT_SIZE RO_C RC_C RO_CUBE RC_CUBE  (radii as fraction of size)
"""
import sys
import numpy as np
from scipy.ndimage import distance_transform_edt as edt
from PIL import Image

src, out = sys.argv[1], sys.argv[2]
size = int(sys.argv[3]) if len(sys.argv) > 3 else 1024
ro_c, rc_c, ro_b, rc_b = [float(v) for v in sys.argv[4:8]] if len(sys.argv) > 7 else (0.07, 0.03, 0.03, 0.0)

z = np.load(src); col = z["col"].astype(float); hit = z["hit"]
N = hit.shape[0]


def rounded(mask, ro, rc):
    ro *= N; rc *= N
    m = mask
    if rc > 0:  # closing: fillet concave corners
        dil = edt(~m) <= rc
        m = edt(dil) > rc
    er = edt(m) > ro  # opening: round convex corners
    d = edt(~er)
    return np.clip(ro - d + 0.5, 0, 1)


aC = rounded(hit == 1, ro_c, rc_c)
aB = rounded(hit == 2, ro_b, rc_b)
# fill colours for pixels the closing added: nearest original pixel of that object
def fill(mask):
    _, idx = edt(~mask, return_indices=True)
    return col[idx[0], idx[1]]
cC = fill(hit == 1); cB = fill(hit == 2)
alpha = np.clip(aC + aB, 0, 1)
rgb = (cC * aC[..., None] + cB * aB[..., None]) / np.maximum(alpha, 1e-6)[..., None]
img = np.concatenate([rgb, alpha[..., None] * 255], -1)
# premultiplied downsample
f = N // size
a = img[..., 3:4] / 255
pm = np.concatenate([img[..., :3] * a, img[..., 3:4]], -1).reshape(size, f, size, f, 4).mean((1, 3))
a2 = pm[..., 3:4] / 255
rgb2 = np.where(a2 > 0, pm[..., :3] / np.maximum(a2, 1e-6), 0)
Image.fromarray(np.concatenate([rgb2, pm[..., 3:4]], -1).clip(0, 255).astype(np.uint8), "RGBA").save(out)
print("saved", out)
