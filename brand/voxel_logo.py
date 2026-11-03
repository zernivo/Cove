"""Cove voxel logo options (A/B/C), rendered from the intro's own geometry.

Geometry mirrors ui/src/intro-core.ts: model() builds the C as a thick ring
around the six silhouette edges of an N-cube (bar thickness T), opened on the
right; drawTeal() puts a T-sized teal block at the centre, drawn last. Colours
are FACE / TEAL from the same file (left face = dark shade, right = mid shade,
exactly as the intro draws them).

Rendering: every face is rasterised as a hard-edged polygon into a face-id
map at a large supersample, then colourised and box-averaged down. Hard
polygons in painter's order cannot leave seams; box averaging gives clean
anti-aliasing with no ringing. Small flat marks (option C) are drawn on the
exact pixel grid instead.

Options
  A  intro final frame: N=12, T=3, seams closed (solid object).
  B  coarse: N=4, T=1 (each block = 3x3x3 intro voxels), blocks kept visible
     with thin Ink joints at large sizes; joints dropped below 64 px.
  C  system: full-colour isometric C with its block joints at large sizes,
     a flat front-on 3x3 block C (teal block in the opening) for 32 px and
     for the monochrome menu bar mark.

Usage: python voxel_logo.py [outdir]
"""
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont

OUT = sys.argv[1] if len(sys.argv) > 1 else \
    "/workspace/projects/52e7094c-440c-4f43-9b7f-f1b831d9e8a2/notes/mockups/logo-voxel"
os.makedirs(OUT, exist_ok=True)
FONT = "/workspace/work/concept/inter/InterVariable.ttf"

INK = (0x11, 0x12, 0x15)
PAPER = (0xF2, 0xEE, 0xE9)   # wordmark colour used by the intro
GREY = (0x8B, 0x8B, 0x93)    # tagline colour used by the intro
hexc = lambda h: tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))
FACE = {"top": hexc("#f7dccd"), "left": hexc("#8c5a64"), "right": hexc("#c48b82")}
TEAL = {"top": hexc("#a6f2e6"), "left": hexc("#169487"), "right": hexc("#2cc9b9")}
TEAL_FLAT = hexc("#2cc9b9")

# face ids in the label map
NONE, RT, RL, RR, TT, TL, TR, SEAM = range(8)


# ---------------------------------------------------------------- geometry
def model(N, T):
    """Port of intro-core.ts model(): voxel coords of the C ring."""
    near = lambda a, edge: (a < T) if edge == 0 else (a >= N - T)
    v = []
    for x in range(N):
        for y in range(N):
            for z in range(N):
                bars = [near(x, N) and near(y, 0), near(x, 0) and near(y, N),
                        near(x, N) and near(z, 0), near(y, N) and near(z, 0),
                        near(x, 0) and near(z, N), near(y, 0) and near(z, N)]
                if not any(bars):
                    continue
                if bars[0] and not bars[2] and not bars[5]:
                    continue
                if bars[0] and T <= z < N - T:
                    continue
                v.append((x, y, z))
    v.sort(key=lambda p: (p[0] + p[1] + p[2], p[2]))
    c = (N - 1) / 2
    return [(x - c, y - c, z - c) for x, y, z in v]


def teal(T):
    """Port of drawTeal(): visible shell of a TxTxT block at the centre."""
    o = [k - (T - 1) / 2 for k in range(T)]
    m = o[-1]
    sh = [(x, y, z) for x in o for y in o for z in o if m in (x, y, z)]
    sh.sort(key=lambda p: (p[0] + p[1] + p[2], p[2]))
    return sh


ISO = 0.866   # true isometric, as the intro
PIX = 1.0     # 2:1 pixel-art isometric, for pixel-exact small marks


def cube_faces(sx, sy, w=ISO):
    """Unit-size (s=1) cube faces, same construction as intro cube()."""
    hh = 0.5
    top = sy - 1
    return [
        ("top", [(sx, top - hh), (sx + w, top), (sx, top + hh), (sx - w, top)]),
        ("left", [(sx - w, top), (sx, top + hh), (sx, sy + hh), (sx - w, sy)]),
        ("right", [(sx + w, top), (sx, top + hh), (sx, sy + hh), (sx + w, sy)]),
    ]


