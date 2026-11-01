"""Cove intro previews: first-run (voxel C + name) and every-launch (quick mark)."""
import math, random, subprocess, os, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter
sys.path.insert(0, "/workspace/work/Cove/brand")
os.chdir("/workspace/work/Cove/brand")
from icon_options import icon, OPTS
DEEP = dict(OPTS)["2  Deep tide"]
SS = 2; W, H = 960, 600; FPS = 30
TEAL = np.array([31, 181, 168])
fb = lambda n: ImageFont.truetype("DejaVuSans-Bold.ttf", n)
fr = lambda n: ImageFont.truetype("DejaVuSans.ttf", n)
def clamp(x): return max(0.0, min(1.0, x))
def eout(t): t = clamp(t); return 1 - (1 - t) ** 3
def eback(t, s=1.4): t = clamp(t) - 1; return 1 + (s + 1) * t ** 3 + s * t ** 2
def einout(t): t = clamp(t); return 4*t**3 if t < .5 else 1 - (-2*t + 2) ** 3 / 2

def background(t, glow_amt):
    """Deep plum wash with two slow teal/coral light pools."""
    y, x = np.mgrid[0:H, 0:W].astype(np.float32)
    v = y / H
    base = np.stack([58 - 31*v, 31 - 13*v, 61 - 20*v], -1)
    def pool(cx, cy, r, col, a):
        d = ((x - cx)**2 + (y - cy)**2) / r**2
        return np.exp(-d)[..., None] * np.array(col) * a
    base += pool(W*(.30 + .04*math.sin(t*.7)), H*(.35 + .03*math.cos(t*.5)), 380, (20, 120, 110), .55*glow_amt)
    base += pool(W*(.75 - .04*math.sin(t*.6)), H*(.75), 420, (120, 50, 60), .45*glow_amt)
    return Image.fromarray(np.clip(base, 0, 255).astype(np.uint8)).resize((W*SS, H*SS), Image.BILINEAR).convert("RGBA")

def mark(px): return icon(px*2, DEEP | {"tile": None, "scale": .96, "glow": None}).resize((px, px), Image.LANCZOS)

def glow(img, cx, cy, r, a):
    g = Image.new("RGBA", img.size, (0, 0, 0, 0))
    ImageDraw.Draw(g).ellipse((cx-r, cy-r, cx+r, cy+r), fill=(31, 181, 168, int(255*a)))
    img.alpha_composite(g.filter(ImageFilter.GaussianBlur(r*.45)))

def voxels(px, cell):
    m = np.asarray(mark(px)).astype(float)
    n = px // cell; out = []
    rnd = random.Random(7)
    for gy in range(n):
        for gx in range(n):
            blk = m[gy*cell:(gy+1)*cell, gx*cell:(gx+1)*cell]
            if blk[..., 3].mean() < 140: continue
            rgb = blk[..., :3][blk[..., 3] > 128].mean(0)
            cube = rgb[1] > rgb[0] + 30
            x, y = (gx+.5)*cell - px/2, (gy+.5)*cell - px/2
            a = (math.degrees(math.atan2(-y, x)) - 25) % 360
            order = .82 + .18*rnd.random() if cube else (a/310)*.78 + .06*rnd.random()
            ang = math.atan2(y, x) + rnd.uniform(-.5, .5)
            dist = rnd.uniform(90, 200)
            out.append(dict(x=x, y=y, rgb=tuple(int(c) for c in rgb), order=min(order, 1), dx=math.cos(ang)*dist, dy=math.sin(ang)*dist - rnd.uniform(30, 90), cube=cube))
    return out

def draw_vox(img, vs, cx, cy, cell, p, span=.35):
    """p in [0,1] overall build progress; each voxel lands over `span`."""
    d = ImageDraw.Draw(img)
    for v in sorted(vs, key=lambda v: v["order"]):
        lt = (p - v["order"]*(1-span)) / span
        if lt <= 0: continue
        k = eout(lt); s = eback(lt, 1.2)
        x = cx + (v["x"] + v["dx"]*(1-k))*SS; y = cy + (v["y"] + v["dy"]*(1-k))*SS
        h = cell*SS*.5*min(s, 1.15)*.94
        if h < .5: continue
        r, g, b = v["rgb"]
        d.rounded_rectangle((x-h, y-h, x+h, y+h), radius=max(1, h*.22), fill=(r, g, b, 255))
        d.line((x-h+2, y-h+1.5, x+h-2, y-h+1.5), fill=(min(255, r+30), min(255, g+30), min(255, b+30), 255), width=max(1, SS))

