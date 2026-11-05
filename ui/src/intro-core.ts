// Cove intro, drawing only (no DOM reads). Design: option B "Blocks settle"
// (notes/mockups/intro-blocks/intro-B.html). A faint isometric grid (the
// floor and two back walls of logo B's 4x4x4 volume) draws itself in; sharp
// cubes drop in rows, bottom row first, each landing with one small
// critically damped settle; the teal cube settles last in the opening; the
// grid fades, the mark rises a little and "Cove" + "Windows, made calm."
// appear, on flat Ink. Every later launch gets the ~1 s version: the mark
// alone settles in and fades.
//
// Built for frame time, not convenience: everything that is a path (each
// cube with its faces and seams, each grid line, each letter) is rendered
// ONCE at device pixels into a sprite when the scene is built. A frame is a
// clear of the dirty rectangle plus a few dozen drawImage calls at
// positions from closed-form, time-based curves. No paths, text layout,
// shadows, filters, gradients or allocations happen per frame.

export const INK = "#111215";

const FACE = { top: "#f7dccd", left: "#8c5a64", right: "#c48b82" };
const TEAL = { top: "#a6f2e6", left: "#169487", right: "#2cc9b9" };
type Faces = typeof FACE;
const GRID = "rgba(242,238,233,0.11)"; // hairlines: wordmark paper at low alpha
const GRID_A = 0.11;
const N = 4, T = 1, SEAM = 0.05; // logo B: 4x4x4, bars 1 thick, joints 0.05 of the edge

export interface IntroOpts { full: boolean; calm: boolean; font: string }

/** The quick launch mark: about a second, never longer. */
export const SHORT = 1.0;
const FULL = 4.9;

/** Length of each intro in seconds. */
export function introLength(o: IntroOpts): number {
  return o.full ? (o.calm ? 2.6 : FULL) : o.calm ? 0.8 : SHORT;
}

const clamp = (x: number) => (x < 0 ? 0 : x > 1 ? 1 : x);
export const eout = (t: number) => 1 - (1 - clamp(t)) ** 3;
const ein = (t: number) => clamp(t) ** 2;
/** Critically damped impulse (damping 1.0): one dip and back, no oscillation. Peak 1 at t = 1/w. */
const settle = (t: number, w: number) => (t <= 0 ? 0 : Math.E * w * t * Math.exp(-w * t));

// ---- model: logo B's cubes, in painter's order, with landing times ----

interface Cube { p: [number, number, number]; teal: boolean; row: number; idx: number }

function model(): Cube[] {
  const v: Cube[] = [];
  const near = (a: number, e: number) => (e === 0 ? a < T : a >= N - T);
  const c = (N - 1) / 2;
  for (let x = 0; x < N; x++) for (let y = 0; y < N; y++) for (let z = 0; z < N; z++) {
    const b = [near(x, N) && near(y, 0), near(x, 0) && near(y, N), near(x, N) && near(z, 0),
      near(y, N) && near(z, 0), near(x, 0) && near(z, N), near(y, 0) && near(z, N)];
    if (!b.some(Boolean)) continue;
    if (b[0] && !b[2] && !b[5]) continue;
    if (b[0] && z >= T && z < N - T) continue;
    v.push({ p: [x - c, y - c, z - c], teal: false, row: 0, idx: 0 });
  }
  // Three rows from the ground up (bottom bar, the spine, the top bar);
  // within a row back to front, so a row reads as one sweep.
  const zs = v.map((q) => q.p[2]), lo = Math.min(...zs), hi = Math.max(...zs);
  for (const q of v) q.row = q.p[2] === lo ? 0 : q.p[2] === hi ? 2 : 1;
  for (let r = 0; r < 3; r++) {
    v.filter((q) => q.row === r)
      .sort((a, b) => a.p[2] - b.p[2] || a.p[0] + a.p[1] - (b.p[0] + b.p[1]) || a.p[0] - b.p[0])
      .forEach((q, i) => (q.idx = i));
  }
  v.push({ p: [0, 0, 0], teal: true, row: 3, idx: 0 });
  return v.sort((a, b) => a.p[0] + a.p[1] + a.p[2] - (b.p[0] + b.p[1] + b.p[2]) || a.p[2] - b.p[2]);
}

