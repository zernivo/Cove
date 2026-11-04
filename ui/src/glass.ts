// Cove Glass: the wallpaper plate and edge refraction for the dock and the
// menu bar (docs/design/GLASS_AND_APPLE_GUIDE.md §4).
//
// A transparent WebView2 window can't see the desktop behind it, so CSS
// backdrop-filter blurs nothing there. Instead the page draws a slice of
// the real wallpaper, pre-frosted once per wallpaper change on a small
// canvas (bilinear upscaling smooths it for free), lined up with where the
// glass sits on the screen. An SVG feDisplacementMap bends the plate in a
// thin edge band only; the centre is exact identity, so nothing wobbles.

const inTauri = "__TAURI_INTERNALS__" in window;

/** URL of the current desktop picture (served by the app's coveicon scheme). */
export function wallpaperUrl(): string {
  if (!inTauri) return new URLSearchParams(location.search).get("wall") ?? "/wallpapers/tidepool-dark.jpg";
  const base = navigator.userAgent.includes("Windows") ? "http://coveicon.localhost" : "coveicon://localhost";
  return `${base}/wallpaper?t=${Date.now()}`;
}

export interface Plate {
  /** Frosted wallpaper, monitor-shaped, as an image URL. */
  url: string;
  /** Brightest (95th percentile) relative luminance in a band of the plate. */
  lum(top: number, height: number): number;
}

const FROST_W = 480; // canvas width; ~4x upscaled on a 1080p screen

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((ok, fail) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.decoding = "async";
    img.onload = () => ok(img);
    img.onerror = () => fail(new Error("wallpaper"));
    img.src = src;
  });
}