def first_run(path):
    N = int(4.8*FPS); px = 280; cell = 10
    vs = voxels(px, cell); crisp = mark(px*SS)
    frames = []
    for f in range(N):
        t = f/FPS
        img = background(t, eout(t/1.0))
        rise = eout((t-2.75)/.7)               # mark lifts to make room for the name
        cx, cy = W*SS/2, (H/2 - 80*rise)*SS
        glow(img, cx, cy, 120*SS, .18 + .35*math.exp(-((t-2.45)/.3)**2))
        build = clamp((t-.35)/2.0)
        swap = eout((t-2.2)/.35)               # voxels melt into the crisp mark
        if swap < 1:
            layer = Image.new("RGBA", img.size, (0, 0, 0, 0)); draw_vox(layer, vs, cx, cy, cell, build)
            if swap > 0: layer.putalpha(layer.getchannel("A").point(lambda a: int(a*(1-swap))))
            img.alpha_composite(layer)
        if swap > 0:
            m = crisp.copy(); m.putalpha(m.getchannel("A").point(lambda a: int(a*swap)))
            img.alpha_composite(m, (int(cx - m.width/2), int(cy - m.height/2)))
        d = ImageDraw.Draw(img)
        word = "Cove"; font = fb(58*SS)
        wx = W*SS/2 - d.textlength(word, font=font)/2
        for i, ch in enumerate(word):
            lt = eout((t - 2.95 - i*.08)/.45)
            if lt <= 0: continue
            col = (251, 225, 210, int(255*lt))
            layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
            ImageDraw.Draw(layer).text((wx, (H/2 + 150 + 18*(1-lt))*SS), ch, font=font, fill=col, anchor="ls")
            img.alpha_composite(layer); wx += d.textlength(ch, font=font)
        ta = eout((t-3.55)/.5)
        if ta > 0:
            layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
            ImageDraw.Draw(layer).text((W*SS/2, (H/2 + 188)*SS), "Windows, made calm.", font=fr(19*SS), fill=(196, 142, 134, int(255*ta)), anchor="mm")
            img.alpha_composite(layer)
        frames.append(img.convert("RGB").resize((W, H), Image.LANCZOS))
    save(frames, path)

def every_launch(path):
    N = int(1.6*FPS); px = 160; crisp = mark(px*SS)
    bgc = Image.new("RGBA", (W*SS, H*SS), (36, 22, 44, 255))
    frames = []
    for f in range(N):
        t = f/FPS
        img = bgc.copy(); cx, cy = W*SS/2, H*SS/2
        a = eout(t/.35) * (1 - eout((t-1.05)/.3))   # in, short hold, out
        s = .9 + .1*eback(t/.45, 1.1)
        glow(img, cx, cy, 70*SS, .4*a*math.exp(-((t-.45)/.25)**2) + .12*a)
        m = crisp.resize((int(crisp.width*s), int(crisp.height*s)), Image.LANCZOS)
        m.putalpha(m.getchannel("A").point(lambda v: int(v*a)))
        img.alpha_composite(m, (int(cx - m.width/2), int(cy - m.height/2)))
        frames.append(img.convert("RGB").resize((W, H), Image.LANCZOS))
    frames += [frames[-1]]*int(.4*FPS)
    save(frames, path)

def save(frames, path):
    tmp = path + "_f"; os.makedirs(tmp, exist_ok=True)
    for i, fr_ in enumerate(frames): fr_.save(f"{tmp}/{i:04d}.png")
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-framerate", str(FPS), "-i", f"{tmp}/%04d.png", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18", path], check=True)

if __name__ == "__main__":
    which = sys.argv[1]
    (first_run if which == "first" else every_launch)(f"/workspace/work/intro/{which}.mp4")
    print("ok", which)