def P(x, y, z, w=ISO):
    return ((x - y) * w, (x + y) * 0.5 - z)


def scene(N, T, w=ISO):
    """List of cubes, each a list of (face_id, polygon), in painter's order."""
    out = []
    for kind, vox in (("ring", model(N, T)), ("teal", teal(T))):
        for x, y, z in vox:
            sx, sy = P(x, y, z, w)
            faces = []
            for name, poly in cube_faces(sx, sy, w):
                fid = {"top": RT, "left": RL, "right": RR}[name]
                if kind == "teal":
                    fid += 3
                faces.append((fid, poly))
            out.append(faces)
    return out


def bbox(sc):
    xs = [p[0] for c in sc for _, poly in c for p in poly]
    ys = [p[1] for c in sc for _, poly in c for p in poly]
    return min(xs), min(ys), max(xs), max(ys)


# --------------------------------------------------------------- rendering
def label_map(N, T, box_w, box_h, ss, seam=0.0, w=ISO, unit=None):
    """Face-id map and seam mask (box*ss), logo fitted and centred.
    seam: joint width as a fraction of the voxel edge (0 = solid). Seams are
    painted per cube in painter's order, so a cube in front hides the joints
    behind it (exactly how the intro strokes each cube). unit: fixed px per
    voxel (pixel-art mode) instead of fit-to-box."""
    sc = scene(N, T, w)
    x0, y0, x1, y1 = bbox(sc)
    W, H = box_w * ss, box_h * ss
    s = unit * ss if unit else min(W / (x1 - x0), H / (y1 - y0))
    ox = (W - (x1 - x0) * s) / 2 - x0 * s
    oy = (H - (y1 - y0) * s) / 2 - y0 * s
    if unit:  # snap the origin to the pixel grid
        ox = round(ox / ss) * ss
        oy = round(oy / ss) * ss
    img = Image.new("L", (W, H), NONE)
    sm = Image.new("L", (W, H), 0)
    d, ds = ImageDraw.Draw(img), ImageDraw.Draw(sm)
    lw = max(1, round(seam * s)) if seam > 0 else 0
    for cube in sc:
        for fid, poly in cube:
            pts = [(ox + px * s, oy + py * s) for px, py in poly]
            d.polygon(pts, fill=fid)
            if lw:
                ds.polygon(pts, fill=0)
        if lw:
            for fid, poly in cube:
                pts = [(ox + px * s, oy + py * s) for px, py in poly]
                ds.line(pts + [pts[0]], fill=255, width=lw, joint="curve")
    m = np.asarray(img)
    seam_m = np.asarray(sm).astype(np.float32) / 255 * (m != NONE)
    return m, seam_m


def colourise(lm, palette, ss, out_size, seam_m=None, seam_a=0.0, hard=False):
    """Colour the face map, darken joints with Ink at seam_a, then
    box-downsample (or, hard=True, take each pixel's majority face so small
    pixel-art marks stay crisp)."""
    oh, ow = out_size[1], out_size[0]
    if hard:
        blocks = lm.reshape(oh, ss, ow, ss).transpose(0, 2, 1, 3).reshape(oh, ow, -1)
        counts = np.stack([(blocks == k).sum(-1) for k in range(8)], -1)
        lm = counts.argmax(-1)
        if seam_m is not None:
            seam_m = seam_m.reshape(oh, ss, ow, ss).mean(axis=(1, 3)) > 0.5
        ss = 1
    h, w = lm.shape
    rgba = np.zeros((h, w, 4), np.float32)
    for fid, col in palette.items():
        rgba[lm == fid] = col
    if seam_m is not None and seam_a > 0:
        k = (np.asarray(seam_m, np.float32) * seam_a)[..., None]
        ink = np.array(INK, np.float32) / 255
        rgba[..., :3] = rgba[..., :3] * (1 - k) + ink * k
    rgba[..., :3] *= rgba[..., 3:4]
    rgba = rgba.reshape(oh, ss, ow, ss, 4).mean(axis=(1, 3))
    a = rgba[..., 3:4]
    rgb = np.where(a > 1e-6, rgba[..., :3] / np.maximum(a, 1e-6), 0)
    out = np.concatenate([rgb, a], -1)
    return Image.fromarray((np.clip(out, 0, 1) * 255 + 0.5).astype(np.uint8), "RGBA")