/** Pre-frost the wallpaper for a monitor of `mw` x `mh` CSS px ("Fill" fit). */
export async function makePlate(src: string, mw: number, mh: number, blurDip = 30, saturate = 1.6): Promise<Plate> {
  const img = await loadImage(src);
  const W = FROST_W, H = Math.max(1, Math.round((FROST_W * mh) / Math.max(1, mw)));
  const k = W / mw;
  const r = Math.max(2, Math.round(blurDip * k));
  const p = r * 2;
  // 1. Cover-fit into the middle of a padded canvas, then clamp the edges
  //    outward so the blur doesn't fade in from transparent at the screen edge.
  const t = Object.assign(document.createElement("canvas"), { width: W + 2 * p, height: H + 2 * p });
  const g = t.getContext("2d")!;
  const s = Math.max(W / img.naturalWidth, H / img.naturalHeight);
  const dw = img.naturalWidth * s, dh = img.naturalHeight * s;
  g.save();
  g.beginPath(); g.rect(p, p, W, H); g.clip();
  g.drawImage(img, p + (W - dw) / 2, p + (H - dh) / 2, dw, dh);
  g.restore();
  g.drawImage(t, p, p, W, 1, p, 0, W, p);
  g.drawImage(t, p, p + H - 1, W, 1, p, p + H, W, p);
  g.drawImage(t, p, 0, 1, H + 2 * p, 0, 0, p, H + 2 * p);
  g.drawImage(t, p + W - 1, 0, 1, H + 2 * p, p + W, 0, p, H + 2 * p);
  // 2. Frost: blur + saturate once.
  const c = Object.assign(document.createElement("canvas"), { width: W, height: H });
  const ctx = c.getContext("2d", { willReadFrequently: true })!;
  ctx.filter = `blur(${r}px) saturate(${saturate})`;
  ctx.drawImage(t, -p, -p);
  ctx.filter = "none";
  const url = c.toDataURL("image/jpeg", 0.9); // throws if the canvas is tainted
  const lin = (v: number) => { v /= 255; return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
  return {
    url,
    lum(top: number, height: number): number {
      const y0 = Math.max(0, Math.floor(top * k)), h = Math.max(1, Math.min(H - y0, Math.ceil(height * k)));
      const d = ctx.getImageData(0, y0, W, h).data;
      const ls: number[] = [];
      for (let i = 0; i < d.length; i += 16) ls.push(0.2126 * lin(d[i]) + 0.7152 * lin(d[i + 1]) + 0.0722 * lin(d[i + 2]));
      ls.sort((a, b) => a - b);
      return ls[Math.floor(ls.length * 0.95)] ?? 0;
    },
  };
}

/** Tint alpha so white text keeps 4.5:1 over the brightest part of the plate
 *  (dark mode), or dark text over the darkest (light mode), never below the
 *  guide's floor for text on glass. */
export function tintFor(lum: number, dark: boolean, floor: number, cap = 0.8): number {
  if (dark) {
    const ink = 0.0062, target = 0.183; // Ink #111215; white text 4.5:1 needs L <= 0.183
    const a = lum <= target ? 0 : (lum - target) / Math.max(1e-3, lum - ink);
    return Math.min(cap, Math.max(floor, a));
  }
  const paper = 0.9, target = 0.175; // dark #111215 text needs L >= 0.175
  const a = lum >= target ? 0 : (target - lum) / Math.max(1e-3, paper - lum);
  return Math.min(cap, Math.max(floor, a));
}

// ---- Refraction map (kube.io convex squircle bezel, Snell n = 1.5) ----
const NS = "http://www.w3.org/2000/svg";
const cache = new Map<string, { id: string; node: Element }>();
let defs: SVGDefsElement | null = null;
function bank(): SVGDefsElement {
  if (defs?.isConnected) return defs;
  const s = document.createElementNS(NS, "svg");
  s.setAttribute("width", "0"); s.setAttribute("height", "0"); s.setAttribute("aria-hidden", "true");
  s.style.position = "absolute";
  defs = s.appendChild(document.createElementNS(NS, "defs")) as SVGDefsElement;
  document.body.appendChild(s);
  return defs;
}

function profile(bezel: number, thick: number, ior: number, n = 128): Float32Array {
  const f = (x: number) => Math.pow(1 - Math.pow(1 - x, 4), 0.25);
  return Float32Array.from({ length: n }, (_, i) => {
    const x = Math.min(Math.max(i / (n - 1), 5e-4), 1 - 5e-4), e = 1e-3;
    const slope = (thick * (f(Math.min(x + e, 1)) - f(Math.max(x - e, 0)))) / (2 * e * bezel);
    const t1 = Math.atan(slope), t2 = Math.asin(Math.sin(t1) / ior);
    return thick * f(x) * Math.tan(t1 - t2);
  });
}

function buildMap(w: number, h: number, r: number, bezel: number, thickness: number, ior: number): { url: string; scale: number } {
  const mags = profile(bezel, thickness, ior);
  let max = 1e-3;
  for (const m of mags) max = Math.max(max, m);
  const c = Object.assign(document.createElement("canvas"), { width: w, height: h });
  const ctx = c.getContext("2d")!, img = ctx.createImageData(w, h), hw = w / 2, hh = h / 2;
  for (let py = 0; py < h; py++) for (let px = 0; px < w; px++) {
    const x = px + 0.5 - hw, y = py + 0.5 - hh, qx = Math.abs(x) - (hw - r), qy = Math.abs(y) - (hh - r);
    const ox = Math.max(qx, 0), oy = Math.max(qy, 0);
    const d = -(Math.hypot(ox, oy) + Math.min(Math.max(qx, qy), 0) - r);
    let dx = 0, dy = 0;
    if (d > 0 && d < bezel) {
      let nx: number, ny: number;
      if (qx > 0 && qy > 0) { const l = Math.hypot(ox, oy) || 1; nx = ox / l; ny = oy / l; }
      else if (qx > qy) { nx = 1; ny = 0; } else { nx = 0; ny = 1; }
      nx *= Math.sign(x) || 1; ny *= Math.sign(y) || 1;
      const m = mags[Math.floor((d / bezel) * (mags.length - 1))] / max;
      dx = -nx * m; dy = -ny * m;
    }
    const i = (py * w + px) * 4;
    img.data[i] = Math.round(128 + dx * 127); img.data[i + 1] = Math.round(128 + dy * 127);
    img.data[i + 2] = 128; img.data[i + 3] = 255;
  }
  ctx.putImageData(img, 0, 0);
  return { url: c.toDataURL(), scale: 2 * max };
}

export interface RefractOpts { bezel?: number; thickness?: number; ior?: number; strength?: number }

/** id of a cached SVG filter that refracts a w x h rounded rect's edge band. */
export function refractionFilter(w: number, h: number, r: number, o: RefractOpts = {}): string {
  w = Math.round(w); h = Math.round(h); r = Math.min(r, w / 2, h / 2);
  const bezel = Math.min(o.bezel ?? 16, w / 3, h / 3), thickness = o.thickness ?? 32, ior = o.ior ?? 1.5, strength = o.strength ?? 0.7;
  const key = [w, h, r, bezel, thickness, ior, strength].join("|");
  const hit = cache.get(key);
  if (hit?.node.isConnected) return hit.id;
  const { url, scale } = buildMap(w, h, r, bezel, thickness, ior);
  const id = `cg-${cache.size}-${w}x${h}`;
  const f = document.createElementNS(NS, "filter");
  for (const [k, v] of Object.entries({ id, x: "0", y: "0", width: String(w), height: String(h), filterUnits: "userSpaceOnUse", primitiveUnits: "userSpaceOnUse", "color-interpolation-filters": "sRGB" })) f.setAttribute(k, v);
  f.innerHTML = `<feImage href="${url}" x="0" y="0" width="${w}" height="${h}" preserveAspectRatio="none" result="map"/><feDisplacementMap in="SourceGraphic" in2="map" scale="${(scale * strength).toFixed(2)}" xChannelSelector="R" yChannelSelector="G"/>`;
  bank().appendChild(f);
  cache.set(key, { id, node: f });
  return id;
}

export const reducedTransparency = matchMedia("(prefers-reduced-transparency: reduce)");
export const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
