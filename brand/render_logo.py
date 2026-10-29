"""Cove logo v2: impossible-C made of thick, rounded square beams + a soft blue cube.
SDF ray-marched, orthographic isometric view, flat brand tones blended smoothly on rounded edges.
Usage: python3 render_logo.py SIZE OUT.png [drop_beam]
"""
import sys
import numpy as np

SIZE = int(sys.argv[1]) if len(sys.argv) > 1 else 512
OUT = sys.argv[2] if len(sys.argv) > 2 else "logo.png"
DROP = int(sys.argv[3]) if len(sys.argv) > 3 else 2
SS = 2  # supersampling

T = 0.40      # beam thickness
R = 0.10
K = 0.002     # joints: plain union (no bulge)
CUBE = 0.19   # blue cube half size
CR = 0.07

# Hexagon loop of a unit cube's silhouette edges, viewed from (1,1,1)
corners = [(1, 0, 0), (1, 1, 0), (0, 1, 0), (0, 1, 1), (0, 0, 1), (1, 0, 1)]
beams = []
for i in range(6):
    a = np.array(corners[i], float); b = np.array(corners[(i + 1) % 6], float)
    c = (a + b) / 2 - 0.5
    half = np.abs(b - a) / 2 + T / 2
    half[half < T / 2] = T / 2
    beams.append((c, half))
beams = [bm for i, bm in enumerate(beams) if i != DROP]

view = np.array([1, 1, 1], float) / np.sqrt(3)
up_w = np.array([0, 0, 1], float)
right = np.cross(up_w, view); right /= np.linalg.norm(right)
up = np.cross(view, right)

# blue cube sits in the opening of the C (direction of the dropped beam's centre, pulled in)
c_drop = (np.array(corners[DROP], float) + np.array(corners[(DROP + 1) % 6], float)) / 2 - 0.5
cube_c = c_drop * 0.40
cube_c -= view * np.dot(cube_c, view)  # keep it on the view-centre plane


def roundbox(p, c, b, r):
    q = np.abs(p - c) - (b - r)
    return np.linalg.norm(np.maximum(q, 0), axis=-1) + np.minimum(q.max(-1), 0) - r


def smin(a, b, k):
    h = np.clip(0.5 + 0.5 * (b - a) / k, 0, 1)
    return b * (1 - h) + a * h - k * h * (1 - h)


def scene(p):
    d = None
    for c, h in beams:
        di = roundbox(p, c, h, R)
        d = di if d is None else smin(d, di, K)
    dc = roundbox(p, cube_c, np.array([CUBE] * 3), CR)
    return d, dc


N = SIZE * SS
extent = float(sys.argv[5]) if len(sys.argv) > 5 else 1.22
u = np.linspace(-extent, extent, N)
U, V = np.meshgrid(u, -u)
P = (U[..., None] * right + V[..., None] * up + view * 3).reshape(-1, 3)
D = -view
t = np.zeros(P.shape[0]); hit = np.zeros(P.shape[0], int)  # 0 none, 1 C, 2 cube
alive = np.ones(P.shape[0], bool)
for _ in range(120):
    idx = np.nonzero(alive)[0]
    if idx.size == 0:
        break
    p = P[idx] + D * t[idx, None]
    d1, d2 = scene(p)
    d = np.minimum(d1, d2)
    done = d < 1e-4
    hit[idx[done]] = np.where(d1[done] < d2[done], 1, 2)
    t[idx] += d
    gone = t[idx] > 6
    alive[idx[done | gone]] = False

pos = P + D * t[:, None]
e = 1e-3
def f(p):
    a, b = scene(p); return np.minimum(a, b)
nrm = np.stack([f(pos + [e, 0, 0]) - f(pos - [e, 0, 0]),
                f(pos + [0, e, 0]) - f(pos - [0, e, 0]),
                f(pos + [0, 0, e]) - f(pos - [0, 0, e])], -1)
nrm /= np.linalg.norm(nrm, axis=-1, keepdims=True) + 1e-9

hexc = lambda s: np.array([int(s[i:i + 2], 16) for i in (1, 3, 5)], float)
# face tones: +z top, +x, +y
PERM = sys.argv[4] if len(sys.argv) > 4 else 'dml'
TONES = {'m': hexc('#4A5160'), 'd': hexc('#15171C'), 'l': hexc('#AEB6C2')}
TC = [TONES[ch] for ch in PERM]
TB = [hexc('#3BA7FF'), hexc('#1C74C9'), hexc('#8FD0FF')]  # blue cube
w = np.clip(nrm, 0, None) ** 10
w /= w.sum(-1, keepdims=True) + 1e-9
colC = w @ np.stack(TC); colB = w @ np.stack(TB)
# soft rim highlight on rounded edges (where normal is between faces)
rim = 1 - (np.clip(nrm, 0, None) ** 2).max(-1)
rim = np.clip(rim * 1.2, 0, 1)[:, None]
colC = colC + rim * 14; colB = colB + rim * 30
col = np.where((hit == 1)[:, None], colC, colB)
alpha = (hit > 0).astype(float)
img = np.concatenate([np.clip(col, 0, 255), alpha[:, None] * 255], -1).reshape(N, N, 4)

from PIL import Image
im = Image.fromarray(img.astype(np.uint8), "RGBA")
# premultiplied downsample to avoid dark fringes
arr = np.asarray(im).astype(float); a = arr[..., 3:4] / 255
pm = np.concatenate([arr[..., :3] * a, arr[..., 3:4]], -1)
pm = pm.reshape(SIZE, SS, SIZE, SS, 4).mean((1, 3))
a2 = pm[..., 3:4] / 255
rgb = np.where(a2 > 0, pm[..., :3] / np.maximum(a2, 1e-6), 0)
Image.fromarray(np.concatenate([rgb, pm[..., 3:4]], -1).clip(0, 255).astype(np.uint8), "RGBA").save(OUT)
print("saved", OUT)