def f(c, a=1.0):
    return tuple(v / 255 for v in c) + (a,)


COLOUR = {RT: f(FACE["top"]), RL: f(FACE["left"]), RR: f(FACE["right"]),
          TT: f(TEAL["top"]), TL: f(TEAL["left"]), TR: f(TEAL["right"]),
          SEAM: f(INK)}  # (SEAM id unused now; joints use the seam mask)
# Monochrome: one colour (white), faces separated by opacity only, as a
# template image would be. Teal block is solid white so it leads.
MONO = {RT: f((255,) * 3, 0.92), RR: f((255,) * 3, 0.62), RL: f((255,) * 3, 0.36),
        TT: f((255,) * 3, 1.0), TR: f((255,) * 3, 1.0), TL: f((255,) * 3, 1.0),
        SEAM: f((0, 0, 0), 0.0)}


def iso_logo(N, T, w, h, palette=COLOUR, seam=0.0, seam_a=1.0, ss=None):
    ss = ss or max(4, min(16, 4096 // max(w, h)))
    lm, sm = label_map(N, T, w, h, ss, seam)
    return colourise(lm, palette, ss, (w, h), sm, seam_a if seam else 0)


def pixel_logo(N, T, unit, palette=COLOUR, pad=0):
    """2:1 pixel-art isometric C, unit px per voxel half-width, no AA.
    Canvas is exactly 2*N*unit square (+pad)."""
    size = 2 * N * unit + 2 * pad
    ss = 8
    lm, _ = label_map(N, T, size, size, ss, 0, w=PIX, unit=unit)
    return colourise(lm, palette, ss, (size, size), hard=True)


# ---------------------------------------------------------- flat block C
# 3x3 grid: ring of blocks, opening on the right-middle, teal block in the
# centre (it sits in the C's opening, as in the intro).
FLAT_RING = [(0, 0), (1, 0), (2, 0), (0, 1), (0, 2), (1, 2), (2, 2)]
FLAT_TEAL = (1, 1)


def flat_logo(size, cell, gap, ring_col, teal_col, radius=0, inset=0):
    """Pixel-exact flat C on a size x size canvas (grid centred, integer px).
    inset shrinks the centre (teal) block so a one-colour mark still reads as
    a C with a cube in it rather than a 3x3 grid."""
    span = 3 * cell + 2 * gap
    o = (size - span) // 2
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for (cx, cy), col in [(p, ring_col) for p in FLAT_RING] + [(FLAT_TEAL, teal_col)]:
        x = o + cx * (cell + gap)
        y = o + cy * (cell + gap)
        if (cx, cy) == FLAT_TEAL and inset:
            d.rectangle([x + inset, y + inset, x + cell - 1 - inset, y + cell - 1 - inset], fill=col)
            continue
        if radius:
            d.rounded_rectangle([x, y, x + cell - 1, y + cell - 1], radius, fill=col)
        else:
            d.rectangle([x, y, x + cell - 1, y + cell - 1], fill=col)
    return img


def flat_logo_big(size, ring_col, teal_col):
    """Smooth large flat C (supersampled), for previews of option C's mark."""
    ss = 4
    cell = size * ss * 0.26
    gap = size * ss * 0.07
    span = 3 * cell + 2 * gap
    o = (size * ss - span) / 2
    img = Image.new("RGBA", (size * ss, size * ss), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    r = cell * 0.12
    for (cx, cy), col in [(p, ring_col) for p in FLAT_RING] + [(FLAT_TEAL, teal_col)]:
        x = o + cx * (cell + gap)
        y = o + cy * (cell + gap)
        d.rounded_rectangle([x, y, x + cell, y + cell], r, fill=col)
    return img.resize((size, size), Image.BOX)


# -------------------------------------------------------------------- tile
def squircle_mask(size, body, ss=4, n=5.0):
    N = size * ss
    yy, xx = np.mgrid[0:N, 0:N].astype(np.float32) / ss + 0.5 / ss
    c = size / 2
    r = body / 2
    dd = np.abs((xx - c) / r) ** n + np.abs((yy - c) / r) ** n
    m = (dd <= 1).astype(np.float32)
    return m.reshape(size, ss, size, ss).mean(axis=(1, 3))


def tile(size, body):
    """Opaque Ink squircle. Only 'glass' is a 1-px-ish top edge light at 7%."""
    m = squircle_mask(size, body)
    inner = squircle_mask(size, body - 2 * max(1, size / 512))
    # top-weighted rim light, very subtle
    y = np.linspace(0, 1, size)[:, None]
    rim = np.clip(m - inner, 0, 1) * np.clip(1.2 - 1.6 * y, 0, 1) * 0.07
    rgb = np.ones((size, size, 3), np.float32) * (np.array(INK, np.float32) / 255)
    rgb = rgb * (1 - rim[..., None]) + rim[..., None]
    out = np.concatenate([rgb, m[..., None]], -1)
    return Image.fromarray((np.clip(out, 0, 1) * 255 + 0.5).astype(np.uint8), "RGBA")


def on_tile(size, body, mark, mark_frac):
    t = tile(size, body)
    return t, int(round(body * mark_frac))


INTRO_SEAM = (0.06, 0.55)   # intro cube(): stroke width s*0.06, Ink at 0.55


def app_icon(opt, size):
    if size >= 256:
        body = round(size * 824 / 1024)
    else:
        body = size - 2          # small icons fill the canvas (1 px margin)
    t = tile(size, body)
    if opt == "A":
        box = round(body * 0.64)
        m = iso_logo(12, 3, box, box)
    elif opt == "B":
        if size >= 64:
            box = round(body * 0.66)
            m = iso_logo(4, 1, box, box, seam=0.05, seam_a=1.0)
        else:
            m = pixel_logo(4, 1, unit=max(1, round(size * 0.66 / 8)))
    else:  # C
        if size >= 64:
            box = round(body * 0.64)
            m = iso_logo(12, 3, box, box, seam=INTRO_SEAM[0], seam_a=INTRO_SEAM[1])
        else:
            m = flat_logo(size, cell=6, gap=2, ring_col=FACE["top"] + (255,),
                          teal_col=TEAL_FLAT + (255,))
            t.alpha_composite(m)
            return t
    x = (size - m.width) // 2
    y = (size - m.height) // 2
    t.alpha_composite(m, (x, y))
    return t


def menu_mark(opt, h):
    """Monochrome white template mark, h px high, transparent background."""
    if opt == "A":
        w = round(h * 0.866)
        if h <= 16:  # box-AA goes to mush at 16 px; majority sampling stays crisp
            lm, _ = label_map(12, 3, w, h, 16)
            return colourise(lm, MONO, 16, (w, h), hard=True)
        return iso_logo(12, 3, w, h, palette=MONO)
    if opt == "B":
        return pixel_logo(4, 1, unit=h // 8, palette=MONO)
    cell, gap, ins = {16: (4, 2, 1), 24: (6, 3, 1)}.get(h, (round(h / 4), round(h / 8), 1))
    return flat_logo(h, cell, gap, (255, 255, 255, 255), (255, 255, 255, 255), inset=ins)


def font(sz, weight="Regular"):
    ft = ImageFont.truetype(FONT, sz)
    ft.set_variation_by_name(weight)
    return ft


def banner(opt):
    W, H = 1200, 400
    img = Image.new("RGBA", (W, H), INK + (255,))
    lh = 230
    if opt == "A":
        m = iso_logo(12, 3, round(lh * 0.866), lh)
    elif opt == "B":
        m = iso_logo(4, 1, round(lh * 0.866), lh, seam=0.05)
    else:
        m = iso_logo(12, 3, round(lh * 0.866), lh, seam=INTRO_SEAM[0], seam_a=INTRO_SEAM[1])
    f1, f2 = font(118, "SemiBold"), font(34, "Regular")
    word, tag = "Cove", "Windows, made calm."
    d = ImageDraw.Draw(img)
    ww = d.textlength(word, font=f1)
    tw = d.textlength(tag, font=f2)
    gapx = 64
    total = m.width + gapx + max(ww, tw)
    x0 = round((W - total) / 2)
    img.alpha_composite(m, (x0, (H - m.height) // 2))
    tx = x0 + m.width + gapx
    # optical vertical centring of the two lines on the mark
    asc1 = f1.getbbox("Cove")
    asc2 = f2.getbbox("Windows,")
    block = (asc1[3] - asc1[1]) + 26 + (asc2[3] - asc2[1])
    ty = (H - block) / 2 - asc1[1]
    d.text((tx, ty), word, font=f1, fill=PAPER)
    d.text((tx + 4, ty + asc1[3] + 26 - asc2[1] + 2), tag, font=f2, fill=GREY)
    return img


def on_bg(im, col, pad=0):
    bg = Image.new("RGBA", (im.width + 2 * pad, im.height + 2 * pad), col + (255,))
    bg.alpha_composite(im, (pad, pad))
    return bg


# ------------------------------------------------------------------- sheet
def zoom(im, k):
    return im.resize((im.width * k, im.height * k), Image.NEAREST)


def build():
    files = {}
    for opt in "ABC":
        ic1024 = app_icon(opt, 1024)
        ic32 = app_icon(opt, 32)
        mm16 = menu_mark(opt, 16)
        mm24 = menu_mark(opt, 24)
        bn = banner(opt)
        p = lambda n: os.path.join(OUT, f"option-{opt}-{n}.png")
        ic1024.save(p("icon-1024"))
        ic32.save(p("icon-32"))
        mm16.save(p("menubar-16"))
        mm24.save(p("menubar-24"))
        bn.convert("RGB").save(p("banner-1200x400"))
        files[opt] = dict(ic1024=ic1024, ic32=ic32, mm16=mm16, mm24=mm24, bn=bn)
    sheet(files)


def sheet(files):
    W = 2200
    rowh = 420
    top = 120
    H = top + rowh * 3 + 40
    BG = (0x1A, 0x1B, 0x1F)
    s = Image.new("RGBA", (W, H), BG + (255,))
    d = ImageDraw.Draw(s)
    d.text((40, 36), "Cove: voxel logo options", font=font(40, "SemiBold"), fill=PAPER)
    heads = [(130, "App icon 1024 (shown 320)"), (500, "Icon 32 px: actual + 8x"),
             (870, "Menu bar 16 / 24 px: actual + 6x"), (1260, "Banner 1200x400 (shown 50%)")]
    for x, t in heads:
        d.text((x, 92), t, font=font(20), fill=GREY)
    notes = {
        "A": "Intro final frame, 12-voxel grid, seams closed",
        "B": "Coarse 4-block grid, joints; pixel-exact 2:1 iso at small sizes",
        "C": "Intro C with its cube joints; flat 3x3 block C at 32 px + mono",
    }
    MB = (0x2A, 0x2B, 0x30)  # dark menu bar strip
    for i, opt in enumerate("ABC"):
        y = top + i * rowh
        fs = files[opt]
        d.text((40, y + 20), opt, font=font(64, "SemiBold"), fill=PAPER)
        d.text((40, y + rowh - 60), notes[opt], font=font(20), fill=GREY)
        s.alpha_composite(fs["ic1024"].resize((320, 320), Image.LANCZOS), (130, y + 20))
        # 32 px icon: actual size on dark + zoom
        s.alpha_composite(fs["ic32"], (500, y + 30))
        s.alpha_composite(zoom(fs["ic32"], 8), (550, y + 30))
        # menu bar: actual-size strip, then zooms
        strip = Image.new("RGBA", (300, 28), MB + (255,))
        strip.alpha_composite(fs["mm16"], (14, 6))
        strip.alpha_composite(fs["mm24"], (50, 2))
        ImageDraw.Draw(strip).text((90, 6), "Finder   File   Edit", font=font(13), fill=(235, 235, 240))
        s.alpha_composite(strip, (870, y + 30))
        z16 = on_bg(zoom(fs["mm16"], 6), MB, 6)
        z24 = on_bg(zoom(fs["mm24"], 6), MB, 6)
        s.alpha_composite(z16, (870, y + 80))
        s.alpha_composite(z24, (870 + z16.width + 20, y + 80))
        s.alpha_composite(fs["bn"].resize((600, 200), Image.LANCZOS), (1260, y + 30))
        if i:
            d.line([(40, y), (W - 40, y)], fill=(0x2E, 0x2F, 0x35), width=1)
    s.convert("RGB").save(os.path.join(OUT, "logo-voxel-options.png"))


if __name__ == "__main__":
    build()
    print(OUT)