const CUBES = model();

/** Mark bounds in unit space (a cube anchored at (sx,sy) spans sx±0.866, sy-1.5 .. sy+0.5). */
const BBOX = (() => {
  let x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
  for (const c of CUBES) {
    const [x, y, z] = c.p, sx = (x - y) * 0.866, sy = (x + y) * 0.5 - z;
    x0 = Math.min(x0, sx - 0.866); x1 = Math.max(x1, sx + 0.866); y0 = Math.min(y0, sy - 1.5); y1 = Math.max(y1, sy + 0.5);
  }
  return { x0, y0, x1, y1 };
})();

/** Timing of each intro. Full = the mockup's numbers exactly. */
const TIMING = {
  full: { start: [0.85, 1.55, 1.95], step: [0.065, 0.12, 0.065], teal: 2.75, fall: 0.3, drop: 1.5, w: 22 },
  short: { start: [0.08, 0.26, 0.38], step: [0.028, 0.05, 0.028], teal: 0.6, fall: 0.2, drop: 0.9, w: 26 },
};

// ---- sprites ----

type Canvas = HTMLCanvasElement | OffscreenCanvas;
type Ctx2D = CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;
export type MakeCanvas = (w: number, h: number) => Canvas;

const defaultMake: MakeCanvas = (w, h) => {
  if (typeof OffscreenCanvas !== "undefined") return new OffscreenCanvas(w, h);
  const c = document.createElement("canvas"); c.width = w; c.height = h; return c;
};

/** A pre-rendered image and where its top-left corner sits (device px, at rest). */
interface Sprite { img: Canvas | ImageBitmap; x: number; y: number; w: number; h: number }

function sprite(make: MakeCanvas, x0: number, y0: number, x1: number, y1: number, paint: (g: Ctx2D, ox: number, oy: number) => void): Sprite {
  const x = Math.floor(x0), y = Math.floor(y0);
  const w = Math.max(1, Math.ceil(x1) - x), h = Math.max(1, Math.ceil(y1) - y);
  const img = make(w, h);
  const g = img.getContext("2d") as Ctx2D;
  paint(g, x, y); // the painter draws in canvas coordinates minus (x, y): sub-pixel offsets are baked in
  // An immutable ImageBitmap uploads to the GPU once and stays there.
  const bmp = "transferToImageBitmap" in img ? img.transferToImageBitmap() : null;
  return { img: bmp || img, x, y, w, h };
}

/** One cube as in the mockup: three flat faces, then Ink joints over every face edge. */
function paintCube(g: Ctx2D, sx: number, sy: number, S: number, col: Faces, lw: number) {
  const w = 0.866 * S, hh = 0.5 * S, top = sy - S;
  const fs: [string, number[]][] = [
    [col.top, [sx, top - hh, sx + w, top, sx, top + hh, sx - w, top]],
    [col.left, [sx - w, top, sx, top + hh, sx, sy + hh, sx - w, sy]],
    [col.right, [sx + w, top, sx, top + hh, sx, sy + hh, sx + w, sy]],
  ];
  const path = (q: number[]) => { g.beginPath(); g.moveTo(q[0], q[1]); for (let i = 2; i < 8; i += 2) g.lineTo(q[i], q[i + 1]); g.closePath(); };
  for (const [c, q] of fs) { path(q); g.fillStyle = c; g.fill(); }
  g.strokeStyle = INK; g.lineWidth = lw; g.lineJoin = "round";
  for (const [, q] of fs) { path(q); g.stroke(); }
}

interface CubeDraw { s: Sprite; land: number }
interface LineDraw { s: Sprite; ax: number; ay: number; bx: number; by: number; pad: number; delay: number }

/**
 * The whole intro, built once for one canvas size and then drawn at any
 * time t. Coordinates are device pixels; the context's transform must be
 * the identity.
 */
export class IntroScene {
  readonly length: number;
  private o: IntroOpts;
  private W = 0; private H = 0;
  private cubes: CubeDraw[] = [];
  private lines: LineDraw[] = [];
  private gridAll: Sprite | null = null;
  private mark: Sprite | null = null;
  private letters: { s: Sprite; delay: number }[] = [];
  private tag: Sprite | null = null;
  private S = 0; private k = 1; private liftMax = 0;
  private dirty = { x: 0, y: 0, w: 0, h: 0 };
  private cleared = false;
  private tm = TIMING.full;

  constructor(o: IntroOpts) { this.o = o; this.length = introLength(o); }

  /** Render every sprite for a W x H device-pixel canvas (dpr = device px per CSS px). */
  build(W: number, H: number, dpr: number, make: MakeCanvas = defaultMake): void {
    const o = this.o;
    this.W = W; this.H = H; this.cleared = false;
    this.tm = o.full ? TIMING.full : TIMING.short;
    const k = (this.k = H / 1080);
    const S = (this.S = (o.full ? 64 : 44) * k);
    const lw = SEAM * S;
    const markH = (BBOX.y1 - BBOX.y0) * S;
    const F = 60 * k, TF = 25 * k, gap = 0.62 * S;
    const groupH = markH + gap + F * 0.73 + 22 * k + TF;
    this.liftMax = o.full ? (groupH - markH) / 2 : 0;
    const mx = W / 2 - ((BBOX.x0 + BBOX.x1) / 2) * S;
    const my = H / 2 - ((BBOX.y0 + BBOX.y1) / 2) * S;
    const L = (X: number, Y: number, Z: number): [number, number] => [mx + (X - Y) * 0.866 * S, my + ((X + Y) * 0.5 - Z) * S];

    // Cubes: one sprite each, with its own sub-pixel position baked in.
    const tm = this.tm;
    const half = 0.866 * S + lw, pad = 2;
    this.cubes = CUBES.map((c) => {
      const [x, y, z] = c.p;
      const [ax, a0] = L(x, y, z - 0.5);
      const ay = a0 - 0.5 * S; // the cube's own anchor (bottom of its top face's centre line)
      const s = sprite(make, ax - half - pad, ay - S - 0.5 * S - lw - pad, ax + half + pad, ay + 0.5 * S + lw + pad,
        (g, ox, oy) => paintCube(g, ax - ox, ay - oy, S, c.teal ? TEAL : FACE, lw));
      const land = c.teal ? tm.teal : tm.start[c.row] + c.idx * tm.step[c.row];
      return { s, land };
    });
    // The finished mark as one layer (identical pixels: the same cubes in the same order).
    let mx0 = 1e9, my0 = 1e9, mx1 = -1e9, my1 = -1e9;
    for (const { s } of this.cubes) { mx0 = Math.min(mx0, s.x); my0 = Math.min(my0, s.y); mx1 = Math.max(mx1, s.x + s.w); my1 = Math.max(my1, s.y + s.h); }
    this.mark = sprite(make, mx0, my0, mx1, my1, (g, ox, oy) => { for (const { s } of this.cubes) g.drawImage(s.img as CanvasImageSource, s.x - ox, s.y - oy); });

    // Grid (full only): floor z = -1.5 and the far walls x = -2, y = -2.
    this.lines = []; this.gridAll = null;
    if (o.full) {
      const glw = Math.max(dpr, 1.25 * k), gp = glw / 2 + 2;
      const seg: [number, number, number][][] = [];
      const z0 = -1.5, z1 = 2.5;
      for (let i = -2; i <= 2; i++) {
        seg.push([[i, -2, z0], [i, 2, z0]], [[-2, i, z0], [2, i, z0]]);
        seg.push([[-2, i, z0], [-2, i, z1]], [[i, -2, z0], [i, -2, z1]]);
      }
      for (let z = z0; z <= z1 + 1e-6; z++) seg.push([[-2, -2, z], [-2, 2, z]], [[-2, -2, z], [2, -2, z]]);
      const stroke = (g: Ctx2D, ax: number, ay: number, bx: number, by: number) => {
        g.strokeStyle = GRID; g.lineWidth = glw; g.lineCap = "butt";
        g.beginPath(); g.moveTo(ax, ay); g.lineTo(bx, by); g.stroke();
      };
      let gx0 = 1e9, gy0 = 1e9, gx1 = -1e9, gy1 = -1e9;
      this.lines = seg.map(([a, b], i) => {
        const [ax, ay] = L(a[0], a[1], a[2]), [bx, by] = L(b[0], b[1], b[2]);
        const x0 = Math.min(ax, bx) - gp, y0 = Math.min(ay, by) - gp, x1 = Math.max(ax, bx) + gp, y1 = Math.max(ay, by) + gp;
        gx0 = Math.min(gx0, x0); gy0 = Math.min(gy0, y0); gx1 = Math.max(gx1, x1); gy1 = Math.max(gy1, y1);
        const s = sprite(make, x0, y0, x1, y1, (g, ox, oy) => stroke(g, ax - ox, ay - oy, bx - ox, by - oy));
        return { s, ax, ay, bx, by, pad: gp, delay: 0.1 + i * 0.012 };
      });
      const lines = this.lines;
      this.gridAll = sprite(make, gx0, gy0, gx1, gy1, (g, ox, oy) => { for (const l of lines) stroke(g, l.ax - ox, l.ay - oy, l.bx - ox, l.by - oy); });
    }

    // Wordmark (full only), at its final (lifted) place; drawn with offsets.
    this.letters = []; this.tag = null;
    if (o.full) {
      const meas = make(4, 4).getContext("2d") as Ctx2D;
      const top = my - this.liftMax + BBOX.y1 * S + gap;
      const base = top + F * 0.73;
      const word = "Cove";
      meas.font = `600 ${F}px ${o.font}`;
      let wx = W / 2 - meas.measureText(word).width / 2;
      for (let i = 0; i < word.length; i++) {
        const ch = word[i], cw = meas.measureText(ch).width, x = wx;
        const s = sprite(make, x - 0.15 * F, base - 1.05 * F, x + cw + 0.15 * F, base + 0.3 * F, (g, ox, oy) => {
          g.font = `600 ${F}px ${o.font}`; g.textBaseline = "alphabetic"; g.textAlign = "left"; g.fillStyle = "#f2eee9";
          g.fillText(ch, x - ox, base - oy);
        });
        this.letters.push({ s, delay: 3.6 + i * 0.07 });
        wx += cw;
      }
      meas.font = `400 ${TF}px ${o.font}`;
      const tw = meas.measureText("Windows, made calm.").width, ty = base + 22 * k + TF;
      this.tag = sprite(make, W / 2 - tw / 2 - TF, ty - 1.1 * TF, W / 2 + tw / 2 + TF, ty + 0.4 * TF, (g, ox, oy) => {
        g.font = `400 ${TF}px ${o.font}`; g.textBaseline = "alphabetic"; g.textAlign = "center"; g.fillStyle = "#8b8b93";
        g.fillText("Windows, made calm.", W / 2 - ox, ty - oy);
      });
    }

    // The only area that ever changes: everything, at both ends of the lift,
    // with the drop above and the settle below. The rest stays Ink.
    let x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
    const add = (s: Sprite, up: number, down: number) => {
      x0 = Math.min(x0, s.x); x1 = Math.max(x1, s.x + s.w);
      y0 = Math.min(y0, s.y - up); y1 = Math.max(y1, s.y + s.h + down);
    };
    const DROP = tm.drop * S;
    for (const c of this.cubes) add(c.s, DROP + this.liftMax, 0.05 * S);
    if (this.gridAll) add(this.gridAll, this.liftMax, 0);
    for (const l of this.letters) add(l.s, 0, 12 * k + this.liftMax);
    if (this.tag) add(this.tag, 0, this.liftMax);
    x0 = Math.max(0, Math.floor(x0) - 2); y0 = Math.max(0, Math.floor(y0) - 2);
    x1 = Math.min(W, Math.ceil(x1) + 2); y1 = Math.min(H, Math.ceil(y1) + 2);
    this.dirty = { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
  }

  /**
   * Warm-up: draw every sprite once and paint Ink over them in the same
   * frame, so each one is uploaded to the GPU before the clock starts and
   * the first drops never pay for it. Nothing shows.
   */
  warm(g: CanvasRenderingContext2D): void {
    g.globalAlpha = 1;
    const all: Sprite[] = [];
    for (const c of this.cubes) all.push(c.s);
    for (const l of this.lines) all.push(l.s);
    for (const l of this.letters) all.push(l.s);
    if (this.mark) all.push(this.mark);
    if (this.gridAll) all.push(this.gridAll);
    if (this.tag) all.push(this.tag);
    for (const s of all) { g.globalAlpha = 0.5; g.drawImage(s.img as CanvasImageSource, s.x, s.y); }
    g.globalAlpha = 1;
    g.fillStyle = INK;
    g.fillRect(0, 0, this.W, this.H);
  }

  /** Draw the intro at t seconds. Allocation-free. */
  draw(g: CanvasRenderingContext2D, t: number): void {
    const o = this.o, d = this.dirty;
    g.globalAlpha = 1;
    g.fillStyle = INK;
    if (!this.cleared) { g.fillRect(0, 0, this.W, this.H); this.cleared = true; }
    else g.fillRect(d.x, d.y, d.w, d.h);

    if (o.calm) return this.drawCalm(g, t);

    const S = this.S, tm = this.tm;
    // The group's rise (full only): sub-pixel while moving, whole pixels at rest.
    let lift = this.liftMax * eout((t - 3.35) / 0.6);
    if (lift === 0 || lift === this.liftMax) lift = Math.round(lift);
    const oy = -lift;

    // 1. Grid: lines draw in from their far end, then fade together.
    if (this.gridAll) {
      const ga = GRID_A * (1 - eout((t - 3.0) / 0.55));
      if (ga > 0.002) {
        const lines = this.lines, last = lines[lines.length - 1];
        if (t >= last.delay + 0.55) {
          g.globalAlpha = ga / GRID_A;
          const s = this.gridAll;
          g.drawImage(s.img as CanvasImageSource, s.x, s.y + oy);
        } else {
          g.globalAlpha = ga / GRID_A;
          for (let i = 0; i < lines.length; i++) {
            const l = lines[i];
            const f = eout((t - l.delay) / 0.55);
            if (f <= 0) continue;
            // A straight line grows from A: show its sprite cropped to the box from A to the tip.
            const px = l.ax + (l.bx - l.ax) * f, py = l.ay + (l.by - l.ay) * f;
            const rx0 = Math.max(l.s.x, Math.min(l.ax, px) - l.pad), rx1 = Math.min(l.s.x + l.s.w, Math.max(l.ax, px) + l.pad);
            const ry0 = Math.max(l.s.y, Math.min(l.ay, py) - l.pad), ry1 = Math.min(l.s.y + l.s.h, Math.max(l.ay, py) + l.pad);
            const sw = rx1 - rx0, sh = ry1 - ry0;
            if (sw <= 0 || sh <= 0) continue;
            g.drawImage(l.s.img as CanvasImageSource, rx0 - l.s.x, ry0 - l.s.y, sw, sh, rx0, ry0 + oy, sw, sh);
          }
        }
      }
    }

    // 2. Cubes drop in rows and settle (no bounce). Once all are still, the
    //    finished mark is one layer.
    const lastLand = tm.teal;
    const short = !o.full;
    if (t >= lastLand + 0.35 && this.mark) {
      g.globalAlpha = 1;
      g.drawImage(this.mark.img as CanvasImageSource, this.mark.x, this.mark.y + oy);
    } else {
      const DROP = tm.drop * S, DIP = 0.045 * S, fall = tm.fall;
      const fade = short ? 0.1 : 0.14;
      for (let i = 0; i < this.cubes.length; i++) {
        const c = this.cubes[i];
        const t0 = c.land - fall;
        if (t < t0) continue;
        let dy = t < c.land ? -DROP * (1 - ein((t - t0) / fall)) : DIP * settle(t - c.land, tm.w);
        if (dy > -0.5 && dy < 0.5) dy = 0; // at rest on whole device pixels: edges stay sharp
        g.globalAlpha = clamp((t - t0) / fade);
        g.drawImage(c.s.img as CanvasImageSource, c.s.x, c.s.y + dy + oy);
      }
    }

    // 3. Name and tagline (full).
    if (o.full) {
      const k = this.k;
      const back = this.liftMax - lift; // sprites sit at the lifted place
      for (let i = 0; i < this.letters.length; i++) {
        const l = this.letters[i];
        const lt = eout((t - l.delay) / 0.5);
        if (lt <= 0) continue;
        g.globalAlpha = lt;
        let y = l.s.y + back + 12 * k * (1 - lt);
        if (lt >= 1 && back === 0) y = l.s.y;
        g.drawImage(l.s.img as CanvasImageSource, l.s.x, y);
      }
      const ta = eout((t - 4.05) / 0.55);
      if (ta > 0 && this.tag) {
        g.globalAlpha = ta;
        g.drawImage(this.tag.img as CanvasImageSource, this.tag.x, this.tag.y + back);
      }
    } else {
      // Short: the finished mark fades back to Ink as one piece.
      const out = eout((t - 0.8) / 0.2);
      if (out > 0) { g.globalAlpha = out; g.fillStyle = INK; g.fillRect(d.x, d.y, d.w, d.h); }
    }
    g.globalAlpha = 1;
  }

  /** Reduce motion: the finished frame fades in (and, for the short one, out). */
  private drawCalm(g: CanvasRenderingContext2D, t: number): void {
    const a = this.o.full ? eout(t / 0.6) : eout(t / 0.3) * (1 - eout((t - 0.55) / 0.25));
    if (a <= 0) return;
    g.globalAlpha = a;
    const lift = Math.round(this.liftMax);
    if (this.mark) g.drawImage(this.mark.img as CanvasImageSource, this.mark.x, this.mark.y - lift);
    for (const l of this.letters) g.drawImage(l.s.img as CanvasImageSource, l.s.x, l.s.y);
    if (this.tag) g.drawImage(this.tag.img as CanvasImageSource, this.tag.x, this.tag.y);
    g.globalAlpha = 1;
  }
}

/**
 * One-off frame for previews and harnesses: draws the intro at time t into a
 * context whose transform maps CSS px to device px (scale dpr). Builds and
 * caches a scene per size.
 */
let cached: { key: string; scene: IntroScene } | null = null;
export function drawIntro(g: CanvasRenderingContext2D, t: number, W: number, H: number, o: IntroOpts): void {
  const dpr = g.getTransform().a || 1;
  const Wd = Math.round(W * dpr), Hd = Math.round(H * dpr);
  const key = `${Wd}x${Hd}@${dpr}:${o.full}:${o.calm}:${o.font}`;
  if (!cached || cached.key !== key) { const scene = new IntroScene(o); scene.build(Wd, Hd, dpr); cached = { key, scene }; }
  g.save();
  g.setTransform(1, 0, 0, 1, 0, 0);
  cached.scene.draw(g, t);
  g.restore();
}
